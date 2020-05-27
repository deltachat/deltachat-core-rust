//! Email accounts autoconfiguration process module

mod auto_mozilla;
mod auto_outlook;
mod read_url;

use anyhow::{bail, ensure, format_err, Context as _, Result};
use async_std::prelude::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::imap::Imap;
use crate::login_param::{CertificateChecks, LoginParam};
use crate::message::Message;
use crate::oauth2::*;
use crate::smtp::Smtp;
use crate::{chat, e2ee, provider};

use auto_mozilla::moz_autoconfigure;
use auto_outlook::outlk_autodiscover;

macro_rules! progress {
    ($context:tt, $progress:expr) => {
        assert!(
            $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.emit_event($crate::events::Event::ConfigureProgress($progress));
    };
}

impl Context {
    /// Checks if the context is already configured.
    pub async fn is_configured(&self) -> bool {
        self.sql.get_raw_config_bool(self, "configured").await
    }

    /// Configures this account with the currently set parameters.
    pub async fn configure(&self) -> Result<()> {
        use futures::future::FutureExt;

        ensure!(
            !self.scheduler.read().await.is_running(),
            "cannot configure, already running"
        );
        ensure!(
            self.sql.is_open().await,
            "cannot configure, database not opened."
        );
        let cancel_channel = self.alloc_ongoing().await?;

        let res = self
            .inner_configure()
            .race(cancel_channel.recv().map(|_| {
                progress!(self, 0);
                Ok(())
            }))
            .await;

        self.free_ongoing().await;

        res
    }

    async fn inner_configure(&self) -> Result<()> {
        info!(self, "Configure ...");

        let was_configured_before = self.is_configured().await;
        let mut param = LoginParam::from_database(self, "").await;
        let success = configure(self, &mut param).await;

        if let Some(provider) = provider::get_provider_info(&param.addr) {
            if !was_configured_before {
                if let Some(config_defaults) = &provider.config_defaults {
                    for def in config_defaults.iter() {
                        info!(self, "apply config_defaults {}={}", def.key, def.value);
                        self.set_config(def.key, Some(def.value)).await?;
                    }
                }
            }

            if !provider.after_login_hint.is_empty() {
                let mut msg = Message::new(Viewtype::Text);
                msg.text = Some(provider.after_login_hint.to_string());
                if chat::add_device_msg(self, Some("core-provider-info"), Some(&mut msg))
                    .await
                    .is_err()
                {
                    warn!(self, "cannot add after_login_hint as core-provider-info");
                }
            }
        }

        match success {
            Ok(_) => {
                progress!(self, 1000);
                Ok(())
            }
            Err(err) => {
                error!(self, "Configure Failed: {}", err);
                progress!(self, 0);
                Err(err)
            }
        }
    }
}

async fn configure(ctx: &Context, param: &mut LoginParam) -> Result<()> {
    let mut param_autoconfig: Option<LoginParam> = None;
    let mut keep_flags = 0;

    // Read login parameters from the database
    progress!(ctx, 1);
    ensure!(!param.addr.is_empty(), "Please enter an email address.");

    // Step 1: Load the parameters and check email-address and password

    if 0 != param.server_flags & DC_LP_AUTH_OAUTH2 {
        // the used oauth2 addr may differ, check this.
        // if dc_get_oauth2_addr() is not available in the oauth2 implementation, just use the given one.
        progress!(ctx, 10);
        if let Some(oauth2_addr) = dc_get_oauth2_addr(ctx, &param.addr, &param.mail_pw)
            .await
            .and_then(|e| e.parse().ok())
        {
            info!(ctx, "Authorized address is {}", oauth2_addr);
            param.addr = oauth2_addr;
            ctx.sql
                .set_raw_config(ctx, "addr", Some(param.addr.as_str()))
                .await?;
        }
        progress!(ctx, 20);
    }
    // no oauth? - just continue it's no error

    let parsed: EmailAddress = param.addr.parse().context("Bad email-address")?;
    let param_domain = parsed.domain;
    let param_addr_urlencoded = utf8_percent_encode(&param.addr, NON_ALPHANUMERIC).to_string();

    // Step 2: Autoconfig
    progress!(ctx, 200);

    // param.mail_user.is_empty() -- the user can enter a loginname which is used by autoconfig then
    // param.send_pw.is_empty()   -- the password cannot be auto-configured and is no criterion for
    //                               autoconfig or not
    if param.mail_server.is_empty()
        && param.mail_port == 0
        && param.send_server.is_empty()
        && param.send_port == 0
        && param.send_user.is_empty()
        && (param.server_flags & !DC_LP_AUTH_OAUTH2) == 0
    {
        // no advanced parameters entered by the user: query provider-database or do Autoconfig
        keep_flags = param.server_flags & DC_LP_AUTH_OAUTH2;
        if let Some(new_param) = get_offline_autoconfig(ctx, &param) {
            // got parameters from our provider-database, skip Autoconfig, preserve the OAuth2 setting
            param_autoconfig = Some(new_param);
        }

        if param_autoconfig.is_none() {
            param_autoconfig =
                get_autoconfig(ctx, param, &param_domain, &param_addr_urlencoded).await;
        }
    }

    // C.  Do we have any autoconfig result?
    progress!(ctx, 500);
    if let Some(ref cfg) = param_autoconfig {
        info!(ctx, "Got autoconfig: {}", &cfg);
        if !cfg.mail_user.is_empty() {
            param.mail_user = cfg.mail_user.clone();
        }
        // all other values are always NULL when entering autoconfig
        param.mail_server = cfg.mail_server.clone();
        param.mail_port = cfg.mail_port;
        param.send_server = cfg.send_server.clone();
        param.send_port = cfg.send_port;
        param.send_user = cfg.send_user.clone();
        param.server_flags = cfg.server_flags;
        // although param_autoconfig's data are no longer needed from,
        // it is used to later to prevent trying variations of port/server/logins
    }
    param.server_flags |= keep_flags;

    // Step 3: Fill missing fields with defaults
    if param.mail_server.is_empty() {
        param.mail_server = format!("imap.{}", param_domain,)
    }
    if param.mail_port == 0 {
        param.mail_port = if 0 != param.server_flags & (0x100 | 0x400) {
            143
        } else {
            993
        }
    }
    if param.mail_user.is_empty() {
        param.mail_user = param.addr.clone();
    }
    if param.send_server.is_empty() && !param.mail_server.is_empty() {
        param.send_server = param.mail_server.clone();
        if param.send_server.starts_with("imap.") {
            param.send_server = param.send_server.replacen("imap", "smtp", 1);
        }
    }
    if param.send_port == 0 {
        param.send_port = if 0 != param.server_flags & DC_LP_SMTP_SOCKET_STARTTLS as i32 {
            587
        } else if 0 != param.server_flags & DC_LP_SMTP_SOCKET_PLAIN as i32 {
            25
        } else {
            465
        }
    }
    if param.send_user.is_empty() && !param.mail_user.is_empty() {
        param.send_user = param.mail_user.clone();
    }
    if param.send_pw.is_empty() && !param.mail_pw.is_empty() {
        param.send_pw = param.mail_pw.clone()
    }
    if !dc_exactly_one_bit_set(param.server_flags & DC_LP_AUTH_FLAGS as i32) {
        param.server_flags &= !(DC_LP_AUTH_FLAGS as i32);
        param.server_flags |= DC_LP_AUTH_NORMAL as i32
    }
    if !dc_exactly_one_bit_set(param.server_flags & DC_LP_IMAP_SOCKET_FLAGS as i32) {
        param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS as i32);
        param.server_flags |= if param.send_port == 143 {
            DC_LP_IMAP_SOCKET_STARTTLS as i32
        } else {
            DC_LP_IMAP_SOCKET_SSL as i32
        }
    }
    if !dc_exactly_one_bit_set(param.server_flags & (DC_LP_SMTP_SOCKET_FLAGS as i32)) {
        param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
        param.server_flags |= if param.send_port == 587 {
            DC_LP_SMTP_SOCKET_STARTTLS as i32
        } else if param.send_port == 25 {
            DC_LP_SMTP_SOCKET_PLAIN as i32
        } else {
            DC_LP_SMTP_SOCKET_SSL as i32
        }
    }

    // do we have a complete configuration?
    ensure!(
        !param.mail_server.is_empty()
            && param.mail_port != 0
            && !param.mail_user.is_empty()
            && !param.mail_pw.is_empty()
            && !param.send_server.is_empty()
            && param.send_port != 0
            && !param.send_user.is_empty()
            && !param.send_pw.is_empty()
            && param.server_flags != 0,
        "Account settings incomplete."
    );

    progress!(ctx, 600);
    // try to connect to IMAP - if we did not got an autoconfig,
    // do some further tries with different settings and username variations
    let (_s, r) = async_std::sync::channel(1);
    let mut imap = Imap::new(r);

    try_imap_connections(ctx, param, param_autoconfig.is_some(), &mut imap).await?;
    progress!(ctx, 800);

    try_smtp_connections(ctx, param, param_autoconfig.is_some()).await?;
    progress!(ctx, 900);

    let create_mvbox = ctx.get_config_bool(Config::MvboxWatch).await
        || ctx.get_config_bool(Config::MvboxMove).await;

    imap.configure_folders(ctx, create_mvbox)
        .await
        .context("configuring folders failed")?;

    imap.select_with_uidvalidity(ctx, "INBOX")
        .await
        .context("could not read INBOX status")?;

    drop(imap);

    progress!(ctx, 910);
    // configuration success - write back the configured parameters with the
    // "configured_" prefix; also write the "configured"-flag */
    // the trailing underscore is correct
    param.save_to_database(ctx, "configured_").await?;
    ctx.sql.set_raw_config_bool(ctx, "configured", true).await?;

    progress!(ctx, 920);

    e2ee::ensure_secret_key_exists(ctx).await?;
    info!(ctx, "key generation completed");

    progress!(ctx, 940);

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum AutoconfigProvider {
    Mozilla,
    Outlook,
}

#[derive(Debug, PartialEq, Eq)]
struct AutoconfigSource {
    provider: AutoconfigProvider,
    url: String,
}

impl AutoconfigSource {
    fn all(domain: &str, addr: &str) -> [Self; 5] {
        [
            AutoconfigSource {
                provider: AutoconfigProvider::Mozilla,
                url: format!(
                    "https://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                    domain, addr,
                ),
            },
            // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
            AutoconfigSource {
                provider: AutoconfigProvider::Mozilla,
                url: format!(
                    "https://{}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={}",
                    domain, addr
                ),
            },
            AutoconfigSource {
                provider: AutoconfigProvider::Outlook,
                url: format!("https://{}/autodiscover/autodiscover.xml", domain),
            },
            // Outlook uses always SSL but different domains (this comment describes the next two steps)
            AutoconfigSource {
                provider: AutoconfigProvider::Outlook,
                url: format!(
                    "https://{}{}/autodiscover/autodiscover.xml",
                    "autodiscover.", domain
                ),
            },
            // always SSL for Thunderbird's database
            AutoconfigSource {
                provider: AutoconfigProvider::Mozilla,
                url: format!("https://autoconfig.thunderbird.net/v1.1/{}", domain),
            },
        ]
    }

    async fn fetch(&self, ctx: &Context, param: &LoginParam) -> Result<LoginParam> {
        let params = match self.provider {
            AutoconfigProvider::Mozilla => moz_autoconfigure(ctx, &self.url, &param).await?,
            AutoconfigProvider::Outlook => outlk_autodiscover(ctx, &self.url, &param).await?,
        };

        Ok(params)
    }
}

/// Retrieve available autoconfigurations.
///
/// A Search configurations from the domain used in the email-address, prefer encrypted
/// B. If we have no configuration yet, search configuration in Thunderbird's centeral database
async fn get_autoconfig(
    ctx: &Context,
    param: &LoginParam,
    param_domain: &str,
    param_addr_urlencoded: &str,
) -> Option<LoginParam> {
    let sources = AutoconfigSource::all(param_domain, param_addr_urlencoded);

    let mut progress = 300;
    for source in &sources {
        let res = source.fetch(ctx, param).await;
        progress!(ctx, progress);
        progress += 10;
        if let Ok(res) = res {
            return Some(res);
        }
    }

    None
}

fn get_offline_autoconfig(context: &Context, param: &LoginParam) -> Option<LoginParam> {
    info!(
        context,
        "checking internal provider-info for offline autoconfig"
    );

    if let Some(provider) = provider::get_provider_info(&param.addr) {
        match provider.status {
            provider::Status::OK | provider::Status::PREPARATION => {
                let imap = provider.get_imap_server();
                let smtp = provider.get_smtp_server();
                // clippy complains about these is_some()/unwrap() settings,
                // however, rewriting the code to "if let" would make things less obvious,
                // esp. if we allow more combinations of servers (pop, jmap).
                // therefore, #[allow(clippy::unnecessary_unwrap)] is added above.
                if let Some(imap) = imap {
                    if let Some(smtp) = smtp {
                        let mut p = LoginParam::new();
                        p.addr = param.addr.clone();

                        p.mail_server = imap.hostname.to_string();
                        p.mail_user = imap.apply_username_pattern(param.addr.clone());
                        p.mail_port = imap.port as i32;
                        p.imap_certificate_checks = CertificateChecks::AcceptInvalidCertificates;
                        p.server_flags |= match imap.socket {
                            provider::Socket::STARTTLS => DC_LP_IMAP_SOCKET_STARTTLS,
                            provider::Socket::SSL => DC_LP_IMAP_SOCKET_SSL,
                        };

                        p.send_server = smtp.hostname.to_string();
                        p.send_user = smtp.apply_username_pattern(param.addr.clone());
                        p.send_port = smtp.port as i32;
                        p.smtp_certificate_checks = CertificateChecks::AcceptInvalidCertificates;
                        p.server_flags |= match smtp.socket {
                            provider::Socket::STARTTLS => DC_LP_SMTP_SOCKET_STARTTLS as i32,
                            provider::Socket::SSL => DC_LP_SMTP_SOCKET_SSL as i32,
                        };

                        info!(context, "offline autoconfig found: {}", p);
                        return Some(p);
                    }
                }
                info!(context, "offline autoconfig found, but no servers defined");
                return None;
            }
            provider::Status::BROKEN => {
                info!(context, "offline autoconfig found, provider is broken");
                return None;
            }
        }
    }
    info!(context, "no offline autoconfig found");
    None
}

async fn try_imap_connections(
    context: &Context,
    param: &mut LoginParam,
    was_autoconfig: bool,
    imap: &mut Imap,
) -> Result<()> {
    // manually_set_param is used to check whether a particular setting was set manually by the user.
    // If yes, we do not want to change it to avoid confusing error messages
    // (you set port 443, but the app tells you it couldn't connect on port 993).
    let manually_set_param = LoginParam::from_database(context, "").await;

    // progress 650 and 660
    if try_imap_connection(context, param, &manually_set_param, was_autoconfig, 0, imap)
        .await
        .is_ok()
    {
        return Ok(()); // we directly return here if it was autoconfig or the connection succeeded
    }

    progress!(context, 670);
    // try_imap_connection() changed the flags and port. Change them back:
    if manually_set_param.server_flags & DC_LP_IMAP_SOCKET_FLAGS == 0 {
        param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS);
        param.server_flags |= DC_LP_IMAP_SOCKET_SSL;
    }
    if manually_set_param.mail_port == 0 {
        param.mail_port = 993;
    }
    if let Some(at) = param.mail_user.find('@') {
        param.mail_user = param.mail_user.split_at(at).0.to_string();
    }
    if let Some(at) = param.send_user.find('@') {
        param.send_user = param.send_user.split_at(at).0.to_string();
    }
    // progress 680 and 690
    try_imap_connection(context, param, &manually_set_param, was_autoconfig, 1, imap).await
}

