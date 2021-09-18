//! Email accounts autoconfiguration process module.

mod auto_mozilla;
mod auto_outlook;
mod read_url;
mod server_params;

use anyhow::{bail, ensure, Context as _, Result};
use async_std::prelude::*;
use async_std::task;
use itertools::Itertools;
use job::Action;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::dc_tools::EmailAddress;
use crate::imap::Imap;
use crate::login_param::Socks5Config;
use crate::login_param::{LoginParam, ServerLoginParam};
use crate::message::Message;
use crate::oauth2::dc_get_oauth2_addr;
use crate::provider::{Protocol, Socket, UsernamePattern};
use crate::smtp::Smtp;
use crate::stock_str;
use crate::{chat, e2ee, provider};
use crate::{config::Config, dc_tools::time};
use crate::{
    constants::{Viewtype, DC_LP_AUTH_FLAGS, DC_LP_AUTH_NORMAL, DC_LP_AUTH_OAUTH2},
    job,
};
use crate::{context::Context, param::Params};

use auto_mozilla::moz_autoconfigure;
use auto_outlook::outlk_autodiscover;
use server_params::{expand_param_vector, ServerParams};

macro_rules! progress {
    ($context:tt, $progress:expr, $comment:expr) => {
        assert!(
            $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.emit_event($crate::events::EventType::ConfigureProgress {
            progress: $progress,
            comment: $comment,
        });
    };
    ($context:tt, $progress:expr) => {
        progress!($context, $progress, None);
    };
}

impl Context {
    /// Checks if the context is already configured.
    pub async fn is_configured(&self) -> Result<bool> {
        self.sql
            .get_raw_config_bool("configured")
            .await
            .map_err(Into::into)
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

        let mut param = LoginParam::from_database(self, "").await?;
        let success = configure(self, &mut param).await;
        self.set_config(Config::NotifyAboutWrongPw, None).await?;

        if let Some(provider) = param.provider {
            if let Some(config_defaults) = &provider.config_defaults {
                for def in config_defaults.iter() {
                    if !self.config_exists(def.key).await? {
                        info!(self, "apply config_defaults {}={}", def.key, def.value);
                        self.set_config(def.key, Some(def.value)).await?;
                    } else {
                        info!(
                            self,
                            "skip already set config_defaults {}={}", def.key, def.value
                        );
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
                self.set_config(Config::NotifyAboutWrongPw, Some("1"))
                    .await?;
                progress!(self, 1000);
                Ok(())
            }
            Err(err) => {
                progress!(
                    self,
                    0,
                    Some(
                        stock_str::configuration_failed(
                            self,
                            // We are using Anyhow's .context() and to show the
                            // inner error, too, we need the {:#}:
                            format!("{:#}", err),
                        )
                        .await
                    )
                );
                Err(err)
            }
        }
    }
}

async fn configure(ctx: &Context, param: &mut LoginParam) -> Result<()> {
    progress!(ctx, 1);

    // Check basic settings.
    ensure!(!param.addr.is_empty(), "Please enter an email address.");

    // Only check for IMAP password, SMTP password is an "advanced" setting.
    ensure!(!param.imap.password.is_empty(), "Please enter a password.");
    if param.smtp.password.is_empty() {
        param.smtp.password = param.imap.password.clone()
    }

    // Normalize authentication flags.
    let oauth2 = match param.server_flags & DC_LP_AUTH_FLAGS as i32 {
        DC_LP_AUTH_OAUTH2 => true,
        DC_LP_AUTH_NORMAL => false,
        _ => false,
    };
    param.server_flags &= !(DC_LP_AUTH_FLAGS as i32);
    param.server_flags |= if oauth2 {
        DC_LP_AUTH_OAUTH2 as i32
    } else {
        DC_LP_AUTH_NORMAL as i32
    };

    let socks5_config = param.socks5_config.clone();
    let socks5_enabled = socks5_config.is_some();

    let ctx2 = ctx.clone();
    let update_device_chats_handle = task::spawn(async move { ctx2.update_device_chats().await });

    // Step 1: Load the parameters and check email-address and password

    // Do oauth2 only if socks5 is disabled. As soon as we have a http library that can do
    // socks5 requests, this can work with socks5 too
    if oauth2 && !socks5_enabled {
        // the used oauth2 addr may differ, check this.
        // if dc_get_oauth2_addr() is not available in the oauth2 implementation, just use the given one.
        progress!(ctx, 10);
        if let Some(oauth2_addr) = dc_get_oauth2_addr(ctx, &param.addr, &param.imap.password)
            .await?
            .and_then(|e| e.parse().ok())
        {
            info!(ctx, "Authorized address is {}", oauth2_addr);
            param.addr = oauth2_addr;
            ctx.sql
                .set_raw_config("addr", Some(param.addr.as_str()))
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

    let param_autoconfig;
    if param.imap.server.is_empty()
        && param.imap.port == 0
        && param.imap.security == Socket::Automatic
        && param.imap.user.is_empty()
        && param.smtp.server.is_empty()
        && param.smtp.port == 0
        && param.smtp.security == Socket::Automatic
        && param.smtp.user.is_empty()
    {
        // no advanced parameters entered by the user: query provider-database or do Autoconfig

        info!(
            ctx,
            "checking internal provider-info for offline autoconfig"
        );

        if let Some(provider) = provider::get_provider_info(&param_domain, socks5_enabled).await {
            param.provider = Some(provider);
            match provider.status {
                provider::Status::Ok | provider::Status::Preparation => {
                    if provider.server.is_empty() {
                        info!(ctx, "offline autoconfig found, but no servers defined");
                        param_autoconfig = None;
                    } else {
                        info!(ctx, "offline autoconfig found");
                        let servers = provider
                            .server
                            .iter()
                            .map(|s| ServerParams {
                                protocol: s.protocol,
                                socket: s.socket,
                                hostname: s.hostname.to_string(),
                                port: s.port,
                                username: match s.username_pattern {
                                    UsernamePattern::Email => param.addr.to_string(),
                                    UsernamePattern::Emaillocalpart => {
                                        if let Some(at) = param.addr.find('@') {
                                            param.addr.split_at(at).0.to_string()
                                        } else {
                                            param.addr.to_string()
                                        }
                                    }
                                },
                            })
                            .collect();

                        param_autoconfig = Some(servers)
                    }
                }
                provider::Status::Broken => {
                    info!(ctx, "offline autoconfig found, provider is broken");
                    param_autoconfig = None;
                }
            }
        } else {
            // Try receiving autoconfig
            info!(ctx, "no offline autoconfig found");
            param_autoconfig = if socks5_enabled {
                // Currently we can't do http requests through socks5, to not leak
                // the ip, just don't do online autoconfig
                info!(ctx, "socks5 enabled, skipping autoconfig");
                None
            } else {
                get_autoconfig(ctx, param, &param_domain, &param_addr_urlencoded).await
            }
        }
    } else {
        param_autoconfig = None;
    }

    progress!(ctx, 500);

    let mut servers = param_autoconfig.unwrap_or_default();
    if !servers
        .iter()
        .any(|server| server.protocol == Protocol::Imap)
    {
        servers.push(ServerParams {
            protocol: Protocol::Imap,
            hostname: param.imap.server.clone(),
            port: param.imap.port,
            socket: param.imap.security,
            username: param.imap.user.clone(),
        })
    }
    if !servers
        .iter()
        .any(|server| server.protocol == Protocol::Smtp)
    {
        servers.push(ServerParams {
            protocol: Protocol::Smtp,
            hostname: param.smtp.server.clone(),
            port: param.smtp.port,
            socket: param.smtp.security,
            username: param.smtp.user.clone(),
        })
    }
    let servers = expand_param_vector(servers, &param.addr, &param_domain);

    progress!(ctx, 550);

    // Spawn SMTP configuration task
    let mut smtp = Smtp::new();

    let context_smtp = ctx.clone();
    let mut smtp_param = param.smtp.clone();
    let smtp_addr = param.addr.clone();
    let smtp_servers: Vec<ServerParams> = servers
        .iter()
        .filter(|params| params.protocol == Protocol::Smtp)
        .cloned()
        .collect();
    let provider_strict_tls = param
        .provider
        .map_or(socks5_config.is_some(), |provider| provider.strict_tls);

    let smtp_config_task = task::spawn(async move {
        let mut smtp_configured = false;
        let mut errors = Vec::new();
        for smtp_server in smtp_servers {
            smtp_param.user = smtp_server.username.clone();
            smtp_param.server = smtp_server.hostname.clone();
            smtp_param.port = smtp_server.port;
            smtp_param.security = smtp_server.socket;

            match try_smtp_one_param(
                &context_smtp,
                &smtp_param,
                &socks5_config,
                &smtp_addr,
                oauth2,
                provider_strict_tls,
                &mut smtp,
            )
            .await
            {
                Ok(_) => {
                    smtp_configured = true;
                    break;
                }
                Err(e) => errors.push(e),
            }
        }

        if smtp_configured {
            Ok(smtp_param)
        } else {
            Err(errors)
        }
    });

    progress!(ctx, 600);

    // Configure IMAP

    let mut imap: Option<Imap> = None;
    let imap_servers: Vec<&ServerParams> = servers
        .iter()
        .filter(|params| params.protocol == Protocol::Imap)
        .collect();
    let imap_servers_count = imap_servers.len();
    let mut errors = Vec::new();
    for (imap_server_index, imap_server) in imap_servers.into_iter().enumerate() {
        param.imap.user = imap_server.username.clone();
        param.imap.server = imap_server.hostname.clone();
        param.imap.port = imap_server.port;
        param.imap.security = imap_server.socket;

        match try_imap_one_param(
            ctx,
            &param.imap,
            &param.socks5_config,
            &param.addr,
            oauth2,
            provider_strict_tls,
        )
        .await
        {
            Ok(configured_imap) => {
                imap = Some(configured_imap);
                break;
            }
            Err(e) => errors.push(e),
        }
        progress!(
            ctx,
            600 + (800 - 600) * (1 + imap_server_index) / imap_servers_count
        );
    }
    let mut imap = match imap {
        Some(imap) => imap,
        None => bail!(nicer_configuration_error(ctx, errors).await),
    };

    progress!(ctx, 850);

    // Wait for SMTP configuration
    match smtp_config_task.await {
        Ok(smtp_param) => {
            param.smtp = smtp_param;
        }
        Err(errors) => {
            bail!(nicer_configuration_error(ctx, errors).await);
        }
    }

    progress!(ctx, 900);

    let create_mvbox = ctx.get_config_bool(Config::MvboxWatch).await?
        || ctx.get_config_bool(Config::MvboxMove).await?;

    imap.configure_folders(ctx, create_mvbox).await?;

    imap.select_with_uidvalidity(ctx, "INBOX")
        .await
        .context("could not read INBOX status")?;

    drop(imap);

    progress!(ctx, 910);
    // configuration success - write back the configured parameters with the
    // "configured_" prefix; also write the "configured"-flag */
    // the trailing underscore is correct
    param.save_to_database(ctx, "configured_").await?;
    ctx.sql.set_raw_config_bool("configured", true).await?;
    ctx.set_config(Config::ConfiguredTimestamp, Some(&time().to_string()))
        .await?;

    progress!(ctx, 920);

    e2ee::ensure_secret_key_exists(ctx).await?;
    info!(ctx, "key generation completed");

    job::add(
        ctx,
        job::Job::new(Action::FetchExistingMsgs, 0, Params::new(), 0),
    )
    .await?;

    progress!(ctx, 940);
    update_device_chats_handle.await?;

    Ok(())
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
) -> Option<Vec<ServerParams>> {
    if let Ok(res) = moz_autoconfigure(
        ctx,
        &format!(
            "https://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
            param_domain, param_addr_urlencoded
        ),
        param,
    )
    .await
    {
        return Some(res);
    }
    progress!(ctx, 300);

    if let Ok(res) = moz_autoconfigure(
        ctx,
        // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see <https://releases.mozilla.org/pub/thunderbird/>,  which makes some sense
        &format!(
            "https://{}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={}",
            &param_domain, &param_addr_urlencoded
        ),
        param,
    )
    .await
    {
        return Some(res);
    }
    progress!(ctx, 310);

    // Outlook uses always SSL but different domains (this comment describes the next two steps)
    if let Ok(res) = outlk_autodiscover(
        ctx,
        format!("https://{}/autodiscover/autodiscover.xml", &param_domain),
    )
    .await
    {
        return Some(res);
    }
    progress!(ctx, 320);

    if let Ok(res) = outlk_autodiscover(
        ctx,
        format!(
            "https://autodiscover.{}/autodiscover/autodiscover.xml",
            &param_domain
        ),
    )
    .await
    {
        return Some(res);
    }
    progress!(ctx, 330);

    // always SSL for Thunderbird's database
    if let Ok(res) = moz_autoconfigure(
        ctx,
        &format!("https://autoconfig.thunderbird.net/v1.1/{}", &param_domain),
        param,
    )
    .await
    {
        return Some(res);
    }

    None
}

async fn try_imap_one_param(
    context: &Context,
    param: &ServerLoginParam,
    socks5_config: &Option<Socks5Config>,
    addr: &str,
    oauth2: bool,
    provider_strict_tls: bool,
) -> Result<Imap, ConfigurationError> {
    let inf = format!(
        "imap: {}@{}:{} security={} certificate_checks={} oauth2={}",
        param.user, param.server, param.port, param.security, param.certificate_checks, oauth2
    );
    info!(context, "Trying: {}", inf);

    let (_s, r) = async_std::channel::bounded(1);

    let mut imap = match Imap::new(
        param,
        socks5_config.clone(),
        addr,
        oauth2,
        provider_strict_tls,
        r,
    )
    .await
    {
        Err(err) => {
            info!(context, "failure: {}", err);
            return Err(ConfigurationError {
                config: inf,
                msg: err.to_string(),
            });
        }
        Ok(imap) => imap,
    };

    match imap.connect(context).await {
        Err(err) => {
            info!(context, "failure: {}", err);
            Err(ConfigurationError {
                config: inf,
                msg: err.to_string(),
            })
        }
        Ok(()) => {
            info!(context, "success: {}", inf);
            Ok(imap)
        }
    }
}

async fn try_smtp_one_param(
    context: &Context,
    param: &ServerLoginParam,
    socks5_config: &Option<Socks5Config>,
    addr: &str,
    oauth2: bool,
    provider_strict_tls: bool,
    smtp: &mut Smtp,
) -> Result<(), ConfigurationError> {
    let inf = format!(
        "smtp: {}@{}:{} security={} certificate_checks={} oauth2={} socks5_config={}",
        param.user,
        param.server,
        param.port,
        param.security,
        param.certificate_checks,
        oauth2,
        if let Some(socks5_config) = socks5_config {
            socks5_config.to_string()
        } else {
            "None".to_string()
        }
    );
    info!(context, "Trying: {}", inf);

    if let Err(err) = smtp
        .connect(
            context,
            param,
            socks5_config,
            addr,
            oauth2,
            provider_strict_tls,
        )
        .await
    {
        info!(context, "failure: {}", err);
        Err(ConfigurationError {
            config: inf,
            msg: err.to_string(),
        })
    } else {
        info!(context, "success: {}", inf);
        smtp.disconnect().await;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Trying {config}…\nError: {msg}")]
pub struct ConfigurationError {
    config: String,
    msg: String,
}

async fn nicer_configuration_error(context: &Context, errors: Vec<ConfigurationError>) -> String {
    let first_err = if let Some(f) = errors.first() {
        f
    } else {
        // This means configuration failed but no errors have been captured. This should never
        // happen, but if it does, the user will see classic "Error: no error".
        return "no error".to_string();
    };

    if errors
        .iter()
        .all(|e| e.msg.to_lowercase().contains("could not resolve"))
    {
        return stock_str::error_no_network(context).await;
    }

    if errors.iter().all(|e| e.msg == first_err.msg) {
        return first_err.msg.to_string();
    }

    errors.iter().map(|e| e.to_string()).join("\n\n")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid email address: {0:?}")]
    InvalidEmailAddress(String),

    #[error("XML error at position {position}: {error}")]
    InvalidXml {
        position: usize,
        #[source]
        error: quick_xml::Error,
    },

    #[error("Failed to get URL: {0}")]
    ReadUrl(#[from] self::read_url::Error),

    #[error("Number of redirection is exceeded")]
    Redirection,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use crate::config::Config;
    use crate::test_utils::TestContext;

    #[async_std::test]
    async fn test_no_panic_on_bad_credentials() {
        let t = TestContext::new().await;
        t.set_config(Config::Addr, Some("probably@unexistant.addr"))
            .await
            .unwrap();
        t.set_config(Config::MailPw, Some("123456")).await.unwrap();
        assert!(t.configure().await.is_err());
    }
}
