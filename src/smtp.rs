//! # SMTP transport module.

pub mod send;

use std::time::{Duration, SystemTime};

use anyhow::{bail, format_err, Context as _, Error, Result};
use async_smtp::response::{Category, Code, Detail};
use async_smtp::{self as smtp, EmailAddress, SmtpTransport};
use tokio::io::BufStream;
use tokio::task;

use crate::config::Config;
use crate::contact::{Contact, ContactId};
use crate::events::EventType;
use crate::login_param::{CertificateChecks, LoginParam, ServerLoginParam};
use crate::message::Message;
use crate::message::{self, MsgId};
use crate::mimefactory::MimeFactory;
use crate::net::connect_tcp;
use crate::net::session::SessionBufStream;
use crate::net::tls::wrap_tls;
use crate::oauth2::get_oauth2_access_token;
use crate::provider::Socket;
use crate::socks::Socks5Config;
use crate::sql;
use crate::{context::Context, scheduler::connectivity::ConnectivityStore};

/// SMTP write and read timeout.
const SMTP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(crate) struct Smtp {
    /// SMTP connection.
    transport: Option<SmtpTransport<Box<dyn SessionBufStream>>>,

    /// Email address we are sending from.
    from: Option<EmailAddress>,

    /// Timestamp of last successful send/receive network interaction
    /// (eg connect or send succeeded). On initialization and disconnect
    /// it is set to None.
    last_success: Option<SystemTime>,

    pub(crate) connectivity: ConnectivityStore,

    /// If sending the last message failed, contains the error message.
    pub(crate) last_send_error: Option<String>,
}

impl Smtp {
    /// Create a new Smtp instances.
    pub fn new() -> Self {
        Default::default()
    }

    /// Disconnect the SMTP transport and drop it entirely.
    pub fn disconnect(&mut self) {
        if let Some(mut transport) = self.transport.take() {
            // Closing connection with a QUIT command may take some time, especially if it's a
            // stale connection and an attempt to send the command times out. Send a command in a
            // separate task to avoid waiting for reply or timeout.
            task::spawn(async move { transport.quit().await });
        }
        self.last_success = None;
    }

    /// Return true if smtp was connected but is not known to
    /// have been successfully used the last 60 seconds
    pub fn has_maybe_stale_connection(&self) -> bool {
        if let Some(last_success) = self.last_success {
            SystemTime::now()
                .duration_since(last_success)
                .unwrap_or_default()
                .as_secs()
                > 60
        } else {
            false
        }
    }

