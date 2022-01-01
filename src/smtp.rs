//! # SMTP transport module.

pub mod send;

use std::time::{Duration, SystemTime};

use anyhow::{format_err, Context as _};
use async_smtp::smtp::client::net::ClientTlsParameters;
use async_smtp::smtp::response::{Category, Code, Detail};
use async_smtp::{error, smtp, EmailAddress, ServerAddress};

use crate::constants::DC_LP_AUTH_OAUTH2;
use crate::events::EventType;
use crate::job::Status;
use crate::login_param::{
    dc_build_tls, CertificateChecks, LoginParam, ServerLoginParam, Socks5Config,
};
use crate::message::{self, MsgId};
use crate::oauth2::dc_get_oauth2_access_token;
use crate::provider::Socket;
use crate::{context::Context, scheduler::connectivity::ConnectivityStore};

/// SMTP write and read timeout in seconds.
const SMTP_TIMEOUT: u64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Bad parameters")]
    BadParameters,
    #[error("Invalid login address {address}: {error}")]
    InvalidLoginAddress {
        address: String,
        #[source]
        error: error::Error,
    },
    #[error("SMTP failed to connect: {0}")]
    ConnectionFailure(#[source] smtp::error::Error),
    #[error("SMTP oauth2 error {address}")]
    Oauth2 { address: String },
    #[error("TLS error {0}")]
    Tls(#[from] async_native_tls::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub(crate) struct Smtp {
    transport: Option<smtp::SmtpTransport>,

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
    pub async fn disconnect(&mut self) {
        if let Some(mut transport) = self.transport.take() {
            transport.close().await.ok();
        }
        self.last_success = None;
    }

    /// Return true if smtp was connected but is not known to
    /// have been successfully used the last 60 seconds
    pub async fn has_maybe_stale_connection(&self) -> bool {
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
    pub async fn is_connected(&self) -> bool {
        self.transport
            .as_ref()
            .map(|t| t.is_connected())
            .unwrap_or_default()
    }

    /// Connect using configured parameters.
    pub async fn connect_configured(&mut self, context: &Context) -> Result<()> {
        if self.is_connected().await {
            return Ok(());
        }

        self.connectivity.set_connecting(context).await;
        let lp = LoginParam::from_database(context, "configured_").await?;
        self.connect(
            context,
            &lp.smtp,
            &lp.socks5_config,
            &lp.addr,
            lp.server_flags & DC_LP_AUTH_OAUTH2 != 0,
            lp.provider
                .map_or(lp.socks5_config.is_some(), |provider| provider.strict_tls),
        )
        .await
    }

    /// Connect using the provided login params.
    pub async fn connect(
        &mut self,
        context: &Context,
        lp: &ServerLoginParam,
        socks5_config: &Option<Socks5Config>,
        addr: &str,
        oauth2: bool,
        provider_strict_tls: bool,
    ) -> Result<()> {
        if self.is_connected().await {
            warn!(context, "SMTP already connected.");
            return Ok(());
        }

        if lp.server.is_empty() || lp.port == 0 {
            return Err(Error::BadParameters);
        }

        let from =
            EmailAddress::new(addr.to_string()).map_err(|err| Error::InvalidLoginAddress {
                address: addr.to_string(),
                error: err,
            })?;

        self.from = Some(from);

        let domain = &lp.server;
        let port = lp.port;

        let strict_tls = match lp.certificate_checks {
            CertificateChecks::Automatic => provider_strict_tls,
            CertificateChecks::Strict => true,
            CertificateChecks::AcceptInvalidCertificates
            | CertificateChecks::AcceptInvalidCertificates2 => false,
        };
        let tls_config = dc_build_tls(strict_tls);
        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls_config);

        let (creds, mechanism) = if oauth2 {
            // oauth2
            let send_pw = &lp.password;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, false).await?;
            if access_token.is_none() {
                return Err(Error::Oauth2 {
                    address: addr.to_string(),
                });
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

        let security = match lp.security {
            Socket::Plain => smtp::ClientSecurity::None,
            Socket::Starttls => smtp::ClientSecurity::Required(tls_parameters),
            _ => smtp::ClientSecurity::Wrapper(tls_parameters),
        };

        let client =
            smtp::SmtpClient::with_security(ServerAddress::new(domain.to_string(), port), security);

        let mut client = client
            .smtp_utf8(true)
            .credentials(creds)
            .authentication_mechanism(mechanism)
            .connection_reuse(smtp::ConnectionReuseParameters::ReuseUnlimited)
            .timeout(Some(Duration::from_secs(SMTP_TIMEOUT)));

        if let Some(socks5_config) = socks5_config {
            client = client.use_socks5(socks5_config.to_async_smtp_socks5_config());
        }

        let mut trans = client.into_transport();
        if let Err(err) = trans.connect().await {
            return Err(Error::ConnectionFailure(err));
        }

        self.transport = Some(trans);
        self.last_success = Some(SystemTime::now());

        context.emit_event(EventType::SmtpConnected(format!(
            "SMTP-LOGIN as {} ok",
            lp.user,
        )));

        Ok(())
    }
}

pub(crate) async fn smtp_send(
    context: &Context,
    recipients: Vec<async_smtp::EmailAddress>,
    message: String,
    smtp: &mut Smtp,
    msg_id: MsgId,
    rowid: i64,
) -> Status {
    if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
        info!(context, "smtp-sending out mime message:");
        println!("{}", message);
    }

    smtp.connectivity.set_working(context).await;

    let send_result = smtp
        .send(context, recipients, message.into_bytes(), rowid)
        .await;
    smtp.last_send_error = send_result.as_ref().err().map(|e| e.to_string());

    let status = match send_result {
        Err(crate::smtp::send::Error::SmtpSend(err)) => {
            // Remote error, retry later.
            warn!(context, "SMTP failed to send: {:?}", &err);

            let res = match err {
                async_smtp::smtp::error::Error::Permanent(ref response) => {
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
                            response.first_word() == Some(&"5.5.0".to_string())
                        }
                        _ => false,
                    };

                    if maybe_transient {
                        Status::RetryLater
                    } else {
                        // If we do not retry, add an info message to the chat.
                        // Yandex error "554 5.7.1 [2] Message rejected under suspicion of SPAM; https://ya.cc/..."
                        // should definitely go here, because user has to open the link to
                        // resume message sending.
                        Status::Finished(Err(format_err!("Permanent SMTP error: {}", err)))
                    }
                }
                async_smtp::smtp::error::Error::Transient(ref response) => {
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
                            info!(context, "Received extended status code {} for a transient error. This looks like a misconfigured smtp server, let's fail immediatly", first_word);
                            Status::Finished(Err(format_err!("Permanent SMTP error: {}", err)))
                        } else {
                            Status::RetryLater
                        }
                    } else {
                        Status::RetryLater
                    }
                }
                _ => {
                    if smtp.has_maybe_stale_connection().await {
                        info!(context, "stale connection? immediately reconnecting");
                        Status::RetryNow
                    } else {
                        Status::RetryLater
                    }
                }
            };

            // this clears last_success info
            smtp.disconnect().await;

            res
        }
        Err(crate::smtp::send::Error::Envelope(err)) => {
            // Local error, job is invalid, do not retry.
            smtp.disconnect().await;
            warn!(context, "SMTP job is invalid: {}", err);
            Status::Finished(Err(err.into()))
        }
        Err(crate::smtp::send::Error::NoTransport) => {
            // Should never happen.
            // It does not even make sense to disconnect here.
            error!(context, "SMTP job failed because SMTP has no transport");
            Status::Finished(Err(format_err!("SMTP has not transport")))
        }
        Err(crate::smtp::send::Error::Other(err)) => {
            // Local error, job is invalid, do not retry.
            smtp.disconnect().await;
            warn!(context, "unable to load job: {}", err);
            Status::Finished(Err(err))
        }
        Ok(()) => Status::Finished(Ok(())),
    };

    if let Status::Finished(Err(err)) = &status {
        // We couldn't send the message, so mark it as failed
        message::set_msg_failed(context, msg_id, Some(err.to_string())).await;
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
        smtp.last_send_error = Some(format!("SMTP connection failure: {:#}", err));
        return Err(err);
    }

    let (body, recipients, msg_id) = context
        .sql
        .query_row(
            "SELECT mime, recipients, msg_id FROM smtp WHERE id=?",
            paramsv![rowid],
            |row| {
                let mime: String = row.get(0)?;
                let recipients: String = row.get(1)?;
                let msg_id: MsgId = row.get(2)?;
                Ok((mime, recipients, msg_id))
            },
        )
        .await?;
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

    let status = smtp_send(context, recipients_list, body, smtp, msg_id, rowid).await;
    match status {
        Status::Finished(res) => {
            if res.is_ok() {
                msg_id.set_delivered(context).await?;

                context
                    .sql
                    .execute("DELETE FROM smtp WHERE id=?", paramsv![rowid])
                    .await?;
            }
            res
        }
        Status::RetryNow | Status::RetryLater => {
            context
                .sql
                .execute(
                    "UPDATE smtp SET retries=retries+1 WHERE id=?",
                    paramsv![rowid],
                )
                .await
                .context("failed to update retries count")?;
            Err(format_err!("Retry"))
        }
    }
}

/// Tries to send all messages currently in `smtp` table.
///
/// Logs and ignores SMTP errors to ensure that a single SMTP message constantly failing to be sent
/// does not block other messages in the queue from being sent.
pub(crate) async fn send_smtp_messages(
    context: &Context,
    connection: &mut Smtp,
) -> anyhow::Result<()> {
    context.send_sync_msg().await?; // Add sync message to the end of the queue if needed.
    context
        .sql
        .execute("DELETE FROM smtp WHERE retries > 5", paramsv![])
        .await?;
    let rowids = context
        .sql
        .query_map(
            "SELECT id FROM smtp ORDER BY id ASC",
            paramsv![],
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
    for rowid in rowids {
        if let Err(err) = send_msg_to_smtp(context, connection, rowid).await {
            info!(context, "Failed to send message over SMTP: {:#}.", err);
        }
    }
    Ok(())
}
