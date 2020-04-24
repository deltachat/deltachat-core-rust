//! Email accounts autoconfiguration process module

mod auto_mozilla;
mod auto_outlook;
mod read_url;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use async_std::task;
use futures::select;
use futures::stream::{FuturesUnordered, StreamExt};

use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::format_err;
use crate::imap::Imap;
use crate::job::{self, job_add, job_kill_action};
use crate::login_param::{
    AuthScheme, CertificateChecks, LoginParam, ServerParam, ServerSecurity, Service,
};
use crate::oauth2::*;
use crate::param::Params;
use crate::provider::Server;
use crate::smtp::Smtp;
use crate::{chat, e2ee, provider};

use crate::message::Message;
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

static IMAP_DEFAULT_PORTS: [i32; 3] = [143, 993, 993];
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

fn all_port_opt(port: i32) -> Vec<ServerOption> {
    server_options!([port, port, port])
}

fn imap_port2opt(port: i32) -> Vec<ServerOption> {
    match port {
        143 => server_options!(ServerSecurity::PlainSocket, port),
        993 => vec![
            ServerOption {
                security: ServerSecurity::Ssl,
                port: port,
            },
            ServerOption {
                security: ServerSecurity::Starttls,
                port: port,
            },
        ],
        // non_standard port specified, try all the security options.
        _ => all_port_opt(port),
    }
}

fn smtp_port2opt(port: i32) -> Vec<ServerOption> {
    match port {
        25 => server_options!(ServerSecurity::PlainSocket, port),
        465 => server_options!(ServerSecurity::Ssl, port),
        587 => server_options!(ServerSecurity::Starttls, port),
        // non_standard port specified, try all the security options.
        _ => all_port_opt(port),
    }
}

fn port2opt(port: i32, service: Service) -> Vec<ServerOption> {
    match service {
        Service::Imap => imap_port2opt(port),
        Service::Smtp => smtp_port2opt(port),
    }
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
        let sec: ServerSecurity = security.unwrap();
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
    /// Starts a configuration job.
    pub fn configure(&self) {
        if self.has_ongoing() {
            warn!(self, "There is already another ongoing process running.",);
            return;
        }
        job_kill_action(self, job::Action::ConfigureImap);
        job_add(self, job::Action::ConfigureImap, 0, Params::new(), 0);
    }

    /// Checks if the context is already configured.
    pub fn is_configured(&self) -> bool {
        self.sql.get_raw_config_bool(self, "configured")
    }
}

/*******************************************************************************
 * Configure JOB
 ******************************************************************************/
