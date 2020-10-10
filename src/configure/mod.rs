//! Email accounts autoconfiguration process module

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

use crate::config::Config;
use crate::dc_tools::*;
use crate::imap::Imap;
use crate::login_param::{LoginParam, ServerLoginParam};
use crate::message::Message;
use crate::oauth2::*;
use crate::provider::{Protocol, Socket, UsernamePattern};
use crate::smtp::Smtp;
use crate::stock::StockMessage;
use crate::{chat, e2ee, provider};
use crate::{constants::*, job};
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

        let mut param = LoginParam::from_database(self, "").await;
        let success = configure(self, &mut param).await;
        self.set_config(Config::NotifyAboutWrongPw, None).await?;

        if let Some(provider) = provider::get_provider_info(&param.addr) {
            if let Some(config_defaults) = &provider.config_defaults {
                for def in config_defaults.iter() {
                    if !self.config_exists(def.key).await {
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
                        self.stock_string_repl_str(
                            StockMessage::ConfigurationFailed,
                            // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
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

    // Step 1: Load the parameters and check email-address and password

    if oauth2 {
        // the used oauth2 addr may differ, check this.
        // if dc_get_oauth2_addr() is not available in the oauth2 implementation, just use the given one.
        progress!(ctx, 10);
        if let Some(oauth2_addr) = dc_get_oauth2_addr(ctx, &param.addr, &param.imap.password)
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

        if let Some(servers) = get_offline_autoconfig(ctx, &param.addr) {
            param_autoconfig = Some(servers);
        } else {
            param_autoconfig =
                get_autoconfig(ctx, param, &param_domain, &param_addr_urlencoded).await;
        }
    } else {
        param_autoconfig = None;
    }

    progress!(ctx, 500);

    let servers = expand_param_vector(
        param_autoconfig.unwrap_or_else(|| {
            vec![
                ServerParams {
                    protocol: Protocol::IMAP,
                    hostname: param.imap.server.clone(),
                    port: param.imap.port,
                    socket: param.imap.security,
                    username: param.imap.user.clone(),
                },
                ServerParams {
                    protocol: Protocol::SMTP,
                    hostname: param.smtp.server.clone(),
                    port: param.smtp.port,
                    socket: param.smtp.security,
                    username: param.smtp.user.clone(),
                },
            ]
        }),
        &param.addr,
        &param_domain,
    );

    progress!(ctx, 550);

    // Spawn SMTP configuration task
    let mut smtp = Smtp::new();

    let context_smtp = ctx.clone();
    let mut smtp_param = param.smtp.clone();
    let smtp_addr = param.addr.clone();
    let smtp_servers: Vec<ServerParams> = servers
        .iter()
        .filter(|params| params.protocol == Protocol::SMTP)
        .cloned()
        .collect();

    let smtp_config_task = task::spawn(async move {
        let mut smtp_configured = false;
        let mut errors = Vec::new();
        for smtp_server in smtp_servers {
            smtp_param.user = smtp_server.username.clone();
            smtp_param.server = smtp_server.hostname.clone();
            smtp_param.port = smtp_server.port;
            smtp_param.security = smtp_server.socket;

            match try_smtp_one_param(&context_smtp, &smtp_param, &smtp_addr, oauth2, &mut smtp)
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
    let (_s, r) = async_std::sync::channel(1);
    let mut imap = Imap::new(r);

    let mut imap_configured = false;
    let imap_servers: Vec<&ServerParams> = servers
        .iter()
        .filter(|params| params.protocol == Protocol::IMAP)
        .collect();
    let imap_servers_count = imap_servers.len();
    let mut errors = Vec::new();
    for (imap_server_index, imap_server) in imap_servers.into_iter().enumerate() {
        param.imap.user = imap_server.username.clone();
        param.imap.server = imap_server.hostname.clone();
        param.imap.port = imap_server.port;
        param.imap.security = imap_server.socket;

        match try_imap_one_param(ctx, &param.imap, &param.addr, oauth2, &mut imap).await {
            Ok(_) => {
                imap_configured = true;
                break;
            }
            Err(e) => errors.push(e),
        }
        progress!(
            ctx,
            600 + (800 - 600) * (1 + imap_server_index) / imap_servers_count
        );
    }
    if !imap_configured {
        bail!(nicer_configuration_error(ctx, errors).await);
    }

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

    let create_mvbox = ctx.get_config_bool(Config::MvboxWatch).await
        || ctx.get_config_bool(Config::MvboxMove).await;

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
    ctx.sql.set_raw_config_bool(ctx, "configured", true).await?;

    progress!(ctx, 920);

    e2ee::ensure_secret_key_exists(ctx).await?;
    info!(ctx, "key generation completed");

    job::add(
        ctx,
        job::Job::new(Action::FetchExistingMsgs, 0, Params::new(), 0),
    )
    .await;

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
                    "https://autodiscover.{}/autodiscover/autodiscover.xml",
                    domain
                ),
            },
            // always SSL for Thunderbird's database
            AutoconfigSource {
                provider: AutoconfigProvider::Mozilla,
                url: format!("https://autoconfig.thunderbird.net/v1.1/{}", domain),
            },
        ]
    }

    async fn fetch(&self, ctx: &Context, param: &LoginParam) -> Result<Vec<ServerParams>> {
        let params = match self.provider {
            AutoconfigProvider::Mozilla => moz_autoconfigure(ctx, &self.url, &param).await?,
            AutoconfigProvider::Outlook => outlk_autodiscover(ctx, &self.url).await?,
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
) -> Option<Vec<ServerParams>> {
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

fn get_offline_autoconfig(context: &Context, addr: &str) -> Option<Vec<ServerParams>> {
    info!(
        context,
        "checking internal provider-info for offline autoconfig"
    );

    if let Some(provider) = provider::get_provider_info(&addr) {
        match provider.status {
            provider::Status::OK | provider::Status::PREPARATION => {
                if provider.server.is_empty() {
                    info!(context, "offline autoconfig found, but no servers defined");
                    None
                } else {
                    info!(context, "offline autoconfig found");
                    let servers = provider
                        .server
                        .iter()
                        .map(|s| ServerParams {
                            protocol: s.protocol,
                            socket: s.socket,
                            hostname: s.hostname.to_string(),
                            port: s.port,
                            username: match s.username_pattern {
                                UsernamePattern::EMAIL => addr.to_string(),
                                UsernamePattern::EMAILLOCALPART => {
                                    if let Some(at) = addr.find('@') {
                                        addr.split_at(at).0.to_string()
                                    } else {
                                        addr.to_string()
                                    }
                                }
                            },
                        })
                        .collect();
                    Some(servers)
                }
            }
            provider::Status::BROKEN => {
                info!(context, "offline autoconfig found, provider is broken");
                None
            }
        }
    } else {
        info!(context, "no offline autoconfig found");
        None
    }
}

async fn try_imap_one_param(
    context: &Context,
    param: &ServerLoginParam,
    addr: &str,
    oauth2: bool,
    imap: &mut Imap,
) -> Result<(), ConfigurationError> {
    let inf = format!(
        "imap: {}@{}:{} security={} certificate_checks={} oauth2={}",
        param.user, param.server, param.port, param.security, param.certificate_checks, oauth2
    );
    info!(context, "Trying: {}", inf);

    if let Err(err) = imap.connect(context, param, addr, oauth2).await {
        info!(context, "failure: {}", err);
        Err(ConfigurationError {
            config: inf,
            msg: err.to_string(),
        })
    } else {
        info!(context, "success: {}", inf);
        Ok(())
    }
}

async fn try_smtp_one_param(
    context: &Context,
    param: &ServerLoginParam,
    addr: &str,
    oauth2: bool,
    smtp: &mut Smtp,
) -> Result<(), ConfigurationError> {
    let inf = format!(
        "smtp: {}@{}:{} security={} certificate_checks={} oauth2={}",
        param.user, param.server, param.port, param.security, param.certificate_checks, oauth2
    );
    info!(context, "Trying: {}", inf);

    if let Err(err) = smtp.connect(context, param, addr, oauth2).await {
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
        return "".to_string();
    };

    if errors
        .iter()
        .all(|e| e.msg.to_lowercase().contains("could not resolve"))
    {
        return context
            .stock_str(StockMessage::ErrorNoNetwork)
            .await
            .to_string();
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

    #[error("XML error at position {position}")]
    InvalidXml {
        position: usize,
        #[source]
        error: quick_xml::Error,
    },

    #[error("Failed to get URL")]
    ReadUrlError(#[from] self::read_url::Error),

    #[error("Number of redirection is exceeded")]
    RedirectionError,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::config::*;
    use crate::test_utils::*;

    #[async_std::test]
    async fn test_no_panic_on_bad_credentials() {
        let t = TestContext::new().await;
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
        let context = TestContext::new().await.ctx;

        let addr = "someone123@example.org";
        assert!(get_offline_autoconfig(&context, addr).is_none());

        let addr = "someone123@nauta.cu";
        let found_params = get_offline_autoconfig(&context, addr).unwrap();
        assert_eq!(found_params.len(), 2);
        assert_eq!(found_params[0].protocol, Protocol::IMAP);
        assert_eq!(found_params[0].hostname, "imap.nauta.cu".to_string());
        assert_eq!(found_params[1].protocol, Protocol::SMTP);
        assert_eq!(found_params[1].hostname, "smtp.nauta.cu".to_string());
    }
}
