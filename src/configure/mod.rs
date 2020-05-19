//! Email accounts autoconfiguration process module

mod auto_mozilla;
mod auto_outlook;
mod read_url;

use anyhow::{bail, ensure, Result};
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
        $context.call_cb($crate::events::Event::ConfigureProgress($progress));
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
        let mut success = false;
        let mut param_autoconfig: Option<LoginParam> = None;

        info!(self, "Configure ...");

        // Variables that are shared between steps:
        let mut param = LoginParam::from_database(self, "").await;
        // need all vars here to be mutable because rust thinks the same step could be called multiple times
        // and also initialize, because otherwise rust thinks it's used while unitilized, even if thats not the case as the loop goes only forward
        let mut param_domain = "undefined.undefined".to_owned();
        let mut param_addr_urlencoded: String =
            "Internal Error: this value should never be used".to_owned();
        let mut keep_flags = 0;

        let mut step_counter: u8 = 0;
        let (_s, r) = async_std::sync::channel(1);
        let mut imap = Imap::new(r);
        let mut is_imap_connected = false;
        let was_configured_before = self.is_configured().await;

        while !self.shall_stop_ongoing().await {
            step_counter += 1;

            match exec_step(
                self,
                &mut imap,
                &mut is_imap_connected,
                &mut param,
                &mut param_domain,
                &mut param_autoconfig,
                &mut param_addr_urlencoded,
                &mut keep_flags,
                &mut step_counter,
            )
            .await
            {
                Ok(step) => {
                    success = true;
                    match step {
                        Step::Continue => {}
                        Step::Done => break,
                    }
                }
                Err(err) => {
                    error!(self, "{}", err);
                    success = false;
                    break;
                }
            }
        }

        if is_imap_connected {
            imap.disconnect(self).await;
        }

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

        // remember the entered parameters on success
        // and restore to last-entered on failure.
        // this way, the parameters visible to the ui are always in-sync with the current configuration.
        if success {
            LoginParam::from_database(self, "")
                .await
                .save_to_database(self, "configured_raw_")
                .await
                .ok();

            progress!(self, 1000);
            Ok(())
        } else {
            LoginParam::from_database(self, "configured_raw_")
                .await
                .save_to_database(self, "")
                .await
                .ok();

            progress!(self, 0);
            bail!("Configure failed")
        }
    }
}