    /// Check whether we are connected.
    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
    }

    /// Connect using configured parameters.
    pub async fn connect_configured(&mut self, context: &Context) -> Result<()> {
        if self.has_maybe_stale_connection() {
            info!(context, "Closing stale connection");
            self.disconnect();
        }

        if self.is_connected() {
            return Ok(());
        }

        self.connectivity.set_connecting(context).await;
        let lp = LoginParam::load_configured_params(context).await?;
        self.connect(
            context,
            &lp.smtp,
            &lp.socks5_config,
            &lp.addr,
            lp.provider.map_or(lp.socks5_config.is_some(), |provider| {
                provider.opt.strict_tls
            }),
        )
        .await
    }

    async fn connect_secure_socks5(
        &self,
        context: &Context,
        hostname: &str,
        port: u16,
        strict_tls: bool,
        socks5_config: Socks5Config,
    ) -> Result<SmtpTransport<Box<dyn SessionBufStream>>> {
        let socks5_stream = socks5_config
            .connect(context, hostname, port, SMTP_TIMEOUT, strict_tls)
            .await?;
        let tls_stream = wrap_tls(strict_tls, hostname, socks5_stream).await?;
        let buffered_stream = BufStream::new(tls_stream);
        let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
        let client = smtp::SmtpClient::new().smtp_utf8(true);
        let transport = SmtpTransport::new(client, session_stream).await?;
        Ok(transport)
    }

    async fn connect_starttls_socks5(
        &self,
        context: &Context,
        hostname: &str,
        port: u16,
        strict_tls: bool,
        socks5_config: Socks5Config,
    ) -> Result<SmtpTransport<Box<dyn SessionBufStream>>> {
        let socks5_stream = socks5_config
            .connect(context, hostname, port, SMTP_TIMEOUT, strict_tls)
            .await?;

        // Run STARTTLS command and convert the client back into a stream.
        let client = smtp::SmtpClient::new().smtp_utf8(true);
        let transport = SmtpTransport::new(client, BufStream::new(socks5_stream)).await?;
        let tcp_stream = transport.starttls().await?.into_inner();
        let tls_stream = wrap_tls(strict_tls, hostname, tcp_stream)
            .await
            .context("STARTTLS upgrade failed")?;
        let buffered_stream = BufStream::new(tls_stream);
        let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
        let client = smtp::SmtpClient::new().smtp_utf8(true).without_greeting();
        let transport = SmtpTransport::new(client, session_stream).await?;
        Ok(transport)
    }

    async fn connect_insecure_socks5(
        &self,
        context: &Context,
        hostname: &str,
        port: u16,
        socks5_config: Socks5Config,
    ) -> Result<SmtpTransport<Box<dyn SessionBufStream>>> {
        let socks5_stream = socks5_config
            .connect(context, hostname, port, SMTP_TIMEOUT, false)
            .await?;
        let buffered_stream = BufStream::new(socks5_stream);
        let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
        let client = smtp::SmtpClient::new().smtp_utf8(true);
        let transport = SmtpTransport::new(client, session_stream).await?;
        Ok(transport)
    }

    async fn connect_secure(
        &self,
        context: &Context,
        hostname: &str,
        port: u16,
        strict_tls: bool,
    ) -> Result<SmtpTransport<Box<dyn SessionBufStream>>> {
        let tcp_stream = connect_tcp(context, hostname, port, SMTP_TIMEOUT, false).await?;
        let tls_stream = wrap_tls(strict_tls, hostname, tcp_stream).await?;
        let buffered_stream = BufStream::new(tls_stream);
        let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
        let client = smtp::SmtpClient::new().smtp_utf8(true);
        let transport = SmtpTransport::new(client, session_stream).await?;
        Ok(transport)
    }

    async fn connect_starttls(
        &self,
        context: &Context,
        hostname: &str,
        port: u16,
        strict_tls: bool,
    ) -> Result<SmtpTransport<Box<dyn SessionBufStream>>> {
        let tcp_stream = connect_tcp(context, hostname, port, SMTP_TIMEOUT, strict_tls).await?;

        // Run STARTTLS command and convert the client back into a stream.
        let client = smtp::SmtpClient::new().smtp_utf8(true);
        let transport = SmtpTransport::new(client, BufStream::new(tcp_stream)).await?;
        let tcp_stream = transport.starttls().await?.into_inner();
        let tls_stream = wrap_tls(strict_tls, hostname, tcp_stream)
            .await
            .context("STARTTLS upgrade failed")?;
        let buffered_stream = BufStream::new(tls_stream);
        let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
        let client = smtp::SmtpClient::new().smtp_utf8(true).without_greeting();
        let transport = SmtpTransport::new(client, session_stream).await?;
        Ok(transport)
    }

    async fn connect_insecure(
        &self,
        context: &Context,
        hostname: &str,
        port: u16,
    ) -> Result<SmtpTransport<Box<dyn SessionBufStream>>> {
        let tcp_stream = connect_tcp(context, hostname, port, SMTP_TIMEOUT, false).await?;
        let buffered_stream = BufStream::new(tcp_stream);
        let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
        let client = smtp::SmtpClient::new().smtp_utf8(true);
        let transport = SmtpTransport::new(client, session_stream).await?;
        Ok(transport)
    }

    /// Connect using the provided login params.
    pub async fn connect(
        &mut self,
        context: &Context,
        lp: &ServerLoginParam,
        socks5_config: &Option<Socks5Config>,
        addr: &str,
        provider_strict_tls: bool,
    ) -> Result<()> {
        if self.is_connected() {
            warn!(context, "SMTP already connected.");
            return Ok(());
        }

        if lp.server.is_empty() || lp.port == 0 {
            bail!("bad connection parameters");
        }

        let from = EmailAddress::new(addr.to_string())
            .with_context(|| format!("invalid login address {addr}"))?;

        self.from = Some(from);

        let domain = &lp.server;
        let port = lp.port;

        let strict_tls = match lp.certificate_checks {
            CertificateChecks::Automatic => provider_strict_tls,
            CertificateChecks::Strict => true,
            CertificateChecks::AcceptInvalidCertificates
            | CertificateChecks::AcceptInvalidCertificates2 => false,
        };

        let mut transport = if let Some(socks5_config) = socks5_config {
            match lp.security {
                Socket::Automatic => bail!("SMTP port security is not configured"),
                Socket::Ssl => {
                    self.connect_secure_socks5(
                        context,
                        domain,
                        port,
                        strict_tls,
                        socks5_config.clone(),
                    )
                    .await?
                }
                Socket::Starttls => {
                    self.connect_starttls_socks5(
                        context,
                        domain,
                        port,
                        strict_tls,
                        socks5_config.clone(),
                    )
                    .await?
                }
                Socket::Plain => {
                    self.connect_insecure_socks5(context, domain, port, socks5_config.clone())
                        .await?
                }
            }
        } else {
            match lp.security {
                Socket::Automatic => bail!("SMTP port security is not configured"),
                Socket::Ssl => {
                    self.connect_secure(context, domain, port, strict_tls)
                        .await?
                }
                Socket::Starttls => {
                    self.connect_starttls(context, domain, port, strict_tls)
                        .await?
                }
                Socket::Plain => self.connect_insecure(context, domain, port).await?,
            }
        };

        // Authenticate.
        {
            let (creds, mechanism) = if lp.oauth2 {
                // oauth2
                let send_pw = &lp.password;
                let access_token = get_oauth2_access_token(context, addr, send_pw, false).await?;
                if access_token.is_none() {
                    bail!("SMTP OAuth 2 error {}", addr);
                }
                let user = &lp.user;
                (
                    smtp::authentication::Credentials::new(
                        user.to_string(),
                        access_token.unwrap_or_default(),
                    ),
                    vec![smtp::authentication::Mechanism::Xoauth2],
                )
            } else {
                // plain
                let user = lp.user.clone();
                let pw = lp.password.clone();
                (
                    smtp::authentication::Credentials::new(user, pw),
                    vec![
                        smtp::authentication::Mechanism::Plain,
                        smtp::authentication::Mechanism::Login,
                    ],
                )
            };
            transport.try_login(&creds, &mechanism).await?;
        }

        self.transport = Some(transport);
        self.last_success = Some(SystemTime::now());

        context.emit_event(EventType::SmtpConnected(format!(
            "SMTP-LOGIN as {} ok",
            lp.user,
        )));

        Ok(())
    }
}