async fn try_imap_connection(
    context: &Context,
    param: &mut LoginParam,
    manually_set_param: &LoginParam,
    was_autoconfig: bool,
    variation: usize,
    imap: &mut Imap,
) -> Result<()> {
    if try_imap_one_param(context, param, imap).await.is_ok() {
        return Ok(());
    }
    if was_autoconfig {
        return Ok(());
    }

    progress!(context, 650 + variation * 30);

    if manually_set_param.server_flags & DC_LP_IMAP_SOCKET_FLAGS == 0 {
        param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS);
        param.server_flags |= DC_LP_IMAP_SOCKET_STARTTLS;

        if try_imap_one_param(context, &param, imap).await.is_ok() {
            return Ok(());
        }
    }

    progress!(context, 660 + variation * 30);

    if manually_set_param.mail_port == 0 {
        param.mail_port = 143;
        try_imap_one_param(context, param, imap).await
    } else {
        Err(format_err!("no more possible configs"))
    }
}

async fn try_imap_one_param(context: &Context, param: &LoginParam, imap: &mut Imap) -> Result<()> {
    let inf = format!(
        "imap: {}@{}:{} flags=0x{:x} certificate_checks={}",
        param.mail_user,
        param.mail_server,
        param.mail_port,
        param.server_flags,
        param.imap_certificate_checks
    );
    info!(context, "Trying: {}", inf);

    if imap.connect(context, &param).await {
        info!(context, "success: {}", inf);
        return Ok(());
    }

    bail!("Could not connect: {}", inf);
}

