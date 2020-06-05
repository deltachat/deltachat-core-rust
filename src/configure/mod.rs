//! Email accounts autoconfiguration process module

mod auto_mozilla;
mod auto_outlook;
mod read_url;

use anyhow::{ensure, format_err, Context as _, Result};
use async_std::prelude::*;
use futures::select;
use futures::stream::{FuturesUnordered, StreamExt};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::imap::Imap;
use crate::login_param::{
    AuthScheme, CertificateChecks, LoginParam, ServerParam, ServerSecurity, Service, IDX_IMAP,
    IDX_SMTP,
};
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

//ports [plain_socket, ssl, starttls]
static IMAP_DEFAULT_PORTS: [i32; 3] = [143, 993, 143];
static SMTP_DEFAULT_PORTS: [i32; 3] = [25, 465, 587];

macro_rules! server_options {
    ($def_ports:expr) => {
        vec![
            ServerOption {
                security: ServerSecurity::PlainSocket,
                port: $def_ports[0],
            },
            ServerOption {
                security: ServerSecurity::Ssl,
                port: $def_ports[1],
            },
            ServerOption {
                security: ServerSecurity::Starttls,
                port: $def_ports[2],
            },
        ]
    };
    ($security:expr, $port:expr) => {
        vec![ServerOption {
            security: $security,
            port: $port,
        }]
    };
}

fn port2opt(port: i32, service: Service) -> Vec<ServerOption> {
    let def_ports = match service {
        Service::Imap => IMAP_DEFAULT_PORTS,
        Service::Smtp => SMTP_DEFAULT_PORTS,
    };

    let mut res = vec![];
    for sec in [
        ServerSecurity::PlainSocket,
        ServerSecurity::Starttls,
        ServerSecurity::Ssl,
    ].iter() {
        if port == def_ports[*sec as usize] {
            res.push(ServerOption {security: *sec, port: port});
        }
    }
    if res.is_empty() {
        return server_options!([port, port, port]);
    }
    res
}

fn select_server_options(
    port: i32,
    security: Option<ServerSecurity>,
    service: Service,
) -> Vec<ServerOption> {
    let def_ports = match service {
        Service::Imap => IMAP_DEFAULT_PORTS,
        Service::Smtp => SMTP_DEFAULT_PORTS,
    };
    if port == 0 && security.is_none() {
        // Nothing is specified, try all default options.
        let res = server_options!(def_ports);
        res
    } else if security.is_none() {
        // Only port is specified, select security options.
        port2opt(port, service)
    } else if 0 == port {
        let sec = security.unwrap();
        server_options!(sec, def_ports[sec as usize])
    } else {
        server_options!(security.unwrap(), port)
    }
}

pub(crate) struct ServerOption {
    pub security: ServerSecurity,
    pub port: i32,
}