async fn exec_step(
    ctx: &Context,
    imap: &mut Imap,
    is_imap_connected: &mut bool,
    param: &mut LoginParam,
    param_domain: &mut String,
    param_autoconfig: &mut Option<LoginParam>,
    param_addr_urlencoded: &mut String,
    keep_flags: &mut i32,
    step_counter: &mut u8,
) -> Result<Step> {
    const STEP_12_USE_AUTOCONFIG: u8 = 12;
    const STEP_13_AFTER_AUTOCONFIG: u8 = 13;

    match *step_counter {
        // Read login parameters from the database
        1 => {
            progress!(ctx, 1);
            ensure!(!param.addr.is_empty(), "Please enter an email address.");
        }
        // Step 1: Load the parameters and check email-address and password
        2 => {
            if 0 != param.server_flags & DC_LP_AUTH_OAUTH2 {
                // the used oauth2 addr may differ, check this.
                // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                // just use the given one.
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
        }
        3 => {
            if let Ok(parsed) = param.addr.parse() {
                let parsed: EmailAddress = parsed;
                *param_domain = parsed.domain;
                *param_addr_urlencoded =
                    utf8_percent_encode(&param.addr, NON_ALPHANUMERIC).to_string();
            } else {
                bail!("Bad email-address.");
            }
        }
        // Step 2: Autoconfig
        4 => {
            progress!(ctx, 200);

            if param.mail_server.is_empty()
                && param.mail_port == 0
            /* && param.mail_user.is_empty() -- the user can enter a loginname which is used by autoconfig then */
                && param.send_server.is_empty()
                && param.send_port == 0
                && param.send_user.is_empty()
            /* && param.send_pw.is_empty() -- the password cannot be auto-configured and is no criterion for autoconfig or not */
                && (param.server_flags & !DC_LP_AUTH_OAUTH2) == 0
            {
                // no advanced parameters entered by the user: query provider-database or do Autoconfig
                *keep_flags = param.server_flags & DC_LP_AUTH_OAUTH2;
                if let Some(new_param) = get_offline_autoconfig(ctx, &param) {
                    // got parameters from our provider-database, skip Autoconfig, preserve the OAuth2 setting
                    *param_autoconfig = Some(new_param);
                    *step_counter = STEP_12_USE_AUTOCONFIG - 1; // minus one as step_counter is increased on next loop
                }
            } else {
                // advanced parameters entered by the user: skip Autoconfig
                *step_counter = STEP_13_AFTER_AUTOCONFIG - 1; // minus one as step_counter is increased on next loop
            }
        }
        /* A.  Search configurations from the domain used in the email-address, prefer encrypted */
        5 => {
            if param_autoconfig.is_none() {
                let url = format!(
                    "https://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                    param_domain, param_addr_urlencoded
                );
                *param_autoconfig = moz_autoconfigure(ctx, &url, &param).ok();
            }
        }
        6 => {
            progress!(ctx, 300);
            if param_autoconfig.is_none() {
                // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
                let url = format!(
                    "https://{}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={}",
                    param_domain, param_addr_urlencoded
                );
                *param_autoconfig = moz_autoconfigure(ctx, &url, &param).ok();
            }
        }
        /* Outlook section start ------------- */
        /* Outlook uses always SSL but different domains (this comment describes the next two steps) */
        7 => {
            progress!(ctx, 310);
            if param_autoconfig.is_none() {
                let url = format!("https://{}/autodiscover/autodiscover.xml", param_domain);
                *param_autoconfig = outlk_autodiscover(ctx, &url, &param).ok();
            }
        }
        8 => {
            progress!(ctx, 320);
            if param_autoconfig.is_none() {
                let url = format!(
                    "https://{}{}/autodiscover/autodiscover.xml",
                    "autodiscover.", param_domain
                );
                *param_autoconfig = outlk_autodiscover(ctx, &url, &param).ok();
            }
        }
        /* ----------- Outlook section end */
        9 => {
            progress!(ctx, 330);
            if param_autoconfig.is_none() {
                let url = format!(
                    "http://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                    param_domain, param_addr_urlencoded
                );
                *param_autoconfig = moz_autoconfigure(ctx, &url, &param).ok();
            }
        }
        10 => {
            progress!(ctx, 340);
            if param_autoconfig.is_none() {
                // do not transfer the email-address unencrypted
                let url = format!(
                    "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
                    param_domain
                );
                *param_autoconfig = moz_autoconfigure(ctx, &url, &param).ok();
            }
        }
        /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
        11 => {
            progress!(ctx, 350);
            if param_autoconfig.is_none() {
                /* always SSL for Thunderbird's database */
                let url = format!("https://autoconfig.thunderbird.net/v1.1/{}", param_domain);
                *param_autoconfig = moz_autoconfigure(ctx, &url, &param).ok();
            }
        }
        /* C.  Do we have any autoconfig result?
           If you change the match-number here, also update STEP_12_COPY_AUTOCONFIG above
        */
        STEP_12_USE_AUTOCONFIG => {
            progress!(ctx, 500);
            if let Some(ref cfg) = param_autoconfig {
                info!(ctx, "Got autoconfig: {}", &cfg);
                if !cfg.mail_user.is_empty() {
                    param.mail_user = cfg.mail_user.clone();
                }
                param.mail_server = cfg.mail_server.clone(); /* all other values are always NULL when entering autoconfig */
                param.mail_port = cfg.mail_port;
                param.send_server = cfg.send_server.clone();
                param.send_port = cfg.send_port;
                param.send_user = cfg.send_user.clone();
                param.server_flags = cfg.server_flags;
                /* although param_autoconfig's data are no longer needed from,
                it is used to later to prevent trying variations of port/server/logins */
            }
            param.server_flags |= *keep_flags;
        }
        // Step 3: Fill missing fields with defaults
        // If you change the match-number here, also update STEP_13_AFTER_AUTOCONFIG above
        STEP_13_AFTER_AUTOCONFIG => {
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
            /* do we have a complete configuration? */
            if param.mail_server.is_empty()
                || param.mail_port == 0
                || param.mail_user.is_empty()
                || param.mail_pw.is_empty()
                || param.send_server.is_empty()
                || param.send_port == 0
                || param.send_user.is_empty()
                || param.send_pw.is_empty()
                || param.server_flags == 0
            {
                bail!("Account settings incomplete.");
            }
        }
        14 => {
            progress!(ctx, 600);
            /* try to connect to IMAP - if we did not got an autoconfig,
            do some further tries with different settings and username variations */
            *is_imap_connected =
                try_imap_connections(ctx, param, param_autoconfig.is_some(), imap).await?;
        }
        15 => {
            progress!(ctx, 800);
            try_smtp_connections(ctx, param, param_autoconfig.is_some()).await?;
        }
        16 => {
            progress!(ctx, 900);

            let create_mvbox = ctx.get_config_bool(Config::MvboxWatch).await
                || ctx.get_config_bool(Config::MvboxMove).await;

            if let Err(err) = imap.configure_folders(ctx, create_mvbox).await {
                bail!("configuring folders failed: {:?}", err);
            }

            if let Err(err) = imap.select_with_uidvalidity(ctx, "INBOX").await {
                bail!("could not read INBOX status: {:?}", err);
            }
        }
        17 => {
            progress!(ctx, 910);
            // configuration success - write back the configured parameters with the
            // "configured_" prefix; also write the "configured"-flag */
            // the trailing underscore is correct
            param.save_to_database(ctx, "configured_").await?;
            ctx.sql.set_raw_config_bool(ctx, "configured", true).await?;
        }
        18 => {
            progress!(ctx, 920);
            // we generate the keypair just now - we could also postpone this until the first message is sent, however,
            // this may result in a unexpected and annoying delay when the user sends his very first message
            // (~30 seconds on a Moto G4 play) and might looks as if message sending is always that slow.
            e2ee::ensure_secret_key_exists(ctx).await?;
            info!(ctx, "key generation completed");
            progress!(ctx, 940);
            return Ok(Step::Done);
        }
        _ => {
            bail!("Internal error: step counter out of bound");
        }
    }

    Ok(Step::Continue)
}

#[derive(Debug)]
enum Step {
    Done,
    Continue,
}

#[allow(clippy::unnecessary_unwrap)]
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
                if imap.is_some() && smtp.is_some() {
                    let imap = imap.unwrap();
                    let smtp = smtp.unwrap();

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
                } else {
                    info!(context, "offline autoconfig found, but no servers defined");
                    return None;
                }
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
    mut param: &mut LoginParam,
    was_autoconfig: bool,
    imap: &mut Imap,
) -> Result<bool> {
    // progress 650 and 660
    if let Ok(val) = try_imap_connection(context, &mut param, was_autoconfig, 0, imap).await {
        return Ok(val);
    }
    progress!(context, 670);
    param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS);
    param.server_flags |= DC_LP_IMAP_SOCKET_SSL;
    param.mail_port = 993;

    if let Some(at) = param.mail_user.find('@') {
        param.mail_user = param.mail_user.split_at(at).0.to_string();
    }
    if let Some(at) = param.send_user.find('@') {
        param.send_user = param.send_user.split_at(at).0.to_string();
    }
    // progress 680 and 690
    try_imap_connection(context, &mut param, was_autoconfig, 1, imap).await
}