async fn try_smtp_connections(
    context: &Context,
    param: &mut LoginParam,
    was_autoconfig: bool,
) -> Result<()> {
    // manually_set_param is used to check whether a particular setting was set manually by the user.
    // If yes, we do not want to change it to avoid confusing error messages
    // (you set port 443, but the app tells you it couldn't connect on port 993).
    let manually_set_param = LoginParam::from_database(context, "").await;

    let mut smtp = Smtp::new();
    // try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do
    // a second try with STARTTLS-587
    if try_smtp_one_param(context, param, &mut smtp).await.is_ok() {
        return Ok(());
    }
    if was_autoconfig {
        return Ok(());
    }
    progress!(context, 850);

    if manually_set_param.server_flags & (DC_LP_SMTP_SOCKET_FLAGS as i32) == 0 {
        param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
        param.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32;
    }
    if manually_set_param.send_port == 0 {
        param.send_port = 587;
    }

    if try_smtp_one_param(context, param, &mut smtp).await.is_ok() {
        return Ok(());
    }
    progress!(context, 860);

    if manually_set_param.server_flags & (DC_LP_SMTP_SOCKET_FLAGS as i32) == 0 {
        param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
        param.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32;
    }
    if manually_set_param.send_port == 0 {
        param.send_port = 25;
    }
    try_smtp_one_param(context, param, &mut smtp).await
}

