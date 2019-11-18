use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use async_imap::{
    error::Result as ImapResult,
    types::{Fetch, Flag, Mailbox, Name, NameAttribute},
};
use async_std::prelude::*;
use async_std::sync::{Arc, Mutex, RwLock};
use async_std::task;

use crate::configure::dc_connect_to_configured_imap;
use crate::constants::*;
use crate::context::Context;
use crate::dc_receive_imf::dc_receive_imf;
use crate::error::Error;
use crate::events::Event;
use crate::imap_client::*;
use crate::job::{connect_to_inbox, job_add, Action};
use crate::login_param::{CertificateChecks, LoginParam};
use crate::message::{self, update_msg_move_state, update_server_uid};
use crate::oauth2::dc_get_oauth2_access_token;
use crate::param::Params;
use crate::stock::StockMessage;
use crate::wrapmime;

const DC_IMAP_SEEN: usize = 0x0001;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum ImapActionResult {
    Failed,
    RetryLater,
    AlreadyDone,
    Success,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum IdlePollMode {
    Often,
    Never,
}

const PREFETCH_FLAGS: &str = "(UID ENVELOPE)";
const BODY_FLAGS: &str = "(FLAGS BODY.PEEK[])";
const SELECT_ALL: &str = "1:*";

#[derive(Debug)]
pub struct Imap {
    config: Arc<RwLock<ImapConfig>>,

    session: Arc<Mutex<Option<Session>>>,
    connected: Arc<Mutex<bool>>,
    interrupt: Arc<Mutex<Option<stop_token::StopSource>>>,
    skip_next_idle_wait: AtomicBool,
    should_reconnect: AtomicBool,
}

#[derive(Debug)]
struct OAuth2 {
    user: String,
    access_token: String,
}

impl async_imap::Authenticator for OAuth2 {
    type Response = String;

    fn process(&self, _data: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

#[derive(Debug)]
enum FolderMeaning {
    Unknown,
    SentObjects,
    Other,
}

#[derive(Debug)]
struct ImapConfig {
    pub addr: String,
    pub imap_server: String,
    pub imap_port: u16,
    pub imap_user: String,
    pub imap_pw: String,
    pub certificate_checks: CertificateChecks,
    pub server_flags: usize,
    pub selected_folder: Option<String>,
    pub selected_mailbox: Option<Mailbox>,
    pub selected_folder_needs_expunge: bool,
    pub can_idle: bool,
    pub has_xlist: bool,
    pub imap_delimiter: char,
    pub watch_folder: Option<String>,
}

impl Default for ImapConfig {
    fn default() -> Self {
        ImapConfig {
            addr: "".into(),
            imap_server: "".into(),
            imap_port: 0,
            imap_user: "".into(),
            imap_pw: "".into(),
            certificate_checks: Default::default(),
            server_flags: 0,
            selected_folder: None,
            selected_mailbox: None,
            selected_folder_needs_expunge: false,
            can_idle: false,
            has_xlist: false,
            imap_delimiter: '.',
            watch_folder: None,
        }
    }
}

impl Imap {
    pub fn new() -> Self {
        Imap {
            session: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(ImapConfig::default())),
            interrupt: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
            skip_next_idle_wait: AtomicBool::new(false),
            should_reconnect: AtomicBool::new(false),
        }
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    pub fn should_reconnect(&self) -> bool {
        self.should_reconnect.load(Ordering::Relaxed)
    }

    fn trigger_reconnect(&self) {
        self.should_reconnect.store(true, Ordering::Relaxed)
    }

    fn setup_handle_if_needed(&self, context: &Context) -> bool {
        task::block_on(async move {
            if self.config.read().await.imap_server.is_empty() {
                return false;
            }

            if self.should_reconnect() {
                self.unsetup_handle(context).await;
                self.should_reconnect.store(false, Ordering::Relaxed);
            } else if self.is_connected().await {
                return true;
            }

            let server_flags = self.config.read().await.server_flags as i32;

            let connection_res: ImapResult<Client> =
                if (server_flags & (DC_LP_IMAP_SOCKET_STARTTLS | DC_LP_IMAP_SOCKET_PLAIN)) != 0 {
                    let config = self.config.read().await;
                    let imap_server: &str = config.imap_server.as_ref();
                    let imap_port = config.imap_port;

                    match Client::connect_insecure((imap_server, imap_port)).await {
                        Ok(client) => {
                            if (server_flags & DC_LP_IMAP_SOCKET_STARTTLS) != 0 {
                                let res =
                                    client.secure(imap_server, config.certificate_checks).await;
                                res
                            } else {
                                Ok(client)
                            }
                        }
                        Err(err) => Err(err),
                    }
                } else {
                    let config = self.config.read().await;
                    let imap_server: &str = config.imap_server.as_ref();
                    let imap_port = config.imap_port;

                    let res = Client::connect_secure(
                        (imap_server, imap_port),
                        imap_server,
                        config.certificate_checks,
                    )
                    .await;
                    res
                };

            let login_res = match connection_res {
                Ok(client) => {
                    let config = self.config.read().await;
                    let imap_user: &str = config.imap_user.as_ref();
                    let imap_pw: &str = config.imap_pw.as_ref();

                    if (server_flags & DC_LP_AUTH_OAUTH2) != 0 {
                        let addr: &str = config.addr.as_ref();

                        if let Some(token) =
                            dc_get_oauth2_access_token(context, addr, imap_pw, true)
                        {
                            let auth = OAuth2 {
                                user: imap_user.into(),
                                access_token: token,
                            };
                            let res = client.authenticate("XOAUTH2", &auth).await;
                            res
                        } else {
                            return false;
                        }
                    } else {
                        let res = client.login(imap_user, imap_pw).await;
                        res
                    }
                }
                Err(err) => {
                    let config = self.config.read().await;
                    let imap_server: &str = config.imap_server.as_ref();
                    let imap_port = config.imap_port;
                    let message = context.stock_string_repl_str2(
                        StockMessage::ServerResponse,
                        format!("{}:{}", imap_server, imap_port),
                        format!("{}", err),
                    );

                    emit_event!(context, Event::ErrorNetwork(message));

                    return false;
                }
            };

            self.should_reconnect.store(false, Ordering::Relaxed);

            match login_res {
                Ok(session) => {
                    *self.session.lock().await = Some(session);
                    true
                }
                Err((err, _)) => {
                    let imap_user = self.config.read().await.imap_user.to_owned();
                    let message =
                        context.stock_string_repl_str(StockMessage::CannotLogin, &imap_user);

                    emit_event!(
                        context,
                        Event::ErrorNetwork(format!("{} ({})", message, err))
                    );
                    self.unsetup_handle(context).await;

                    false
                }
            }
        })
    }

    async fn unsetup_handle(&self, context: &Context) {
        info!(
            context,
            "IMAP unsetup_handle step 2 (acquiring session.lock)"
        );
        if let Some(mut session) = self.session.lock().await.take() {
            if let Err(err) = session.close().await {
                warn!(context, "failed to close connection: {:?}", err);
            }
        }

        info!(context, "IMAP unsetup_handle step 3 (clearing config).");
        self.config.write().await.selected_folder = None;
        self.config.write().await.selected_mailbox = None;
        info!(context, "IMAP unsetup_handle step 4 (disconnected).");
    }

    async fn free_connect_params(&self) {
        let mut cfg = self.config.write().await;

        cfg.addr = "".into();
        cfg.imap_server = "".into();
        cfg.imap_user = "".into();
        cfg.imap_pw = "".into();
        cfg.imap_port = 0;

        cfg.can_idle = false;
        cfg.has_xlist = false;

        cfg.watch_folder = None;
    }

    pub fn connect(&self, context: &Context, lp: &LoginParam) -> bool {
        task::block_on(async move {
            if lp.mail_server.is_empty() || lp.mail_user.is_empty() || lp.mail_pw.is_empty() {
                return false;
            }

            if self.is_connected().await {
                return true;
            }

            {
                let addr = &lp.addr;
                let imap_server = &lp.mail_server;
                let imap_port = lp.mail_port as u16;
                let imap_user = &lp.mail_user;
                let imap_pw = &lp.mail_pw;
                let server_flags = lp.server_flags as usize;

                let mut config = self.config.write().await;
                config.addr = addr.to_string();
                config.imap_server = imap_server.to_string();
                config.imap_port = imap_port;
                config.imap_user = imap_user.to_string();
                config.imap_pw = imap_pw.to_string();
                config.certificate_checks = lp.imap_certificate_checks;
                config.server_flags = server_flags;
            }

            if !self.setup_handle_if_needed(context) {
                self.free_connect_params().await;
                return false;
            }

            let teardown = match &mut *self.session.lock().await {
                Some(ref mut session) => match session.capabilities().await {
                    Ok(caps) => {
                        if !context.sql.is_open() {
                            warn!(context, "IMAP-LOGIN as {} ok but ABORTING", lp.mail_user,);
                            true
                        } else {
                            let can_idle = caps.has_str("IDLE");
                            let has_xlist = caps.has_str("XLIST");
                            let caps_list = caps
                                .iter()
                                .fold(String::new(), |s, c| s + &format!(" {:?}", c));
                            self.config.write().await.can_idle = can_idle;
                            self.config.write().await.has_xlist = has_xlist;
                            *self.connected.lock().await = true;
                            emit_event!(
                                context,
                                Event::ImapConnected(format!(
                                    "IMAP-LOGIN as {}, capabilities: {}",
                                    lp.mail_user, caps_list,
                                ))
                            );
                            false
                        }
                    }
                    Err(err) => {
                        info!(context, "CAPABILITY command error: {}", err);
                        true
                    }
                },
                None => true,
            };

            if teardown {
                self.disconnect(context);

                false
            } else {
                true
            }
        })
    }

    pub fn disconnect(&self, context: &Context) {
        task::block_on(async move {
            if self.is_connected().await {
                self.unsetup_handle(context).await;
                self.free_connect_params().await;
                *self.connected.lock().await = false;
            }
        });
    }

    pub fn set_watch_folder(&self, watch_folder: String) {
        task::block_on(async move {
            self.config.write().await.watch_folder = Some(watch_folder);
        });
    }

    pub fn fetch(&self, context: &Context) -> bool {
        task::block_on(async move {
            if !context.sql.is_open() {
                // probably shutdown
                return false;
            }

            self.setup_handle_if_needed(context);

            let watch_folder = self.config.read().await.watch_folder.to_owned();

            if let Some(ref watch_folder) = watch_folder {
                // as during the fetch commands, new messages may arrive, we fetch until we do not
                // get any more. if IDLE is called directly after, there is only a small chance that
                // messages are missed and delayed until the next IDLE call
                loop {
                    if self.fetch_from_single_folder(context, watch_folder).await == 0 {
                        break;
                    }
                }
                true
            } else {
                false
            }
        })
    }

    async fn select_folder<S: AsRef<str>>(
        &self,
        context: &Context,
        folder: Option<S>,
    ) -> ImapActionResult {
        if self.session.lock().await.is_none() {
            let mut cfg = self.config.write().await;
            cfg.selected_folder = None;
            cfg.selected_folder_needs_expunge = false;
            return ImapActionResult::Failed;
        }

        // if there is a new folder and the new folder is equal to the selected one, there's nothing to do.
        // if there is _no_ new folder, we continue as we might want to expunge below.
        if let Some(ref folder) = folder {
            if let Some(ref selected_folder) = self.config.read().await.selected_folder {
                if folder.as_ref() == selected_folder {
                    return ImapActionResult::AlreadyDone;
                }
            }
        }

        // deselect existing folder, if needed (it's also done implicitly by SELECT, however, without EXPUNGE then)
        let needs_expunge = { self.config.read().await.selected_folder_needs_expunge };
        if needs_expunge {
            if let Some(ref folder) = self.config.read().await.selected_folder {
                info!(context, "Expunge messages in \"{}\".", folder);

                // A CLOSE-SELECT is considerably faster than an EXPUNGE-SELECT, see
                // https://tools.ietf.org/html/rfc3501#section-6.4.2
                if let Some(ref mut session) = &mut *self.session.lock().await {
                    match session.close().await {
                        Ok(_) => {
                            info!(context, "close/expunge succeeded");
                        }
                        Err(err) => {
                            warn!(context, "failed to close session: {:?}", err);
                            return ImapActionResult::Failed;
                        }
                    }
                } else {
                    return ImapActionResult::Failed;
                }
            }
            self.config.write().await.selected_folder_needs_expunge = false;
        }

        // select new folder
        if let Some(ref folder) = folder {
            if let Some(ref mut session) = &mut *self.session.lock().await {
                match session.select(folder).await {
                    Ok(mailbox) => {
                        let mut config = self.config.write().await;
                        config.selected_folder = Some(folder.as_ref().to_string());
                        config.selected_mailbox = Some(mailbox);
                    }
                    Err(err) => {
                        warn!(
                            context,
                            "Cannot select folder: {}; {:?}.",
                            folder.as_ref(),
                            err
                        );

                        self.config.write().await.selected_folder = None;
                        self.trigger_reconnect();
                        return ImapActionResult::Failed;
                    }
                }
            } else {
                unreachable!();
            }
        }

        ImapActionResult::Success
    }

    fn get_config_last_seen_uid<S: AsRef<str>>(&self, context: &Context, folder: S) -> (u32, u32) {
        let key = format!("imap.mailbox.{}", folder.as_ref());
        if let Some(entry) = context.sql.get_raw_config(context, &key) {
            // the entry has the format `imap.mailbox.<folder>=<uidvalidity>:<lastseenuid>`
            let mut parts = entry.split(':');
            (
                parts
                    .next()
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or_else(|_| 0),
                parts
                    .next()
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or_else(|_| 0),
            )
        } else {
            (0, 0)
        }
    }

    async fn fetch_from_single_folder<S: AsRef<str>>(&self, context: &Context, folder: S) -> usize {
        match self.select_folder(context, Some(&folder)).await {
            ImapActionResult::Failed | ImapActionResult::RetryLater => {
                warn!(
                    context,
                    "Cannot select folder \"{}\" for fetching.",
                    folder.as_ref()
                );
                return 0;
            }
            ImapActionResult::Success | ImapActionResult::AlreadyDone => {}
        }

        // compare last seen UIDVALIDITY against the current one
        let (mut uid_validity, mut last_seen_uid) = self.get_config_last_seen_uid(context, &folder);

        let config = self.config.read().await;
        let mailbox = config.selected_mailbox.as_ref().expect("just selected");

        if mailbox.uid_validity.is_none() {
            error!(
                context,
                "Cannot get UIDVALIDITY for folder \"{}\".",
                folder.as_ref(),
            );

            return 0;
        }

        if mailbox.uid_validity.unwrap_or_default() != uid_validity {
            // first time this folder is selected or UIDVALIDITY has changed, init lastseenuid and save it to config

            if mailbox.exists == 0 {
                info!(context, "Folder \"{}\" is empty.", folder.as_ref());

                // set lastseenuid=0 for empty folders.
                // id we do not do this here, we'll miss the first message
                // as we will get in here again and fetch from lastseenuid+1 then

                self.set_config_last_seen_uid(
                    context,
                    &folder,
                    mailbox.uid_validity.unwrap_or_default(),
                    0,
                );
                return 0;
            }

            let list = if let Some(ref mut session) = &mut *self.session.lock().await {
                // `FETCH <message sequence number> (UID)`
                let set = format!("{}", mailbox.exists);
                match session.fetch(set, PREFETCH_FLAGS).await {
                    Ok(list) => list,
                    Err(_err) => {
                        self.trigger_reconnect();
                        info!(
                            context,
                            "No result returned for folder \"{}\".",
                            folder.as_ref()
                        );

                        return 0;
                    }
                }
            } else {
                return 0;
            };

            last_seen_uid = list[0].uid.unwrap_or_else(|| 0);

            // if the UIDVALIDITY has _changed_, decrease lastseenuid by one to avoid gaps (well add 1 below
            if uid_validity > 0 && last_seen_uid > 1 {
                last_seen_uid -= 1;
            }

            uid_validity = mailbox.uid_validity.unwrap_or_default();
            self.set_config_last_seen_uid(context, &folder, uid_validity, last_seen_uid);
            info!(
                context,
                "lastseenuid initialized to {} for {}@{}",
                last_seen_uid,
                folder.as_ref(),
                uid_validity,
            );
        }

        let mut read_cnt = 0;
        let mut read_errors = 0;
        let mut new_last_seen_uid = 0;

        let list = if let Some(ref mut session) = &mut *self.session.lock().await {
            // fetch messages with larger UID than the last one seen
            // (`UID FETCH lastseenuid+1:*)`, see RFC 4549
            let set = format!("{}:*", last_seen_uid + 1);
            match session.uid_fetch(set, PREFETCH_FLAGS).await {
                Ok(list) => list,
                Err(err) => {
                    warn!(context, "failed to fetch uids: {}", err);
                    return 0;
                }
            }
        } else {
            return 0;
        };

        // go through all mails in folder (this is typically _fast_ as we already have the whole list)
        for msg in &list {
            let cur_uid = msg.uid.unwrap_or_else(|| 0);
            if cur_uid > last_seen_uid {
                read_cnt += 1;

                let message_id = prefetch_get_message_id(msg).unwrap_or_default();

                if !precheck_imf(context, &message_id, folder.as_ref(), cur_uid) {
                    // check passed, go fetch the rest
                    if self.fetch_single_msg(context, &folder, cur_uid).await == 0 {
                        info!(
                            context,
                            "Read error for message {} from \"{}\", trying over later.",
                            message_id,
                            folder.as_ref()
                        );

                        read_errors += 1;
                    }
                } else {
                    // check failed
                    info!(
                        context,
                        "Skipping message {} from \"{}\" by precheck.",
                        message_id,
                        folder.as_ref(),
                    );
                }
                if cur_uid > new_last_seen_uid {
                    new_last_seen_uid = cur_uid
                }
            }
        }

        if 0 == read_errors && new_last_seen_uid > 0 {
            // TODO: it might be better to increase the lastseenuid also on partial errors.
            // however, this requires to sort the list before going through it above.
            self.set_config_last_seen_uid(context, &folder, uid_validity, new_last_seen_uid);
        }

        if read_errors > 0 {
            warn!(
                context,
                "{} mails read from \"{}\" with {} errors.",
                read_cnt,
                folder.as_ref(),
                read_errors
            );
        } else {
            info!(
                context,
                "{} mails read from \"{}\".",
                read_cnt,
                folder.as_ref()
            );
        }

        read_cnt
    }

    fn set_config_last_seen_uid<S: AsRef<str>>(
        &self,
        context: &Context,
        folder: S,
        uidvalidity: u32,
        lastseenuid: u32,
    ) {
        let key = format!("imap.mailbox.{}", folder.as_ref());
        let val = format!("{}:{}", uidvalidity, lastseenuid);

        context.sql.set_raw_config(context, &key, Some(&val)).ok();
    }

    async fn fetch_single_msg<S: AsRef<str>>(
        &self,
        context: &Context,
        folder: S,
        server_uid: u32,
    ) -> usize {
        // the function returns:
        // 0  the caller should try over again later
        // or  1  if the messages should be treated as received, the caller should not try to read the message again (even if no database entries are returned)
        if !self.is_connected().await {
            return 0;
        }

        let set = format!("{}", server_uid);

        let msgs = if let Some(ref mut session) = &mut *self.session.lock().await {
            match session.uid_fetch(set, BODY_FLAGS).await {
                Ok(msgs) => msgs,
                Err(err) => {
                    self.trigger_reconnect();
                    warn!(
                        context,
                        "Error on fetching message #{} from folder \"{}\"; retry={}; error={}.",
                        server_uid,
                        folder.as_ref(),
                        self.should_reconnect(),
                        err
                    );
                    return 0;
                }
            }
        } else {
            return 1;
        };

        if msgs.is_empty() {
            warn!(
                context,
                "Message #{} does not exist in folder \"{}\".",
                server_uid,
                folder.as_ref()
            );
        } else {
            let msg = &msgs[0];

            // XXX put flags into a set and pass them to dc_receive_imf
            let is_deleted = msg.flags().any(|flag| match flag {
                Flag::Deleted => true,
                _ => false,
            });
            let is_seen = msg.flags().any(|flag| match flag {
                Flag::Seen => true,
                _ => false,
            });

            let flags = if is_seen { DC_IMAP_SEEN } else { 0 };

            if !is_deleted && msg.body().is_some() {
                let body = msg.body().unwrap_or_default();
                unsafe {
                    dc_receive_imf(context, &body, folder.as_ref(), server_uid, flags as u32);
                }
            }
        }

        1
    }

    pub fn idle(&self, context: &Context) -> Result<(), Error> {
        task::block_on(async move {
            ensure!(
                self.config.read().await.selected_folder.is_some(),
                "no folder selected, probably in teardown?"
            );

            if !self.config.read().await.can_idle {
                return Err(Error::ImapMissesIdle);
            }

            self.setup_handle_if_needed(context);

            let watch_folder = self.config.read().await.watch_folder.clone();
            match self.select_folder(context, watch_folder.as_ref()).await {
                ImapActionResult::Success | ImapActionResult::AlreadyDone => {}

                ImapActionResult::Failed | ImapActionResult::RetryLater => {
                    bail!("IMAP select failed for {:?}", watch_folder.as_ref());
                }
            }

            let session = self.session.lock().await.take();
            let timeout = Duration::from_secs(23 * 60);
            if let Some(session) = session {
                match session.idle() {
                    // BEWARE: If you change the Secure branch you
                    // typically also need to change the Insecure branch.
                    IdleHandle::Secure(mut handle) => {
                        if let Err(err) = handle.init().await {
                            bail!("IDLE init failed: {}", err);
                        }

                        let (idle_wait, interrupt) = handle.wait_with_timeout(timeout);
                        *self.interrupt.lock().await = Some(interrupt);

                        if self.skip_next_idle_wait.load(Ordering::Relaxed) {
                            // interrupt_idle has happened before we
                            // provided self.interrupt
                            self.skip_next_idle_wait.store(false, Ordering::Relaxed);
                            std::mem::drop(idle_wait);
                            info!(context, "Idle wait was skipped");
                        } else {
                            info!(context, "Idle entering wait-on-remote state");
                            let res = idle_wait.await;
                            info!(context, "Idle finished wait-on-remote: {:?}", res);
                        }
                        match handle.done().await {
                            Ok(session) => {
                                *self.session.lock().await = Some(Session::Secure(session));
                            }
                            Err(err) => {
                                // if we cannot terminate IDLE it probably
                                // means that we waited long (with idle_wait)
                                // but the network went away/changed
                                self.trigger_reconnect();
                                return Err(Error::ImapIdleProtocolFailed(format!("{}", err)));
                            }
                        }
                    }
                    IdleHandle::Insecure(mut handle) => {
                        if let Err(err) = handle.init().await {
                            bail!("IDLE init failed: {}", err);
                        }

                        let (idle_wait, interrupt) = handle.wait_with_timeout(timeout);
                        *self.interrupt.lock().await = Some(interrupt);

                        if self.skip_next_idle_wait.load(Ordering::Relaxed) {
                            // interrupt_idle has happened before we
                            // provided self.interrupt
                            self.skip_next_idle_wait.store(false, Ordering::Relaxed);
                            std::mem::drop(idle_wait);
                            info!(context, "Idle wait was skipped");
                        } else {
                            info!(context, "Idle entering wait-on-remote state");
                            let res = idle_wait.await;
                            info!(context, "Idle finished wait-on-remote: {:?}", res);
                        }
                        match handle.done().await {
                            Ok(session) => {
                                *self.session.lock().await = Some(Session::Insecure(session));
                            }
                            Err(err) => {
                                // if we cannot terminate IDLE it probably
                                // means that we waited long (with idle_wait)
                                // but the network went away/changed
                                self.trigger_reconnect();
                                return Err(Error::ImapIdleProtocolFailed(format!("{}", err)));
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    pub(crate) fn fake_idle(&self, context: &Context, poll_mode: IdlePollMode) {
        // Idle using polling.
        task::block_on(async move {
            let fake_idle_start_time = SystemTime::now();

            info!(context, "IMAP-fake-IDLEing...");

            let interrupt = stop_token::StopSource::new();

            // we use 1000 minutes if we are told to not try network
            // which can happen because the watch_folder is not defined
            // but clients are still calling us in a loop.
            // if we are to use network, we check every minute if there
            // is new mail -- TODO: make this more flexible
            let secs = match poll_mode {
                IdlePollMode::Never => 60000,
                IdlePollMode::Often => 60,
            };
            let interval = async_std::stream::interval(Duration::from_secs(secs));
            let mut interrupt_interval = interrupt.stop_token().stop_stream(interval);
            *self.interrupt.lock().await = Some(interrupt);

            // loop until we are interrupted or if we fetched something
            while let Some(_) = interrupt_interval.next().await {
                if poll_mode == IdlePollMode::Never {
                    continue;
                }
                if !self.is_connected().await {
                    // try to connect with proper login params
                    // (setup_handle_if_needed might not know about them if we
                    // never successfully connected)
                    match dc_connect_to_configured_imap(context, &self) {
                        Ok(()) => {}
                        Err(err) => {
                            warn!(context, "fake_idle: could not connect: {}", err);
                            continue;
                        }
                    }
                }
                // we are connected, let's see if fetching messages results
                // in anything.  If so, we behave as if IDLE had data but
                // will have already fetched the messages so perform_*_fetch
                // will not find any new.

                let watch_folder = self.config.read().await.watch_folder.clone();
                if let Some(watch_folder) = watch_folder {
                    if 0 != self.fetch_from_single_folder(context, watch_folder).await {
                        break;
                    }
                }
            }
            self.interrupt.lock().await.take();

            info!(
                context,
                "IMAP-fake-IDLE done after {:.4}s",
                SystemTime::now()
                    .duration_since(fake_idle_start_time)
                    .unwrap()
                    .as_millis() as f64
                    / 1000.,
            );
        })
    }

    pub fn interrupt_idle(&self) {
        task::block_on(async move {
            if self.interrupt.lock().await.take().is_none() {
                // idle wait is not running, signal it needs to skip
                self.skip_next_idle_wait.store(true, Ordering::Relaxed);

                // meanwhile idle-wait may have produced the interrupter
                let _ = self.interrupt.lock().await.take();
            }
        });
    }

    pub fn mv(
        &self,
        context: &Context,
        folder: &str,
        uid: u32,
        dest_folder: &str,
        dest_uid: &mut u32,
    ) -> ImapActionResult {
        task::block_on(async move {
            if folder == dest_folder {
                info!(
                    context,
                    "Skip moving message; message {}/{} is already in {}...",
                    folder,
                    uid,
                    dest_folder,
                );
                return ImapActionResult::AlreadyDone;
            }
            if let Some(imapresult) = self.prepare_imap_operation_on_msg(context, folder, uid) {
                return imapresult;
            }
            // we are connected, and the folder is selected

            // XXX Rust-Imap provides no target uid on mv, so just set it to 0
            *dest_uid = 0;

            let set = format!("{}", uid);
            let display_folder_id = format!("{}/{}", folder, uid);
            if let Some(ref mut session) = &mut *self.session.lock().await {
                match session.uid_mv(&set, &dest_folder).await {
                    Ok(_) => {
                        emit_event!(
                            context,
                            Event::ImapMessageMoved(format!(
                                "IMAP Message {} moved to {}",
                                display_folder_id, dest_folder
                            ))
                        );
                        return ImapActionResult::Success;
                    }
                    Err(err) => {
                        warn!(
                            context,
                            "Cannot move message, fallback to COPY/DELETE {}/{} to {}: {}",
                            folder,
                            uid,
                            dest_folder,
                            err
                        );
                    }
                }
            } else {
                unreachable!();
            };

            if let Some(ref mut session) = &mut *self.session.lock().await {
                match session.uid_copy(&set, &dest_folder).await {
                    Ok(_) => {
                        if !self.add_flag_finalized(context, uid, "\\Deleted").await {
                            warn!(context, "Cannot mark {} as \"Deleted\" after copy.", uid);
                            ImapActionResult::Failed
                        } else {
                            self.config.write().await.selected_folder_needs_expunge = true;
                            ImapActionResult::Success
                        }
                    }
                    Err(err) => {
                        warn!(context, "Could not copy message: {}", err);
                        ImapActionResult::Failed
                    }
                }
            } else {
                unreachable!();
            }
        })
    }

    async fn add_flag_finalized(&self, context: &Context, server_uid: u32, flag: &str) -> bool {
        // return true if we successfully set the flag or we otherwise
        // think add_flag should not be retried: Disconnection during setting
        // the flag, or other imap-errors, returns true as well.
        //
        // returning false means that the operation can be retried.
        if server_uid == 0 {
            return true; // might be moved but we don't want to have a stuck job
        }
        let s = server_uid.to_string();
        self.add_flag_finalized_with_set(context, &s, flag).await
    }

    async fn add_flag_finalized_with_set(
        &self,
        context: &Context,
        uid_set: &str,
        flag: &str,
    ) -> bool {
        if self.should_reconnect() {
            return false;
        }
        if let Some(ref mut session) = &mut *self.session.lock().await {
            let query = format!("+FLAGS ({})", flag);
            match session.uid_store(uid_set, &query).await {
                Ok(_) => {}
                Err(err) => {
                    warn!(
                        context,
                        "IMAP failed to store: ({}, {}) {:?}", uid_set, query, err
                    );
                }
            }
            true // we tried once, that's probably enough for setting flag
        } else {
            unreachable!();
        }
    }

    pub fn prepare_imap_operation_on_msg(
        &self,
        context: &Context,
        folder: &str,
        uid: u32,
    ) -> Option<ImapActionResult> {
        task::block_on(async move {
            if uid == 0 {
                return Some(ImapActionResult::Failed);
            } else if !self.is_connected().await {
                if let Err(err) = connect_to_inbox(context, &self) {
                    warn!(context, "prepare_imap_op failed: {}", err);
                    return Some(ImapActionResult::RetryLater);
                }
            }
            match self.select_folder(context, Some(&folder)).await {
                ImapActionResult::Success | ImapActionResult::AlreadyDone => None,
                res => {
                    warn!(
                        context,
                        "Cannot select folder {} for preparing IMAP operation", folder
                    );

                    Some(res)
                }
            }
        })
    }

    pub fn set_seen(&self, context: &Context, folder: &str, uid: u32) -> ImapActionResult {
        task::block_on(async move {
            if let Some(imapresult) = self.prepare_imap_operation_on_msg(context, folder, uid) {
                return imapresult;
            }
            // we are connected, and the folder is selected
            info!(context, "Marking message {}/{} as seen...", folder, uid,);

            if self.add_flag_finalized(context, uid, "\\Seen").await {
                ImapActionResult::Success
            } else {
                warn!(
                    context,
                    "Cannot mark message {} in folder {} as seen, ignoring.", uid, folder
                );
                ImapActionResult::Failed
            }
        })
    }

    // only returns 0 on connection problems; we should try later again in this case *
    pub fn delete_msg(
        &self,
        context: &Context,
        message_id: &str,
        folder: &str,
        uid: &mut u32,
    ) -> ImapActionResult {
        task::block_on(async move {
            if let Some(imapresult) = self.prepare_imap_operation_on_msg(context, folder, *uid) {
                return imapresult;
            }
            // we are connected, and the folder is selected

            let set = format!("{}", uid);
            let display_imap_id = format!("{}/{}", folder, uid);

            // double-check that we are deleting the correct message-id
            // this comes at the expense of another imap query
            if let Some(ref mut session) = &mut *self.session.lock().await {
                match session.uid_fetch(set, PREFETCH_FLAGS).await {
                    Ok(msgs) => {
                        if msgs.is_empty() {
                            warn!(
                                context,
                                "Cannot delete on IMAP, {}: imap entry gone '{}'",
                                display_imap_id,
                                message_id,
                            );
                            return ImapActionResult::Failed;
                        }
                        let remote_message_id =
                            prefetch_get_message_id(msgs.first().unwrap()).unwrap_or_default();

                        if remote_message_id != message_id {
                            warn!(
                                context,
                                "Cannot delete on IMAP, {}: remote message-id '{}' != '{}'",
                                display_imap_id,
                                remote_message_id,
                                message_id,
                            );
                        }
                        *uid = 0;
                    }
                    Err(err) => {
                        warn!(
                            context,
                            "Cannot delete {} on IMAP: {}", display_imap_id, err
                        );
                        *uid = 0;
                    }
                }
            }

            // mark the message for deletion
            if !self.add_flag_finalized(context, *uid, "\\Deleted").await {
                warn!(
                    context,
                    "Cannot mark message {} as \"Deleted\".", display_imap_id
                );
                ImapActionResult::Failed
            } else {
                emit_event!(
                    context,
                    Event::ImapMessageDeleted(format!(
                        "IMAP Message {} marked as deleted [{}]",
                        display_imap_id, message_id
                    ))
                );
                self.config.write().await.selected_folder_needs_expunge = true;
                ImapActionResult::Success
            }
        })
    }

    pub fn configure_folders(&self, context: &Context, flags: libc::c_int) {
        task::block_on(async move {
            if !self.is_connected().await {
                return;
            }

            info!(context, "Configuring IMAP-folders.");

            if let Some(ref mut session) = &mut *self.session.lock().await {
                let folders = self
                    .list_folders(session, context)
                    .await
                    .expect("no folders found");

                let sentbox_folder =
                    folders
                        .iter()
                        .find(|folder| match get_folder_meaning(folder) {
                            FolderMeaning::SentObjects => true,
                            _ => false,
                        });
                info!(context, "sentbox folder is {:?}", sentbox_folder);

                let delimiter = self.config.read().await.imap_delimiter;
                let fallback_folder = format!("INBOX{}DeltaChat", delimiter);

                let mut mvbox_folder = folders
                    .iter()
                    .find(|folder| folder.name() == "DeltaChat" || folder.name() == fallback_folder)
                    .map(|n| n.name().to_string());

                if mvbox_folder.is_none() && 0 != (flags as usize & DC_CREATE_MVBOX) {
                    info!(context, "Creating MVBOX-folder \"DeltaChat\"...",);

                    match session.create("DeltaChat").await {
                        Ok(_) => {
                            mvbox_folder = Some("DeltaChat".into());

                            info!(context, "MVBOX-folder created.",);
                        }
                        Err(err) => {
                            warn!(
                                context,
                                "Cannot create MVBOX-folder, using trying INBOX subfolder. ({})",
                                err
                            );

                            match session.create(&fallback_folder).await {
                                Ok(_) => {
                                    mvbox_folder = Some(fallback_folder);
                                    info!(
                                        context,
                                        "MVBOX-folder created as INBOX subfolder. ({})", err
                                    );
                                }
                                Err(err) => {
                                    warn!(context, "Cannot create MVBOX-folder. ({})", err);
                                }
                            }
                        }
                    }
                    // SUBSCRIBE is needed to make the folder visible to the LSUB command
                    // that may be used by other MUAs to list folders.
                    // for the LIST command, the folder is always visible.
                    if let Some(ref mvbox) = mvbox_folder {
                        // TODO: better error handling
                        session.subscribe(mvbox).await.expect("failed to subscribe");
                    }
                }

                context
                    .sql
                    .set_raw_config_int(context, "folders_configured", 3)
                    .ok();
                if let Some(ref mvbox_folder) = mvbox_folder {
                    context
                        .sql
                        .set_raw_config(context, "configured_mvbox_folder", Some(mvbox_folder))
                        .ok();
                }
                if let Some(ref sentbox_folder) = sentbox_folder {
                    context
                        .sql
                        .set_raw_config(
                            context,
                            "configured_sentbox_folder",
                            Some(sentbox_folder.name()),
                        )
                        .ok();
                }
            }
        })
    }

    async fn list_folders<'a>(
        &self,
        session: &'a mut Session,
        context: &Context,
    ) -> Option<Vec<Name>> {
        // TODO: use xlist when available
        match session.list(Some(""), Some("*")).await {
            Ok(list) => {
                if list.is_empty() {
                    warn!(context, "Folder list is empty.",);
                }
                Some(list)
            }
            Err(err) => {
                eprintln!("list error: {:?}", err);
                warn!(context, "Cannot get folder list.",);

                None
            }
        }
    }

    pub fn empty_folder(&self, context: &Context, folder: &str) {
        task::block_on(async move {
            info!(context, "emptying folder {}", folder);

            if folder.is_empty() {
                warn!(context, "cannot perform empty, folder not set");
                return;
            }
            match self.select_folder(context, Some(&folder)).await {
                ImapActionResult::Success | ImapActionResult::AlreadyDone => {
                    if !self
                        .add_flag_finalized_with_set(context, SELECT_ALL, "\\Deleted")
                        .await
                    {
                        warn!(context, "Cannot empty folder {}", folder);
                    } else {
                        // we now trigger expunge to actually delete messages
                        self.config.write().await.selected_folder_needs_expunge = true;
                        if self.select_folder::<String>(context, None).await
                            == ImapActionResult::Success
                        {
                            emit_event!(context, Event::ImapFolderEmptied(folder.to_string()));
                        } else {
                            warn!(
                                context,
                                "could not perform expunge on empty-marked folder {}", folder
                            );
                        }
                    }
                }
                ImapActionResult::Failed | ImapActionResult::RetryLater => {
                    warn!(context, "could not select folder {}", folder);
                }
            }
        });
    }
}

/// Try to get the folder meaning by the name of the folder only used if the server does not support XLIST.
// TODO: lots languages missing - maybe there is a list somewhere on other MUAs?
// however, if we fail to find out the sent-folder,
// only watching this folder is not working. at least, this is no show stopper.
// CAVE: if possible, take care not to add a name here that is "sent" in one language
// but sth. different in others - a hard job.
fn get_folder_meaning_by_name(folder_name: &Name) -> FolderMeaning {
    let sent_names = vec!["sent", "sent objects", "gesendet"];
    let lower = folder_name.name().to_lowercase();

    if sent_names.into_iter().find(|s| *s == lower).is_some() {
        FolderMeaning::SentObjects
    } else {
        FolderMeaning::Unknown
    }
}

fn get_folder_meaning(folder_name: &Name) -> FolderMeaning {
    if folder_name.attributes().is_empty() {
        return FolderMeaning::Unknown;
    }

    let mut res = FolderMeaning::Unknown;
    let special_names = vec!["\\Spam", "\\Trash", "\\Drafts", "\\Junk"];

    for attr in folder_name.attributes() {
        match attr {
            NameAttribute::Custom(ref label) => {
                if special_names.iter().find(|s| *s == label).is_some() {
                    res = FolderMeaning::Other;
                } else if label == "\\Sent" {
                    res = FolderMeaning::SentObjects
                }
            }
            _ => {}
        }
    }

    match res {
        FolderMeaning::Unknown => get_folder_meaning_by_name(folder_name),
        _ => res,
    }
}

fn precheck_imf(context: &Context, rfc724_mid: &str, server_folder: &str, server_uid: u32) -> bool {
    if let Ok((old_server_folder, old_server_uid, msg_id)) =
        message::rfc724_mid_exists(context, &rfc724_mid)
    {
        if old_server_folder.is_empty() && old_server_uid == 0 {
            info!(context, "[move] detected bbc-self {}", rfc724_mid,);
            job_add(
                context,
                Action::MarkseenMsgOnImap,
                msg_id.to_u32() as i32,
                Params::new(),
                0,
            );
        } else if old_server_folder != server_folder {
            info!(context, "[move] detected moved message {}", rfc724_mid,);
            update_msg_move_state(context, &rfc724_mid, MoveState::Stay);
        }

        if old_server_folder != server_folder || old_server_uid != server_uid {
            update_server_uid(context, &rfc724_mid, server_folder, server_uid);
        }
        true
    } else {
        false
    }
}

fn prefetch_get_message_id(prefetch_msg: &Fetch) -> Result<String, Error> {
    let message_id = prefetch_msg.envelope().unwrap().message_id.unwrap();
    wrapmime::parse_message_id(&message_id)
}