enum TryResult {
    Success(ServerParam),
    Failure(ServerParam),
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

fn choose_starting_port_socket_security(
    param: &mut ServerParam,
    port_ssl: i32,
    port_starttls: i32,
    port_plain: i32,
) {
    if param.security.is_none() {
        if param.port == 0 {
            param.port = port_ssl;
        }
        if param.port == port_starttls {
            param.security = Some(ServerSecurity::Starttls);
        } else if param.port == port_plain {
            param.security = Some(ServerSecurity::PlainSocket);
        } else {
            param.security = Some(ServerSecurity::Ssl);
        }
    } else if param.port == 0 {
        param.port = match param.security.unwrap() {
            ServerSecurity::PlainSocket => port_plain,
            ServerSecurity::Ssl => port_ssl,
            ServerSecurity::Starttls => port_starttls,
        };
    }
}

async fn configure(ctx: &Context, param: &mut LoginParam) -> Result<()> {
    let mut param_autoconfig: Option<LoginParam> = None;
    let mut keep_oauth2 = false;

    // Read login parameters from the database
    progress!(ctx, 1);
    ensure!(!param.addr.is_empty(), "Please enter an email address.");

    let mut user_options: Vec<String> = Vec::new();
    let imap_options: Vec<ServerOption>;
    let smtp_options: Vec<ServerOption>;

    // Step 1: Load the parameters and check email-address and password

    if param.auth_scheme.is_oauth2() {
        // the used oauth2 addr may differ, check this.
        // if dc_get_oauth2_addr() is not available in the oauth2 implementation, just use the given one.
        progress!(ctx, 10);
        if let Some(oauth2_addr) =
            dc_get_oauth2_addr(ctx, &param.addr, &param.srv_params[IDX_IMAP].pw)
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

    // param.srv_params[IDX_IMAP].user.is_empty() -- the user can enter a loginname which is used by autoconfig then
    // param.srv_params[IDX_SMTP].pw.is_empty()   -- the password cannot be auto-configured and is no criterion for
    //                               autoconfig or not
    if param.srv_params[IDX_IMAP].hostname.is_empty()
        && param.srv_params[IDX_IMAP].port == 0
        && param.srv_params[IDX_SMTP].hostname.is_empty()
        && param.srv_params[IDX_SMTP].port == 0
        && param.srv_params[IDX_SMTP].user.is_empty()
        && param.auth_scheme.is_oauth2()
    {
        // no advanced parameters entered by the user: query provider-database or do Autoconfig
        keep_oauth2 = param.auth_scheme.is_oauth2();
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
        if !cfg.srv_params[IDX_IMAP].user.is_empty() {
            param.srv_params[IDX_IMAP].user = cfg.srv_params[IDX_IMAP].user.clone();
        }
        // all other values are always NULL when entering autoconfig
        param.srv_params[IDX_IMAP].hostname = cfg.srv_params[IDX_IMAP].hostname.clone();
        param.srv_params[IDX_IMAP].port = cfg.srv_params[IDX_IMAP].port;
        param.srv_params[IDX_IMAP].security = cfg.srv_params[IDX_IMAP].security;
        param.srv_params[IDX_SMTP].hostname = cfg.srv_params[IDX_SMTP].hostname.clone();
        param.srv_params[IDX_SMTP].port = cfg.srv_params[IDX_SMTP].port;
        param.srv_params[IDX_SMTP].user = cfg.srv_params[IDX_SMTP].user.clone();
        param.srv_params[IDX_SMTP].security = cfg.srv_params[IDX_SMTP].security;
        param.auth_scheme = cfg.auth_scheme;
    }
    if keep_oauth2 {
        param.auth_scheme = AuthScheme::Oauth2;
    }

    // Step 3: Fill missing fields with defaults
    if param.srv_params[IDX_IMAP].hostname.is_empty() {
        param.srv_params[IDX_IMAP].hostname = format!("imap.{}", param_domain,)
    }

    imap_options = select_server_options(
        param.srv_params[IDX_IMAP].port,
        param.srv_params[IDX_IMAP].security,
        Service::Imap,
    );
    if param.srv_params[IDX_IMAP].user.is_empty() || param.srv_params[IDX_SMTP].user.is_empty() {
        user_options.push(param.addr.clone());

        if let Some(at) = param.addr.find('@') {
            user_options.push(param.addr.split_at(at).0.to_string());
        }
    }
    if param.srv_params[IDX_SMTP].hostname.is_empty()
        && !param.srv_params[IDX_IMAP].hostname.is_empty()
    {
        param.srv_params[IDX_SMTP].hostname = param.srv_params[IDX_IMAP].hostname.clone();
        if param.srv_params[IDX_SMTP].hostname.starts_with("imap.") {
            param.srv_params[IDX_SMTP].hostname = param.srv_params[IDX_SMTP]
                .hostname
                .replacen("imap", "smtp", 1);
        }
    }
    smtp_options = select_server_options(
        param.srv_params[IDX_SMTP].port,
        param.srv_params[IDX_SMTP].security,
        Service::Smtp,
    );
    if param.srv_params[IDX_SMTP].pw.is_empty() && !param.srv_params[IDX_IMAP].pw.is_empty() {
        param.srv_params[IDX_SMTP].pw = param.srv_params[IDX_IMAP].pw.clone()
    }
    choose_starting_port_socket_security(&mut param.srv_params[IDX_SMTP], 465, 587, 25);

    // do we have a complete configuration?
    ensure!(
        !param.srv_params[IDX_IMAP].hostname.is_empty()
            && !imap_options.is_empty()
            && !(param.srv_params[IDX_IMAP].user.is_empty() && user_options.is_empty())
            && !param.srv_params[IDX_IMAP].pw.is_empty()
            && !param.srv_params[IDX_SMTP].hostname.is_empty()
            && !smtp_options.is_empty()
            && !(param.srv_params[IDX_SMTP].user.is_empty() && user_options.is_empty())
            && !param.srv_params[IDX_SMTP].pw.is_empty(),
        "Account settings incomplete."
    );

    progress!(ctx, 600);
    // try to connect to IMAP with selected options.
    try_connections(ctx, param, Service::Imap, &imap_options, &user_options).await?;
    progress!(ctx, 800);

    // try to connect to SMTP with selected options.
    try_connections(ctx, param, Service::Smtp, &smtp_options, &user_options).await?;
    progress!(ctx, 900);

    let create_mvbox = ctx.get_config_bool(Config::MvboxWatch).await
        || ctx.get_config_bool(Config::MvboxMove).await;

    let (_s, r) = async_std::sync::channel(1);
    let mut imap = Imap::new(r);
    if imap.connect(ctx, &param).await {
        imap.configure_folders(ctx, create_mvbox)
            .await
            .context("configuring folders failed")?;

        imap.select_with_uidvalidity(ctx, "INBOX")
            .await
            .context("could not read INBOX status")?;
    } else {
        return Err(format_err!("imap connection for folder config failed."));
    }

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

                        p.srv_params[IDX_IMAP].hostname = imap.hostname.to_string();
                        p.srv_params[IDX_IMAP].user =
                            imap.apply_username_pattern(param.addr.clone());
                        p.srv_params[IDX_IMAP].port = imap.port as i32;
                        p.srv_params[IDX_IMAP].certificate_checks =
                            CertificateChecks::AcceptInvalidCertificates;
                        p.srv_params[IDX_IMAP].security = match imap.socket {
                            provider::Socket::STARTTLS => Some(ServerSecurity::Starttls),
                            provider::Socket::SSL => Some(ServerSecurity::Ssl),
                        };

                        p.srv_params[IDX_SMTP].hostname = smtp.hostname.to_string();
                        p.srv_params[IDX_SMTP].user =
                            smtp.apply_username_pattern(param.addr.clone());
                        p.srv_params[IDX_SMTP].port = smtp.port as i32;
                        p.srv_params[IDX_SMTP].certificate_checks =
                            CertificateChecks::AcceptInvalidCertificates;
                        p.srv_params[IDX_SMTP].security = match smtp.socket {
                            provider::Socket::STARTTLS => Some(ServerSecurity::Starttls),
                            provider::Socket::SSL => Some(ServerSecurity::Ssl),
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

async fn try_connect(context: &Context, lp: LoginParam, service: Service) -> TryResult {
    let inf = format!(
        "{}: {}",
        service.as_ref(),
        lp.srv_params[service as usize],
    );
    info!(context, "Trying: {}", inf);
    let res = match service {
        Service::Imap => {
            let (_s, r) = async_std::sync::channel(1);
            let mut imap = Imap::new(r);
            imap.connect(context, &lp.clone()).await
        }
        Service::Smtp => {
            let mut smtp = Smtp::new();
            match smtp.connect(context, &lp.clone()).await {
                Ok(()) => true,
                Err(err) => {
                    warn!(context, "SMTP connection error: {}", err);
                    false
                }
            }
        }
    };
    if res {
        info!(context, "Success: {}", inf);
        return TryResult::Success(lp.srv_params[service as usize].clone());
    }
    TryResult::Failure(lp.srv_params[service as usize].clone())
}

async fn try_connections(
    context: &Context,
    param: &mut LoginParam,
    service: Service,
    srv_options: &Vec<ServerOption>,
    user_options: &Vec<String>,
) -> Result<()> {
    let mut res = Err(format_err!("{} config not found", service));
    let mut secure_opt_count: u32 = 0;
    let mut all_tries = FuturesUnordered::new();
    let user_options = if !param.srv_params[IDX_SMTP].user.is_empty() {
        vec![param.srv_params[IDX_SMTP].user.clone()]
    } else {
        user_options.clone()
    };
    for u in user_options {
        for s in srv_options {
            if s.security != ServerSecurity::PlainSocket {
                secure_opt_count += 1;
            }
            let mut p = param.clone();
            p.srv_params[service as usize].user = u.to_string();
            p.srv_params[service as usize].port = s.port;
            p.srv_params[service as usize].security = Some(s.security);
            all_tries.push(try_connect(context, p, service));
        }
    }
    loop {
        select! {
            try_res = all_tries.select_next_some() => {
                match try_res {
                    TryResult::Success(sp) => {
                        param.srv_params[service as usize].user = sp.user;
                        param.srv_params[service as usize].port = sp.port;
                        param.srv_params[service as usize].security = sp.security;
                        res = Ok(());
                        // Prioritise secure connections, so if security option is no, still wait for more secure options to complete
                        if sp.security.unwrap() != ServerSecurity::PlainSocket || secure_opt_count == 0 {
                            return res;
                        }
                    },
                    TryResult::Failure(sp) => {
                        if sp.security.unwrap() != ServerSecurity::PlainSocket {
                            assert!(secure_opt_count > 0);
                            secure_opt_count-=1;
                        }
                    }
                }
            },
            complete => {return res;},
        }
    }
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
        assert_eq!(
            found_params.srv_params[IDX_IMAP].hostname,
            "imap.nauta.cu".to_string()
        );
        assert_eq!(
            found_params.srv_params[IDX_SMTP].hostname,
            "smtp.nauta.cu".to_string()
        );
    }
}