async fn try_smtp_one_param(context: &Context, param: &LoginParam, smtp: &mut Smtp) -> Result<()> {
    let inf = format!(
        "smtp: {}@{}:{} flags: 0x{:x}",
        param.send_user, param.send_server, param.send_port, param.server_flags
    );
    info!(context, "Trying: {}", inf);

    if let Err(err) = smtp.connect(context, &param).await {
        bail!("could not connect: {}", err);
    }

    info!(context, "success: {}", inf);
    smtp.disconnect().await;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid email address: {0:?}")]
    InvalidEmailAddress(String),

    #[error("XML error at position {position}")]
    InvalidXml {
        position: usize,
        #[source]
        error: quick_xml::Error,
    },

    #[error("Bad or incomplete autoconfig")]
    IncompleteAutoconfig(LoginParam),

    #[error("Failed to get URL")]
    ReadUrlError(#[from] self::read_url::Error),

    #[error("Number of redirection is exceeded")]
    RedirectionError,
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::*;
    use crate::test_utils::*;

    #[async_std::test]
    async fn test_no_panic_on_bad_credentials() {
        let t = dummy_context().await;
        t.ctx
            .set_config(Config::Addr, Some("probably@unexistant.addr"))
            .await
            .unwrap();
        t.ctx
            .set_config(Config::MailPw, Some("123456"))
            .await
            .unwrap();
        assert!(t.ctx.configure().await.is_err());
    }

    #[async_std::test]
    async fn test_get_offline_autoconfig() {
        let context = dummy_context().await.ctx;

        let mut params = LoginParam::new();
        params.addr = "someone123@example.org".to_string();
        assert!(get_offline_autoconfig(&context, &params).is_none());

        let mut params = LoginParam::new();
        params.addr = "someone123@nauta.cu".to_string();
        let found_params = get_offline_autoconfig(&context, &params).unwrap();
        assert_eq!(found_params.mail_server, "imap.nauta.cu".to_string());
        assert_eq!(found_params.send_server, "smtp.nauta.cu".to_string());
    }
}