#[allow(non_snake_case, unused_must_use, clippy::cognitive_complexity)]
pub(crate) fn JobConfigureImap(context: &Context) -> job::Status {
    if !context.sql.is_open() {
        error!(context, "Cannot configure, database not opened.",);
        progress!(context, 0);
        return job::Status::Finished(Err(format_err!("Database not opened")));
    }
    if !context.alloc_ongoing() {
        progress!(context, 0);
        return job::Status::Finished(Err(format_err!("Cannot allocated ongoing process")));
    }
    let mut success = false;
    let mut imap_connected_here = false;
    let mut smtp_connected_here = false;

    let mut param_autoconfig: Option<LoginParam> = None;
    let was_configured_before = context.is_configured();

    context
        .inbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
    context
        .sentbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
    context
        .mvbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
    context.smtp.clone().lock().unwrap().disconnect();
    info!(context, "Configure ...",);

    // Variables that are shared between steps:
    let mut param = LoginParam::from_database(context, "");
    // need all vars here to be mutable because rust thinks the same step could be called multiple times
    // and also initialize, because otherwise rust thinks it's used while unitilized, even if thats not the case as the loop goes only forward
    let mut param_domain = "undefined.undefined".to_owned();
    let mut param_addr_urlencoded: String =
        "Internal Error: this value should never be used".to_owned();
    let mut keep_oauth2 = false;

    const STEP_12_USE_AUTOCONFIG: u8 = 12;
    const STEP_13_AFTER_AUTOCONFIG: u8 = 13;

    let mut step_counter: u8 = 0;

    let mut user_options: Vec<String> = Vec::new();
    let mut imap_options: Vec<ServerOption> = Vec::new();
    let mut smtp_options: Vec<ServerOption> = Vec::new();

    while !context.shall_stop_ongoing() {
        step_counter += 1;

        let success = match step_counter {
            // Read login parameters from the database
            1 => {
                progress!(context, 1);
                if param.addr.is_empty() {
                    error!(context, "Please enter an email address.",);
                }
                !param.addr.is_empty()
            }
            // Step 1: Load the parameters and check email-address and password
            2 => {
                if param.auth_scheme == AuthScheme::Oauth2 {
                    // the used oauth2 addr may differ, check this.
                    // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                    // just use the given one.
                    progress!(context, 10);
                    if let Some(oauth2_addr) = dc_get_oauth2_addr(
                        context,
                        &param.addr,
                        &param.srv_params[Service::Imap as usize].pw,
                    )
                    .and_then(|e| e.parse().ok())
                    {
                        info!(context, "Authorized address is {}", oauth2_addr);
                        param.addr = oauth2_addr;
                        context
                            .sql
                            .set_raw_config(context, "addr", Some(param.addr.as_str()))
                            .ok();
                    }
                    progress!(context, 20);
                }
                true // no oauth? - just continue it's no error
            }
            3 => {
                if let Ok(parsed) = param.addr.parse() {
                    let parsed: EmailAddress = parsed;
                    param_domain = parsed.domain;
                    param_addr_urlencoded =
                        utf8_percent_encode(&param.addr, NON_ALPHANUMERIC).to_string();
                    true
                } else {
                    error!(context, "Bad email-address.");
                    false
                }
            }
            // Step 2: Autoconfig
            4 => {
                progress!(context, 200);

                if param.srv_params[Service::Imap as usize].hostname.is_empty()
                    && param.srv_params[Service::Imap as usize].port == 0
                /*&&param.srv_params[Service::Imap as usize].user.is_empty() -- the user can enter a loginname which is used by autoconfig then */
                    && param.srv_params[Service::Smtp as usize].hostname.is_empty()
                    && param.srv_params[Service::Smtp as usize].port == 0
                    && param.srv_params[Service::Smtp as usize].user.is_empty()
                /*&&param.srv_params[Service::Smtp as usize].pw.is_empty() -- the password cannot be auto-configured and is no criterion for autoconfig or not */
                    && param.auth_scheme == AuthScheme::Oauth2
                {
                    // no advanced parameters entered by the user: query provider-database or do Autoconfig
                    keep_oauth2 = true;
                    if let Some(new_param) = get_offline_autoconfig(context, &param) {
                        // got parameters from our provider-database, skip Autoconfig, preserve the OAuth2 setting
                        param_autoconfig = Some(new_param);
                        step_counter = STEP_12_USE_AUTOCONFIG - 1; // minus one as step_counter is increased on next loop
                    }
                } else {
                    // advanced parameters entered by the user: skip Autoconfig
                    step_counter = STEP_13_AFTER_AUTOCONFIG - 1; // minus one as step_counter is increased on next loop
                }
                true
            }
            /* A.  Search configurations from the domain used in the email-address, prefer encrypted */
            5 => {
                if param_autoconfig.is_none() {
                    let url = format!(
                        "https://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                        param_domain, param_addr_urlencoded
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            6 => {
                progress!(context, 300);
                if param_autoconfig.is_none() {
                    // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
                    let url = format!(
                        "https://{}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={}",
                        param_domain, param_addr_urlencoded
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            /* Outlook section start ------------- */
            /* Outlook uses always SSL but different domains (this comment describes the next two steps) */
            7 => {
                progress!(context, 310);
                if param_autoconfig.is_none() {
                    let url = format!("https://{}/autodiscover/autodiscover.xml", param_domain);
                    param_autoconfig = outlk_autodiscover(context, &url, &param).ok();
                }
                true
            }
            8 => {
                progress!(context, 320);
                if param_autoconfig.is_none() {
                    let url = format!(
                        "https://{}{}/autodiscover/autodiscover.xml",
                        "autodiscover.", param_domain
                    );
                    param_autoconfig = outlk_autodiscover(context, &url, &param).ok();
                }
                true
            }
            /* ----------- Outlook section end */
            9 => {
                progress!(context, 330);
                if param_autoconfig.is_none() {
                    let url = format!(
                        "http://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                        param_domain, param_addr_urlencoded
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            10 => {
                progress!(context, 340);
                if param_autoconfig.is_none() {
                    // do not transfer the email-address unencrypted
                    let url = format!(
                        "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
                        param_domain
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
            11 => {
                progress!(context, 350);
                if param_autoconfig.is_none() {
                    /* always SSL for Thunderbird's database */
                    let url = format!("https://autoconfig.thunderbird.net/v1.1/{}", param_domain);
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            /* C.  Do we have any autoconfig result?
               If you change the match-number here, also update STEP_12_COPY_AUTOCONFIG above
            */
            STEP_12_USE_AUTOCONFIG => {
                progress!(context, 500);
                if let Some(ref cfg) = param_autoconfig {
                    info!(context, "Got autoconfig: {}", &cfg);
                    if !cfg.srv_params[Service::Imap as usize].user.is_empty() {
                        param.srv_params[Service::Imap as usize].user =
                            cfg.srv_params[Service::Imap as usize].user.clone();
                    }
                    param.srv_params[Service::Imap as usize].hostname =
                        cfg.srv_params[Service::Imap as usize].hostname.clone(); /* all other values are always NULL when entering autoconfig */
                    imap_options = server_options!(
                        param.srv_params[Service::Imap as usize].security.unwrap(),
                        param.srv_params[Service::Imap as usize].port
                    );
                    param.srv_params[Service::Smtp as usize].hostname =
                        cfg.srv_params[Service::Smtp as usize].hostname.clone();
                    smtp_options = server_options!(
                        param.srv_params[Service::Smtp as usize].security.unwrap(),
                        param.srv_params[Service::Smtp as usize].port
                    );
                    param.srv_params[Service::Smtp as usize].user =
                        cfg.srv_params[Service::Smtp as usize].user.clone();
                    /* although param_autoconfig's data are no longer needed from,
                    it is used to later to prevent trying variations of port/server/logins */
                }
                if keep_oauth2 {
                    param.auth_scheme = AuthScheme::Oauth2;
                }
                true
            }
            // Step 3: Fill missing fields with defaults
            // If you change the match-number here, also update STEP_13_AFTER_AUTOCONFIG above
            STEP_13_AFTER_AUTOCONFIG => {
                if param.srv_params[Service::Imap as usize].hostname.is_empty() {
                    param.srv_params[Service::Imap as usize].hostname =
                        format!("imap.{}", param_domain,)
                }

                imap_options = select_server_options(
                    param.srv_params[Service::Imap as usize].port,
                    param.srv_params[Service::Imap as usize].security,
                    Service::Imap,
                );

                if param.srv_params[Service::Imap as usize].user.is_empty()
                    || param.srv_params[Service::Smtp as usize].user.is_empty()
                {
                    user_options.push(param.addr.clone());

                    if let Some(at) = param.addr.find('@') {
                        user_options.push(param.addr.split_at(at).0.to_string());
                    }
                }
                if param.srv_params[Service::Smtp as usize].hostname.is_empty()
                    && !param.srv_params[Service::Imap as usize].hostname.is_empty()
                {
                    param.srv_params[Service::Smtp as usize].hostname =
                        param.srv_params[Service::Imap as usize].hostname.clone();
                    if param.srv_params[Service::Smtp as usize]
                        .hostname
                        .starts_with("imap.")
                    {
                        param.srv_params[Service::Smtp as usize].hostname = param.srv_params
                            [Service::Smtp as usize]
                            .hostname
                            .replacen("imap", "smtp", 1);
                    }
                }

                smtp_options = select_server_options(
                    param.srv_params[Service::Smtp as usize].port,
                    param.srv_params[Service::Smtp as usize].security,
                    Service::Smtp,
                );

                if param.srv_params[Service::Smtp as usize].pw.is_empty()
                    && !param.srv_params[Service::Imap as usize].pw.is_empty()
                {
                    param.srv_params[Service::Smtp as usize].pw =
                        param.srv_params[Service::Imap as usize].pw.clone()
                }
                /* do we have a complete configuration? */
                if param.srv_params[Service::Imap as usize].hostname.is_empty()
                    || imap_options.is_empty()
                    || (param.srv_params[Service::Imap as usize].user.is_empty()
                        && user_options.is_empty())
                    || param.srv_params[Service::Imap as usize].pw.is_empty()
                    || param.srv_params[Service::Smtp as usize].hostname.is_empty()
                    || smtp_options.is_empty()
                    || (param.srv_params[Service::Smtp as usize].user.is_empty()
                        && user_options.is_empty())
                    || param.srv_params[Service::Smtp as usize].pw.is_empty()
                {
                    error!(context, "Account settings incomplete.");
                    false
                } else {
                    true
                }
            }
            14 => {
                progress!(context, 600);
                /* try to connect to IMAP - if we did not got an autoconfig,
                do some further tries with different settings and username variations */
                imap_connected_here = try_srv_options(
                    context,
                    &mut param,
                    Service::Imap,
                    &imap_options,
                    &user_options,
                );
                imap_connected_here
            }
            15 => {
                progress!(context, 800);
                smtp_connected_here = try_srv_options(
                    context,
                    &mut param,
                    Service::Smtp,
                    &smtp_options,
                    &user_options,
                );
                smtp_connected_here
            }
            16 => {
                progress!(context, 900);
                let create_mvbox = context.get_config_bool(Config::MvboxWatch)
                    || context.get_config_bool(Config::MvboxMove);
                let imap = &context.inbox_thread.read().unwrap().imap;
                if task::block_on(imap.connect(context, &param)) {
                    if let Err(err) = imap.configure_folders(context, create_mvbox) {
                        warn!(context, "configuring folders failed: {:?}", err);
                        false
                    } else {
                        let res = imap.select_with_uidvalidity(context, "INBOX");
                        if let Err(err) = res {
                            error!(context, "could not read INBOX status: {:?}", err);
                            false
                        } else {
                            true
                        }
                    }
                } else {
                    false
                }
            }
            17 => {
                progress!(context, 910);
                /* configuration success - write back the configured parameters with the "configured_" prefix; also write the "configured"-flag */
                param
                    .save_to_database(
                        context,
                        "configured_", /*the trailing underscore is correct*/
                    )
                    .ok();

                context.sql.set_raw_config_bool(context, "configured", true);
                true
            }
            18 => {
                progress!(context, 920);
                // we generate the keypair just now - we could also postpone this until the first message is sent, however,
                // this may result in a unexpected and annoying delay when the user sends his very first message
                // (~30 seconds on a Moto G4 play) and might looks as if message sending is always that slow.
                e2ee::ensure_secret_key_exists(context);
                success = true;
                info!(context, "key generation completed");
                progress!(context, 940);
                break; // We are done here
            }
            _ => {
                error!(context, "Internal error: step counter out of bound",);
                break;
            }
        };

        if !success {
            break;
        }
    }
    if imap_connected_here {
        context
            .inbox_thread
            .read()
            .unwrap()
            .imap
            .disconnect(context);
    }
    if smtp_connected_here {
        context.smtp.clone().lock().unwrap().disconnect();
    }

    // remember the entered parameters on success
    // and restore to last-entered on failure.
    // this way, the parameters visible to the ui are always in-sync with the current configuration.
    if success {
        LoginParam::from_database(context, "").save_to_database(context, "configured_raw_");
    } else {
        LoginParam::from_database(context, "configured_raw_").save_to_database(context, "");
    }

    if let Some(provider) = provider::get_provider_info(&param.addr) {
        if !was_configured_before {
            if let Some(config_defaults) = &provider.config_defaults {
                for def in config_defaults.iter() {
                    info!(context, "apply config_defaults {}={}", def.key, def.value);
                    context.set_config(def.key, Some(def.value));
                }
            }
        }

        if !provider.after_login_hint.is_empty() {
            let mut msg = Message::new(Viewtype::Text);
            msg.text = Some(provider.after_login_hint.to_string());
            if chat::add_device_msg(context, Some("core-provider-info"), Some(&mut msg)).is_err() {
                warn!(context, "cannot add after_login_hint as core-provider-info");
            }
        }
    }

    context.free_ongoing();
    progress!(context, if success { 1000 } else { 0 });
    job::Status::Finished(Ok(()))
}

fn set_offline_autoconf_one_server(p: &mut ServerParam, s: &Server, addr: String) {
    p.user = s.apply_username_pattern(addr);
    p.port = s.port as i32;
    p.certificate_checks = CertificateChecks::AcceptInvalidCertificates;
    p.security = Some(match s.socket {
        provider::Socket::STARTTLS => ServerSecurity::Starttls,
        provider::Socket::SSL => ServerSecurity::Ssl,
    });
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

                    set_offline_autoconf_one_server(
                        &mut p.srv_params[Service::Imap as usize],
                        imap,
                        param.addr.clone(),
                    );
                    set_offline_autoconf_one_server(
                        &mut p.srv_params[Service::Smtp as usize],
                        smtp,
                        param.addr.clone(),
                    );

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

async fn try_connect(context: &Context, lp: LoginParam, service: Service) -> TryResult {
    // It is better to use Display for ServerParams here, but it formats data differenly.
    let inf = format!(
        "{}: {}@{}:{} security={:?} certificate_checks={}",
        service.as_ref(),
        lp.srv_params[service as usize].user,
        lp.srv_params[service as usize].hostname,
        lp.srv_params[service as usize].port,
        lp.srv_params[service as usize].security,
        lp.srv_params[service as usize].certificate_checks
    );
    info!(context, "Trying: {}", inf);
    let res = match service {
        Service::Imap => {
            let imap = Imap::new();
            imap.connect(context, &lp.clone()).await
        }
        Service::Smtp => {
            let mut smtp = Smtp::new();
            smtp.try_connect(context, &lp.clone()).await
        }
    };
    if res {
        info!(context, "Success: {}", inf);
        return TryResult::Success(lp.srv_params[service as usize].clone());
    }
    TryResult::Failure(lp.srv_params[service as usize].clone())
}

async fn try_srv_options_async(
    context: &Context,
    param: &mut LoginParam,
    service: Service,
    srv_options: &Vec<ServerOption>,
    user_options: &Vec<String>,
) -> bool {
    let mut res = false;
    // Count TLS and STARTTLS options, decrement on failure of such an option
    // If there are still unchecked secure options and plain socket option is successful,
    // we still wait for result, in order to prioritize secure options.
    let mut secure_opt_count: u32 = 0;
    let mut all_tries = FuturesUnordered::new();
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
                        res = true;
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

fn try_srv_options(
    context: &Context,
    param: &mut LoginParam,
    service: Service,
    srv_options: &Vec<ServerOption>,
    user_options: &Vec<String>,
) -> bool {
    let user_options = if !param.srv_params[service as usize].user.is_empty() {
        vec![param.srv_params[service as usize].user.clone()]
    } else {
        user_options.clone()
    };
    task::block_on(try_srv_options_async(
        context,
        param,
        service,
        srv_options,
        &user_options,
    ))
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
    use crate::configure::JobConfigureImap;
    use crate::test_utils::*;

    #[test]
    fn test_no_panic_on_bad_credentials() {
        let t = dummy_context();
        t.ctx
            .set_config(Config::Addr, Some("probably@unexistant.addr"))
            .unwrap();
        t.ctx.set_config(Config::MailPw, Some("123456")).unwrap();
        JobConfigureImap(&t.ctx);
    }

    #[test]
    fn test_get_offline_autoconfig() {
        let context = dummy_context().ctx;

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