pub(crate) enum SendResult {
    /// Message was sent successfully.
    Success,

    /// Permanent error, message sending has failed.
    Failure(Error),

    /// Temporary error, the message should be retried later.
    Retry,
}

/// Tries to send a message.
pub(crate) async fn smtp_send(
    context: &Context,
    recipients: &[async_smtp::EmailAddress],
    message: &str,
    smtp: &mut Smtp,
    msg_id: MsgId,
) -> SendResult {
    if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
        info!(context, "smtp-sending out mime message:");
        println!("{message}");
    }

    smtp.connectivity.set_working(context).await;

    if let Err(err) = smtp
        .connect_configured(context)
        .await
        .context("Failed to open SMTP connection")
    {
        smtp.last_send_error = Some(format!("{err:#}"));
        return SendResult::Retry;
    }

    let send_result = smtp.send(context, recipients, message.as_bytes()).await;
    smtp.last_send_error = send_result.as_ref().err().map(|e| e.to_string());

    let status = match send_result {
        Err(crate::smtp::send::Error::SmtpSend(err)) => {
            // Remote error, retry later.
            info!(context, "SMTP failed to send: {:?}", &err);

            let res = match err {
                async_smtp::error::Error::Permanent(ref response) => {
                    // Workaround for incorrectly configured servers returning permanent errors
                    // instead of temporary ones.
                    let maybe_transient = match response.code {
                        // Sometimes servers send a permanent error when actually it is a temporary error
                        // For documentation see <https://tools.ietf.org/html/rfc3463>
                        Code {
                            category: Category::MailSystem,
                            detail: Detail::Zero,
                            ..
                        } => {
                            // Ignore status code 5.5.0, see <https://support.delta.chat/t/every-other-message-gets-stuck/877/2>
                            // Maybe incorrectly configured Postfix milter with "reject" instead of "tempfail", which returns
                            // "550 5.5.0 Service unavailable" instead of "451 4.7.1 Service unavailable - try again later".
                            //
                            // Other enhanced status codes, such as Postfix
                            // "550 5.1.1 <foobar@example.org>: Recipient address rejected: User unknown in local recipient table"
                            // are not ignored.
                            response.first_word() == Some("5.5.0")
                        }
                        _ => false,
                    };

                    if maybe_transient {
                        info!(context, "Permanent error that is likely to actually be transient, postponing retry for later");
                        SendResult::Retry
                    } else {
                        info!(context, "Permanent error, message sending failed");
                        // If we do not retry, add an info message to the chat.
                        // Yandex error "554 5.7.1 [2] Message rejected under suspicion of SPAM; https://ya.cc/..."
                        // should definitely go here, because user has to open the link to
                        // resume message sending.
                        SendResult::Failure(format_err!("Permanent SMTP error: {}", err))
                    }
                }
                async_smtp::error::Error::Transient(ref response) => {
                    // We got a transient 4xx response from SMTP server.
                    // Give some time until the server-side error maybe goes away.

                    if let Some(first_word) = response.first_word() {
                        if first_word.ends_with(".1.1")
                            || first_word.ends_with(".1.2")
                            || first_word.ends_with(".1.3")
                        {
                            // Sometimes we receive transient errors that should be permanent.
                            // Any extended smtp status codes like x.1.1, x.1.2 or x.1.3 that we
                            // receive as a transient error are misconfigurations of the smtp server.
                            // See <https://tools.ietf.org/html/rfc3463#section-3.2>
                            info!(context, "Received extended status code {} for a transient error. This looks like a misconfigured SMTP server, let's fail immediately", first_word);
                            SendResult::Failure(format_err!("Permanent SMTP error: {}", err))
                        } else {
                            info!(
                                context,
                                "Transient error with status code {}, postponing retry for later",
                                first_word
                            );
                            SendResult::Retry
                        }
                    } else {
                        info!(
                            context,
                            "Transient error without status code, postponing retry for later"
                        );
                        SendResult::Retry
                    }
                }
                _ => {
                    info!(
                        context,
                        "Message sending failed without error returned by the server, retry later"
                    );
                    SendResult::Retry
                }
            };

            // this clears last_success info
            info!(context, "Failed to send message over SMTP, disconnecting");
            smtp.disconnect();

            res
        }
        Err(crate::smtp::send::Error::Envelope(err)) => {
            // Local error, job is invalid, do not retry.
            smtp.disconnect();
            warn!(context, "SMTP job is invalid: {}", err);
            SendResult::Failure(err)
        }
        Err(crate::smtp::send::Error::NoTransport) => {
            // Should never happen.
            // It does not even make sense to disconnect here.
            error!(context, "SMTP job failed because SMTP has no transport");
            SendResult::Failure(format_err!("SMTP has not transport"))
        }
        Err(crate::smtp::send::Error::Other(err)) => {
            // Local error, job is invalid, do not retry.
            smtp.disconnect();
            warn!(context, "unable to load job: {}", err);
            SendResult::Failure(err)
        }
        Ok(()) => SendResult::Success,
    };

    if let SendResult::Failure(err) = &status {
        // We couldn't send the message, so mark it as failed
        match Message::load_from_db(context, msg_id).await {
            Ok(mut msg) => {
                if let Err(err) = message::set_msg_failed(context, &mut msg, &err.to_string()).await
                {
                    error!(context, "Failed to mark {msg_id} as failed: {err:#}.");
                }
            }
            Err(err) => {
                error!(
                    context,
                    "Failed to load {msg_id} to mark it as failed: {err:#}."
                );
            }
        }
    }
    status
}