async fn try_imap_connection(
    context: &Context,
    param: &mut LoginParam,
    was_autoconfig: bool,
    variation: usize,
    imap: &mut Imap,
) -> Result<bool> {
    if try_imap_one_param(context, &param, imap).await.is_ok() {
        return Ok(true);
    }
    if was_autoconfig {
        return Ok(false);
    }
    progress!(context, 650 + variation * 30);
    param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS);
    param.server_flags |= DC_LP_IMAP_SOCKET_STARTTLS;
    if try_imap_one_param(context, &param, imap).await.is_ok() {
        return Ok(true);
    }

    progress!(context, 660 + variation * 30);
    param.mail_port = 143;

    try_imap_one_param(context, &param, imap).await?;

    Ok(true)
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

    if context.shall_stop_ongoing().await {
        bail!("Interrupted");
    }

    bail!("Could not connect: {}", inf);
}

async fn try_smtp_connections(
    context: &Context,
    mut param: &mut LoginParam,
    was_autoconfig: bool,
) -> Result<()> {
    let mut smtp = Smtp::new();
    /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
    if try_smtp_one_param(context, &param, &mut smtp).await.is_ok() {
        return Ok(());
    }
    if was_autoconfig {
        return Ok(());
    }
    progress!(context, 850);
    param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
    param.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32;
    param.send_port = 587;

    if try_smtp_one_param(context, &param, &mut smtp).await.is_ok() {
        return Ok(());
    }
    progress!(context, 860);
    param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
    param.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32;
    param.send_port = 25;
    try_smtp_one_param(context, &param, &mut smtp).await?;

    Ok(())
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