/// Sends message identified by `smtp` table rowid over SMTP connection.
///
/// Removes row if the message should not be retried, otherwise increments retry count.
pub(crate) async fn send_msg_to_smtp(
    context: &Context,
    smtp: &mut Smtp,
    rowid: i64,
) -> anyhow::Result<()> {
    if let Err(err) = smtp
        .connect_configured(context)
        .await
        .context("SMTP connection failure")
    {
        smtp.last_send_error = Some(format!("{err:#}"));
        return Err(err);
    }

    // Increase retry count as soon as we have an SMTP connection. This ensures that the message is
    // eventually removed from the queue by exceeding retry limit even in case of an error that
    // keeps happening early in the message sending code, e.g. failure to read the message from the
    // database.
    context
        .sql
        .execute("UPDATE smtp SET retries=retries+1 WHERE id=?", (rowid,))
        .await
        .context("failed to update retries count")?;

    let (body, recipients, msg_id, retries) = context
        .sql
        .query_row(
            "SELECT mime, recipients, msg_id, retries FROM smtp WHERE id=?",
            (rowid,),
            |row| {
                let mime: String = row.get(0)?;
                let recipients: String = row.get(1)?;
                let msg_id: MsgId = row.get(2)?;
                let retries: i64 = row.get(3)?;
                Ok((mime, recipients, msg_id, retries))
            },
        )
        .await?;
    if retries > 6 {
        let mut msg = Message::load_from_db(context, msg_id).await?;
        message::set_msg_failed(context, &mut msg, "Number of retries exceeded the limit.").await?;
        context
            .sql
            .execute("DELETE FROM smtp WHERE id=?", (rowid,))
            .await
            .context("failed to remove message with exceeded retry limit from smtp table")?;
        return Ok(());
    }
    info!(
        context,
        "Try number {retries} to send message {msg_id} (entry {rowid}) over SMTP"
    );

    let recipients_list = recipients
        .split(' ')
        .filter_map(
            |addr| match async_smtp::EmailAddress::new(addr.to_string()) {
                Ok(addr) => Some(addr),
                Err(err) => {
                    warn!(context, "invalid recipient: {} {:?}", addr, err);
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    let status = smtp_send(context, &recipients_list, body.as_str(), smtp, msg_id).await;

    match status {
        SendResult::Retry => {}
        SendResult::Success | SendResult::Failure(_) => {
            context
                .sql
                .execute("DELETE FROM smtp WHERE id=?", (rowid,))
                .await?;
        }
    };

    match status {
        SendResult::Retry => Err(format_err!("Retry")),
        SendResult::Success => {
            msg_id.set_delivered(context).await?;
            Ok(())
        }
        SendResult::Failure(err) => Err(format_err!("{}", err)),
    }
}

/// Attempts to send queued MDNs.
async fn send_mdns(context: &Context, connection: &mut Smtp) -> Result<()> {
    loop {
        if !context.ratelimit.read().await.can_send() {
            info!(context, "Ratelimiter does not allow sending MDNs now");
            return Ok(());
        }

        let more_mdns = send_mdn(context, connection).await?;
        if !more_mdns {
            // No more MDNs to send.
            return Ok(());
        }
    }
}

/// Tries to send all messages currently in `smtp`, `smtp_status_updates` and `smtp_mdns` tables.
///
/// Logs and ignores SMTP errors to ensure that a single SMTP message constantly failing to be sent
/// does not block other messages in the queue from being sent.
pub(crate) async fn send_smtp_messages(context: &Context, connection: &mut Smtp) -> Result<()> {
    let ratelimited = if context.ratelimit.read().await.can_send() {
        // add status updates and sync messages to end of sending queue
        context.flush_status_updates().await?;
        context.send_sync_msg().await?;
        false
    } else {
        true
    };

    let rowids = context
        .sql
        .query_map(
            "SELECT id FROM smtp ORDER BY id ASC",
            (),
            |row| {
                let rowid: i64 = row.get(0)?;
                Ok(rowid)
            },
            |rowids| {
                rowids
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            },
        )
        .await?;

    info!(context, "Selected rows from SMTP queue: {rowids:?}.");
    for rowid in rowids {
        send_msg_to_smtp(context, connection, rowid)
            .await
            .context("failed to send message")?;
    }

    // although by slow sending, ratelimit may have been expired meanwhile,
    // do not attempt to send MDNs if ratelimited happened before on status-updates/sync:
    // instead, let the caller recall this function so that more important status-updates/sync are sent out.
    if !ratelimited {
        send_mdns(context, connection)
            .await
            .context("failed to send MDNs")?;
    }
    Ok(())
}

/// Tries to send MDN for message `msg_id` to `contact_id`.
///
/// Attempts to aggregate additional MDNs for `contact_id` into sent MDN.
///
/// On failure returns an error without removing any `smtp_mdns` entries, the caller is responsible
/// for removing the corresponding entry to prevent endless loop in case the entry is invalid, e.g.
/// points to non-existent message or contact.
///
/// Returns true on success, false on temporary error.
async fn send_mdn_msg_id(
    context: &Context,
    msg_id: MsgId,
    contact_id: ContactId,
    smtp: &mut Smtp,
) -> Result<bool> {
    let contact = Contact::get_by_id(context, contact_id).await?;
    if contact.is_blocked() {
        return Err(format_err!("Contact is blocked"));
    }

    // Try to aggregate additional MDNs into this MDN.
    let (additional_msg_ids, additional_rfc724_mids): (Vec<MsgId>, Vec<String>) = context
        .sql
        .query_map(
            "SELECT msg_id, rfc724_mid
             FROM smtp_mdns
             WHERE from_id=? AND msg_id!=?",
            (contact_id, msg_id),
            |row| {
                let msg_id: MsgId = row.get(0)?;
                let rfc724_mid: String = row.get(1)?;
                Ok((msg_id, rfc724_mid))
            },
            |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )
        .await?
        .into_iter()
        .unzip();

    let msg = Message::load_from_db(context, msg_id).await?;
    let mimefactory = MimeFactory::from_mdn(context, &msg, additional_rfc724_mids).await?;
    let rendered_msg = mimefactory.render(context).await?;
    let body = rendered_msg.message;

    let addr = contact.get_addr();
    let recipient = async_smtp::EmailAddress::new(addr.to_string())
        .map_err(|err| format_err!("invalid recipient: {} {:?}", addr, err))?;
    let recipients = vec![recipient];

    match smtp_send(context, &recipients, &body, smtp, msg_id).await {
        SendResult::Success => {
            info!(context, "Successfully sent MDN for {}", msg_id);
            context
                .sql
                .execute("DELETE FROM smtp_mdns WHERE msg_id = ?", (msg_id,))
                .await?;
            if !additional_msg_ids.is_empty() {
                let q = format!(
                    "DELETE FROM smtp_mdns WHERE msg_id IN({})",
                    sql::repeat_vars(additional_msg_ids.len())
                );
                context
                    .sql
                    .execute(&q, rusqlite::params_from_iter(additional_msg_ids))
                    .await?;
            }
            Ok(true)
        }
        SendResult::Retry => {
            info!(
                context,
                "Temporary SMTP failure while sending an MDN for {}", msg_id
            );
            Ok(false)
        }
        SendResult::Failure(err) => Err(err),
    }
}

/// Tries to send a single MDN. Returns false if there are no MDNs to send.
async fn send_mdn(context: &Context, smtp: &mut Smtp) -> Result<bool> {
    let mdns_enabled = context.get_config_bool(Config::MdnsEnabled).await?;
    if !mdns_enabled {
        // User has disabled MDNs.
        context.sql.execute("DELETE FROM smtp_mdns", []).await?;
        return Ok(false);
    }
    info!(context, "Sending MDNs");

    context
        .sql
        .execute("DELETE FROM smtp_mdns WHERE retries > 6", [])
        .await?;
    let msg_row = match context
        .sql
        .query_row_optional(
            "SELECT msg_id, from_id FROM smtp_mdns ORDER BY retries LIMIT 1",
            [],
            |row| {
                let msg_id: MsgId = row.get(0)?;
                let from_id: ContactId = row.get(1)?;
                Ok((msg_id, from_id))
            },
        )
        .await?
    {
        Some(msg_row) => msg_row,
        None => return Ok(false),
    };
    let (msg_id, contact_id) = msg_row;

    context
        .sql
        .execute(
            "UPDATE smtp_mdns SET retries=retries+1 WHERE msg_id=?",
            (msg_id,),
        )
        .await
        .context("failed to update MDN retries count")?;

    let res = send_mdn_msg_id(context, msg_id, contact_id, smtp).await;
    if let Err(ref err) = res {
        // If there is an error, for example there is no message corresponding to the msg_id in the
        // database, do not try to send this MDN again.
        warn!(context, "Error sending MDN for {msg_id}, removing it: {err:#}.");
        context
            .sql
            .execute("DELETE FROM smtp_mdns WHERE msg_id = ?", (msg_id,))
            .await?;
    }
    // If there's a temporary error, pretend there are no more MDNs to send. It's unlikely that
    // other MDNs could be sent successfully in case of connectivity problems.
    res
}
