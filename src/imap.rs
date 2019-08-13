use std::ffi::CString;
use std::net;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex, RwLock,
};
use std::time::{Duration, SystemTime};

use crate::constants::*;
use crate::context::Context;
use crate::dc_loginparam::*;
use crate::dc_tools::CStringExt;
use crate::oauth2::dc_get_oauth2_access_token;
use crate::types::*;

pub const DC_IMAP_SEEN: usize = 0x0001;
pub const DC_REGENERATE: usize = 0x01;

pub const DC_SUCCESS: usize = 3;
pub const DC_ALREADY_DONE: usize = 2;
pub const DC_RETRY_LATER: usize = 1;
pub const DC_FAILED: usize = 0;

const PREFETCH_FLAGS: &str = "(UID ENVELOPE)";
const BODY_FLAGS: &str = "(FLAGS BODY.PEEK[])";
const FETCH_FLAGS: &str = "(FLAGS)";

#[repr(C)]
pub struct Imap {
    config: Arc<RwLock<ImapConfig>>,
    watch: Arc<(Mutex<bool>, Condvar)>,

    get_config: dc_get_config_t,
    set_config: dc_set_config_t,
    precheck_imf: dc_precheck_imf_t,
    receive_imf: dc_receive_imf_t,

    session: Arc<Mutex<Option<Session>>>,
    stream: Arc<RwLock<Option<net::TcpStream>>>,
    connected: Arc<Mutex<bool>>,

    should_reconnect: AtomicBool,
}

struct OAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for OAuth2 {
    type Response = String;

    #[allow(unused_variables)]
    fn process(&self, data: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

#[derive(Debug)]
pub enum FolderMeaning {
    Unknown,
    SentObjects,
    Other,
}

pub enum Client {
    Secure(
        imap::Client<native_tls::TlsStream<net::TcpStream>>,
        net::TcpStream,
    ),
    Insecure(imap::Client<net::TcpStream>, net::TcpStream),
}

pub enum Session {
    Secure(imap::Session<native_tls::TlsStream<net::TcpStream>>),
    Insecure(imap::Session<net::TcpStream>),
}

pub enum IdleHandle<'a> {
    Secure(imap::extensions::idle::Handle<'a, native_tls::TlsStream<net::TcpStream>>),
    Insecure(imap::extensions::idle::Handle<'a, net::TcpStream>),
}

impl<'a> IdleHandle<'a> {
    pub fn set_keepalive(&mut self, interval: Duration) {
        match self {
            IdleHandle::Secure(i) => i.set_keepalive(interval),
            IdleHandle::Insecure(i) => i.set_keepalive(interval),
        }
    }

    pub fn wait_keepalive(self) -> imap::error::Result<()> {
        match self {
            IdleHandle::Secure(i) => i.wait_keepalive(),
            IdleHandle::Insecure(i) => i.wait_keepalive(),
        }
    }
}

impl Client {
    pub fn connect_secure<A: net::ToSocketAddrs, S: AsRef<str>>(
        addr: A,
        domain: S,
    ) -> imap::error::Result<Self> {
        let stream = net::TcpStream::connect(addr)?;
        let tls = native_tls::TlsConnector::builder()
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap();

        let s = stream.try_clone().expect("cloning the stream failed");
        let tls_stream = native_tls::TlsConnector::connect(&tls, domain.as_ref(), s)?;

        let client = imap::Client::new(tls_stream);
        // TODO: Read greeting

        Ok(Client::Secure(client, stream))
    }

    pub fn connect_insecure<A: net::ToSocketAddrs>(addr: A) -> imap::error::Result<Self> {
        let stream = net::TcpStream::connect(addr)?;

        let client = imap::Client::new(stream.try_clone().unwrap());
        // TODO: Read greeting

        Ok(Client::Insecure(client, stream))
    }

    pub fn secure<S: AsRef<str>>(self, domain: S) -> imap::error::Result<Client> {
        match self {
            Client::Insecure(client, stream) => {
                let tls = native_tls::TlsConnector::builder()
                    .danger_accept_invalid_hostnames(true)
                    .build()
                    .unwrap();

                let client_sec = client.secure(domain, &tls)?;

                Ok(Client::Secure(client_sec, stream))
            }
            // Nothing to do
            Client::Secure(_, _) => Ok(self),
        }
    }

    pub fn authenticate<A: imap::Authenticator, S: AsRef<str>>(
        self,
        auth_type: S,
        authenticator: &A,
    ) -> Result<(Session, net::TcpStream), (imap::error::Error, Client)> {
        match self {
            Client::Secure(i, stream) => match i.authenticate(auth_type, authenticator) {
                Ok(session) => Ok((Session::Secure(session), stream)),
                Err((err, c)) => Err((err, Client::Secure(c, stream))),
            },
            Client::Insecure(i, stream) => match i.authenticate(auth_type, authenticator) {
                Ok(session) => Ok((Session::Insecure(session), stream)),
                Err((err, c)) => Err((err, Client::Insecure(c, stream))),
            },
        }
    }

    pub fn login<U: AsRef<str>, P: AsRef<str>>(
        self,
        username: U,
        password: P,
    ) -> Result<(Session, net::TcpStream), (imap::error::Error, Client)> {
        match self {
            Client::Secure(i, stream) => match i.login(username, password) {
                Ok(session) => Ok((Session::Secure(session), stream)),
                Err((err, c)) => Err((err, Client::Secure(c, stream))),
            },
            Client::Insecure(i, stream) => match i.login(username, password) {
                Ok(session) => Ok((Session::Insecure(session), stream)),
                Err((err, c)) => Err((err, Client::Insecure(c, stream))),
            },
        }
    }
}

impl Session {
    pub fn capabilities(
        &mut self,
    ) -> imap::error::Result<imap::types::ZeroCopy<imap::types::Capabilities>> {
        match self {
            Session::Secure(i) => i.capabilities(),
            Session::Insecure(i) => i.capabilities(),
        }
    }

    pub fn list(
        &mut self,
        reference_name: Option<&str>,
        mailbox_pattern: Option<&str>,
    ) -> imap::error::Result<imap::types::ZeroCopy<Vec<imap::types::Name>>> {
        match self {
            Session::Secure(i) => i.list(reference_name, mailbox_pattern),
            Session::Insecure(i) => i.list(reference_name, mailbox_pattern),
        }
    }

    pub fn create<S: AsRef<str>>(&mut self, mailbox_name: S) -> imap::error::Result<()> {
        match self {
            Session::Secure(i) => i.subscribe(mailbox_name),
            Session::Insecure(i) => i.subscribe(mailbox_name),
        }
    }

    pub fn subscribe<S: AsRef<str>>(&mut self, mailbox: S) -> imap::error::Result<()> {
        match self {
            Session::Secure(i) => i.subscribe(mailbox),
            Session::Insecure(i) => i.subscribe(mailbox),
        }
    }

    pub fn close(&mut self) -> imap::error::Result<()> {
        match self {
            Session::Secure(i) => i.close(),
            Session::Insecure(i) => i.close(),
        }
    }

    pub fn select<S: AsRef<str>>(
        &mut self,
        mailbox_name: S,
    ) -> imap::error::Result<imap::types::Mailbox> {
        match self {
            Session::Secure(i) => i.select(mailbox_name),
            Session::Insecure(i) => i.select(mailbox_name),
        }
    }

    pub fn fetch<S1, S2>(
        &mut self,
        sequence_set: S1,
        query: S2,
    ) -> imap::error::Result<imap::types::ZeroCopy<Vec<imap::types::Fetch>>>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        match self {
            Session::Secure(i) => i.fetch(sequence_set, query),
            Session::Insecure(i) => i.fetch(sequence_set, query),
        }
    }

    pub fn uid_fetch<S1, S2>(
        &mut self,
        uid_set: S1,
        query: S2,
    ) -> imap::error::Result<imap::types::ZeroCopy<Vec<imap::types::Fetch>>>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        match self {
            Session::Secure(i) => i.uid_fetch(uid_set, query),
            Session::Insecure(i) => i.uid_fetch(uid_set, query),
        }
    }

    pub fn idle(&mut self) -> imap::error::Result<IdleHandle> {
        match self {
            Session::Secure(i) => i.idle().map(IdleHandle::Secure),
            Session::Insecure(i) => i.idle().map(IdleHandle::Insecure),
        }
    }

    pub fn uid_store<S1, S2>(
        &mut self,
        uid_set: S1,
        query: S2,
    ) -> imap::error::Result<imap::types::ZeroCopy<Vec<imap::types::Fetch>>>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        match self {
            Session::Secure(i) => i.uid_store(uid_set, query),
            Session::Insecure(i) => i.uid_store(uid_set, query),
        }
    }

    pub fn uid_mv<S1: AsRef<str>, S2: AsRef<str>>(
        &mut self,
        uid_set: S1,
        mailbox_name: S2,
    ) -> imap::error::Result<()> {
        match self {
            Session::Secure(i) => i.uid_mv(uid_set, mailbox_name),
            Session::Insecure(i) => i.uid_mv(uid_set, mailbox_name),
        }
    }

    pub fn uid_copy<S1: AsRef<str>, S2: AsRef<str>>(
        &mut self,
        uid_set: S1,
        mailbox_name: S2,
    ) -> imap::error::Result<()> {
        match self {
            Session::Secure(i) => i.uid_copy(uid_set, mailbox_name),
            Session::Insecure(i) => i.uid_copy(uid_set, mailbox_name),
        }
    }
}

pub struct ImapConfig {
    pub addr: String,
    pub imap_server: String,
    pub imap_port: u16,
    pub imap_user: String,
    pub imap_pw: String,
    pub server_flags: usize,
    pub selected_folder: Option<String>,
    pub selected_mailbox: Option<imap::types::Mailbox>,
    pub selected_folder_needs_expunge: bool,
    pub can_idle: bool,
    pub has_xlist: bool,
    pub imap_delimiter: char,
    pub watch_folder: Option<String>,
}

impl Default for ImapConfig {
    fn default() -> Self {
        let cfg = ImapConfig {
            addr: "".into(),
            imap_server: "".into(),
            imap_port: 0,
            imap_user: "".into(),
            imap_pw: "".into(),
            server_flags: 0,
            selected_folder: None,
            selected_mailbox: None,
            selected_folder_needs_expunge: false,
            can_idle: false,
            has_xlist: false,
            imap_delimiter: '.',
            watch_folder: None,
        };

        cfg
    }
}

impl Imap {
    pub fn new(
        get_config: dc_get_config_t,
        set_config: dc_set_config_t,
        precheck_imf: dc_precheck_imf_t,
        receive_imf: dc_receive_imf_t,
    ) -> Self {
        Imap {
            session: Arc::new(Mutex::new(None)),
            stream: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(ImapConfig::default())),
            watch: Arc::new((Mutex::new(false), Condvar::new())),
            get_config,
            set_config,
            precheck_imf,
            receive_imf,
            connected: Arc::new(Mutex::new(false)),
            should_reconnect: AtomicBool::new(false),
        }
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    pub fn should_reconnect(&self) -> bool {
        self.should_reconnect.load(Ordering::Relaxed)
    }

    fn setup_handle_if_needed(&self, context: &Context) -> bool {
        if self.config.read().unwrap().imap_server.is_empty() {
            return false;
        }

        if self.should_reconnect() {
            self.unsetup_handle(context);
        }

        if self.is_connected() && self.stream.read().unwrap().is_some() {
            self.should_reconnect.store(false, Ordering::Relaxed);
            return true;
        }

        let server_flags = self.config.read().unwrap().server_flags;

        let connection_res: imap::error::Result<Client> =
            if (server_flags & (DC_LP_IMAP_SOCKET_STARTTLS | DC_LP_IMAP_SOCKET_PLAIN)) != 0 {
                let config = self.config.read().unwrap();
                let imap_server: &str = config.imap_server.as_ref();
                let imap_port = config.imap_port;

                Client::connect_insecure((imap_server, imap_port)).and_then(|client| {
                    if (server_flags & DC_LP_IMAP_SOCKET_STARTTLS) != 0 {
                        client.secure(imap_server)
                    } else {
                        Ok(client)
                    }
                })
            } else {
                let config = self.config.read().unwrap();
                let imap_server: &str = config.imap_server.as_ref();
                let imap_port = config.imap_port;

                Client::connect_secure((imap_server, imap_port), imap_server)
            };

        let login_res = match connection_res {
            Ok(client) => {
                let config = self.config.read().unwrap();
                let imap_user: &str = config.imap_user.as_ref();
                let imap_pw: &str = config.imap_pw.as_ref();

                if (server_flags & DC_LP_AUTH_OAUTH2) != 0 {
                    let addr: &str = config.addr.as_ref();

                    if let Some(token) =
                        dc_get_oauth2_access_token(context, addr, imap_pw, DC_REGENERATE as usize)
                    {
                        let auth = OAuth2 {
                            user: imap_user.into(),
                            access_token: token,
                        };
                        client.authenticate("XOAUTH2", &auth)
                    } else {
                        return false;
                    }
                } else {
                    client.login(imap_user, imap_pw)
                }
            }
            Err(err) => {
                let config = self.config.read().unwrap();
                let imap_server: &str = config.imap_server.as_ref();
                let imap_port = config.imap_port;

                log_event!(
                    context,
                    Event::ERROR_NETWORK,
                    0,
                    "Could not connect to IMAP-server {}:{}. ({})",
                    imap_server,
                    imap_port,
                    err
                );

                return false;
            }
        };

        self.should_reconnect.store(false, Ordering::Relaxed);

        match login_res {
            Ok((session, stream)) => {
                *self.session.lock().unwrap() = Some(session);
                *self.stream.write().unwrap() = Some(stream);
                true
            }
            Err((err, _)) => {
                log_event!(context, Event::ERROR_NETWORK, 0, "Cannot login ({})", err);
                self.unsetup_handle(context);

                false
            }
        }
    }

    fn unsetup_handle(&self, context: &Context) {
        info!(context, 0, "IMAP unsetup_handle starts");

        info!(
            context,
            0, "IMAP unsetup_handle step 1 (closing down stream)."
        );
        let stream = self.stream.write().unwrap().take();
        if stream.is_some() {
            match stream.unwrap().shutdown(net::Shutdown::Both) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("failed to shutdown connection: {:?}", err);
                }
            }
        }
        info!(
            context,
            0, "IMAP unsetup_handle step 2 (acquiring session.lock)"
        );
        let session = self.session.lock().unwrap().take();
        if session.is_some() {
            match session.unwrap().close() {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("failed to close connection: {:?}", err);
                }
            }
        }

        info!(context, 0, "IMAP unsetup_handle step 3 (clearing config).");
        self.config.write().unwrap().selected_folder = None;
        self.config.write().unwrap().selected_mailbox = None;
        info!(context, 0, "IMAP unsetup_handle step 4 (disconnected).",);
    }

    fn free_connect_params(&self) {
        let mut cfg = self.config.write().unwrap();

        cfg.addr = "".into();
        cfg.imap_server = "".into();
        cfg.imap_user = "".into();
        cfg.imap_pw = "".into();
        cfg.imap_port = 0;

        cfg.can_idle = false;
        cfg.has_xlist = false;

        cfg.watch_folder = None;
    }

    pub fn connect(&self, context: &Context, lp: &dc_loginparam_t) -> bool {
        if lp.mail_server.is_empty() || lp.mail_user.is_empty() || lp.mail_pw.is_empty() {
            return false;
        }

        if self.is_connected() {
            return true;
        }

        {
            let addr = &lp.addr;
            let imap_server = &lp.mail_server;
            let imap_port = lp.mail_port as u16;
            let imap_user = &lp.mail_user;
            let imap_pw = &lp.mail_pw;
            let server_flags = lp.server_flags as usize;

            let mut config = self.config.write().unwrap();
            config.addr = addr.to_string();
            config.imap_server = imap_server.to_string();
            config.imap_port = imap_port;
            config.imap_user = imap_user.to_string();
            config.imap_pw = imap_pw.to_string();
            config.server_flags = server_flags;
        }

        if !self.setup_handle_if_needed(context) {
            self.free_connect_params();
            return false;
        }

        let (teardown, can_idle, has_xlist) = match &mut *self.session.lock().unwrap() {
            Some(ref mut session) => {
                if let Ok(caps) = session.capabilities() {
                    if !context.sql.is_open() {
                        warn!(context, 0, "IMAP-LOGIN as {} ok but ABORTING", lp.mail_user,);
                        (true, false, false)
                    } else {
                        let can_idle = caps.has("IDLE");
                        let has_xlist = caps.has("XLIST");
                        let caps_list = caps.iter().fold(String::new(), |mut s, c| {
                            s += " ";
                            s += c;
                            s
                        });
                        log_event!(
                            context,
                            Event::IMAP_CONNECTED,
                            0,
                            "IMAP-LOGIN as {}, capabilities: {}",
                            lp.mail_user,
                            caps_list,
                        );
                        (false, can_idle, has_xlist)
                    }
                } else {
                    (true, false, false)
                }
            }
            None => (true, false, false),
        };

        if teardown {
            self.unsetup_handle(context);
            self.free_connect_params();
            false
        } else {
            self.config.write().unwrap().can_idle = can_idle;
            self.config.write().unwrap().has_xlist = has_xlist;
            *self.connected.lock().unwrap() = true;
            true
        }
    }

    pub fn disconnect(&self, context: &Context) {
        if self.is_connected() {
            self.unsetup_handle(context);
            self.free_connect_params();
            *self.connected.lock().unwrap() = false;
        }
    }

    pub fn set_watch_folder(&self, watch_folder: String) {
        self.config.write().unwrap().watch_folder = Some(watch_folder);
    }

    pub fn fetch(&self, context: &Context) -> libc::c_int {
        if !self.is_connected() || !context.sql.is_open() {
            return 0;
        }

        self.setup_handle_if_needed(context);

        let watch_folder = self.config.read().unwrap().watch_folder.to_owned();

        if let Some(ref watch_folder) = watch_folder {
            // as during the fetch commands, new messages may arrive, we fetch until we do not
            // get any more. if IDLE is called directly after, there is only a small chance that
            // messages are missed and delayed until the next IDLE call
            loop {
                if self.fetch_from_single_folder(context, watch_folder) == 0 {
                    break;
                }
            }
            1
        } else {
            0
        }
    }

    fn select_folder<S: AsRef<str>>(&self, context: &Context, folder: Option<S>) -> usize {
        if self.session.lock().unwrap().is_none() {
            let mut cfg = self.config.write().unwrap();
            cfg.selected_folder = None;
            cfg.selected_folder_needs_expunge = false;
            return 0;
        }

        // if there is a new folder and the new folder is equal to the selected one, there's nothing to do.
        // if there is _no_ new folder, we continue as we might want to expunge below.
        if let Some(ref folder) = folder {
            if let Some(ref selected_folder) = self.config.read().unwrap().selected_folder {
                if folder.as_ref() == selected_folder {
                    return 1;
                }
            }
        }

        // deselect existing folder, if needed (it's also done implicitly by SELECT, however, without EXPUNGE then)
        if self.config.read().unwrap().selected_folder_needs_expunge {
            if let Some(ref folder) = self.config.read().unwrap().selected_folder {
                info!(context, 0, "Expunge messages in \"{}\".", folder);

                // A CLOSE-SELECT is considerably faster than an EXPUNGE-SELECT, see
                // https://tools.ietf.org/html/rfc3501#section-6.4.2
                if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                    match session.close() {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("failed to close session: {:?}", err);
                        }
                    }
                } else {
                    return 0;
                }
                self.config.write().unwrap().selected_folder_needs_expunge = true;
            }
        }

        // select new folder
        if let Some(ref folder) = folder {
            if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                match session.select(folder) {
                    Ok(mailbox) => {
                        let mut config = self.config.write().unwrap();
                        config.selected_folder = Some(folder.as_ref().to_string());
                        config.selected_mailbox = Some(mailbox);
                    }
                    Err(err) => {
                        info!(
                            context,
                            0,
                            "Cannot select folder: {}; {:?}.",
                            folder.as_ref(),
                            err
                        );

                        self.config.write().unwrap().selected_folder = None;
                        self.should_reconnect.store(true, Ordering::Relaxed);
                        return 0;
                    }
                }
            } else {
                return 0;
            }
        }

        1
    }

    fn get_config_last_seen_uid<S: AsRef<str>>(&self, context: &Context, folder: S) -> (u32, u32) {
        let key = format!("imap.mailbox.{}", folder.as_ref());
        if let Some(entry) = (self.get_config)(context, &key) {
            // the entry has the format `imap.mailbox.<folder>=<uidvalidity>:<lastseenuid>`
            let mut parts = entry.split(':');
            (
                parts.next().unwrap().parse().unwrap_or_else(|_| 0),
                parts.next().unwrap().parse().unwrap_or_else(|_| 0),
            )
        } else {
            (0, 0)
        }
    }

    fn fetch_from_single_folder<S: AsRef<str>>(&self, context: &Context, folder: S) -> usize {
        if !self.is_connected() {
            info!(
                context,
                0,
                "Cannot fetch from \"{}\" - not connected.",
                folder.as_ref()
            );

            return 0;
        }

        if self.select_folder(context, Some(&folder)) == 0 {
            info!(
                context,
                0,
                "Cannot select folder \"{}\" for fetching.",
                folder.as_ref()
            );

            return 0;
        }

        // compare last seen UIDVALIDITY against the current one
        let (mut uid_validity, mut last_seen_uid) = self.get_config_last_seen_uid(context, &folder);

        let config = self.config.read().unwrap();
        let mailbox = config.selected_mailbox.as_ref().expect("just selected");

        if mailbox.uid_validity.is_none() {
            error!(
                context,
                0,
                "Cannot get UIDVALIDITY for folder \"{}\".",
                folder.as_ref(),
            );

            return 0;
        }

        if mailbox.uid_validity.unwrap() != uid_validity {
            // first time this folder is selected or UIDVALIDITY has changed, init lastseenuid and save it to config

            if mailbox.exists == 0 {
                info!(context, 0, "Folder \"{}\" is empty.", folder.as_ref());

                // set lastseenuid=0 for empty folders.
                // id we do not do this here, we'll miss the first message
                // as we will get in here again and fetch from lastseenuid+1 then

                self.set_config_last_seen_uid(context, &folder, mailbox.uid_validity.unwrap(), 0);
                return 0;
            }

            let list = if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                // `FETCH <message sequence number> (UID)`
                let set = format!("{}", mailbox.exists);
                match session.fetch(set, PREFETCH_FLAGS) {
                    Ok(list) => list,
                    Err(_err) => {
                        self.should_reconnect.store(true, Ordering::Relaxed);
                        info!(
                            context,
                            0,
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

            uid_validity = mailbox.uid_validity.unwrap();
            self.set_config_last_seen_uid(context, &folder, uid_validity, last_seen_uid);
            info!(
                context,
                0,
                "lastseenuid initialized to {} for {}@{}",
                last_seen_uid,
                folder.as_ref(),
                uid_validity,
            );
        }

        let mut read_cnt = 0;
        let mut read_errors = 0;
        let mut new_last_seen_uid = 0;

        let list = if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
            // fetch messages with larger UID than the last one seen
            // (`UID FETCH lastseenuid+1:*)`, see RFC 4549
            let set = format!("{}:*", last_seen_uid + 1);
            match session.uid_fetch(set, PREFETCH_FLAGS) {
                Ok(list) => list,
                Err(err) => {
                    warn!(context, 0, "failed to fetch uids: {}", err);
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

                let message_id = msg
                    .envelope()
                    .expect("missing envelope")
                    .message_id
                    .expect("missing message id");

                if 0 == unsafe {
                    let message_id_c = CString::yolo(message_id);
                    (self.precheck_imf)(context, message_id_c.as_ptr(), folder.as_ref(), cur_uid)
                } {
                    // check passed, go fetch the rest
                    if self.fetch_single_msg(context, &folder, cur_uid) == 0 {
                        info!(
                            context,
                            0,
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
                        0,
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
                0,
                "{} mails read from \"{}\" with {} errors.",
                read_cnt,
                folder.as_ref(),
                read_errors
            );
        } else {
            info!(
                context,
                0,
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

        (self.set_config)(context, &key, Some(&val));
    }

    fn fetch_single_msg<S: AsRef<str>>(
        &self,
        context: &Context,
        folder: S,
        server_uid: u32,
    ) -> usize {
        // the function returns:
        // 0  the caller should try over again later
        // or  1  if the messages should be treated as received, the caller should not try to read the message again (even if no database entries are returned)
        if !self.is_connected() {
            return 0;
        }

        let set = format!("{}", server_uid);

        let msgs = if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
            match session.uid_fetch(set, BODY_FLAGS) {
                Ok(msgs) => msgs,
                Err(err) => {
                    self.should_reconnect.store(true, Ordering::Relaxed);
                    warn!(
                        context,
                        0,
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
                0,
                "Message #{} does not exist in folder \"{}\".",
                server_uid,
                folder.as_ref()
            );
        } else {
            let msg = &msgs[0];

            let is_deleted = msg
                .flags()
                .iter()
                .find(|flag| match flag {
                    imap::types::Flag::Deleted => true,
                    _ => false,
                })
                .is_some();
            let is_seen = msg
                .flags()
                .iter()
                .find(|flag| match flag {
                    imap::types::Flag::Seen => true,
                    _ => false,
                })
                .is_some();

            let flags = if is_seen { DC_IMAP_SEEN } else { 0 };

            if !is_deleted && msg.body().is_some() {
                let body = msg.body().unwrap();
                unsafe {
                    (self.receive_imf)(
                        context,
                        body.as_ptr() as *const libc::c_char,
                        body.len(),
                        folder.as_ref(),
                        server_uid,
                        flags as u32,
                    );
                }
            }
        }

        1
    }

    pub fn idle(&self, context: &Context) {
        if !self.config.read().unwrap().can_idle {
            return self.fake_idle(context);
        }

        self.setup_handle_if_needed(context);

        let watch_folder = self.config.read().unwrap().watch_folder.clone();
        if self.select_folder(context, watch_folder.as_ref()) == 0 {
            warn!(context, 0, "IMAP-IDLE not setup.",);

            return self.fake_idle(context);
        }

        let session = self.session.clone();
        let mut worker = Some({
            let (sender, receiver) = std::sync::mpsc::channel();
            let v = self.watch.clone();

            info!(context, 0, "IMAP-IDLE SPAWNING");
            std::thread::spawn(move || {
                let &(ref lock, ref cvar) = &*v;
                if let Some(ref mut session) = &mut *session.lock().unwrap() {
                    let mut idle = match session.idle() {
                        Ok(idle) => idle,
                        Err(err) => {
                            eprintln!("failed to setup idle: {:?}", err);
                            return;
                        }
                    };

                    // most servers do not allow more than ~28 minutes; stay clearly below that.
                    // a good value that is also used by other MUAs is 23 minutes.
                    // if needed, the ui can call dc_imap_interrupt_idle() to trigger a reconnect.
                    idle.set_keepalive(Duration::from_secs(23 * 60));
                    let res = idle.wait_keepalive();

                    // Ignoring the error, as this happens when we try sending after the drop
                    let _send_res = sender.send(res);

                    // Trigger condvar
                    let mut watch = lock.lock().unwrap();
                    *watch = true;
                    cvar.notify_one();
                }
            });
            receiver
        });

        let &(ref lock, ref cvar) = &*self.watch.clone();
        let mut watch = lock.lock().unwrap();

        let handle_res = |res| match res {
            Ok(()) => {
                info!(context, 0, "IMAP-IDLE has data.");
            }
            Err(err) => match err {
                imap::error::Error::ConnectionLost => {
                    info!(
                        context,
                        0, "IMAP-IDLE wait cancelled, we will reconnect soon."
                    );
                    self.should_reconnect.store(true, Ordering::Relaxed);
                }
                _ => {
                    warn!(context, 0, "IMAP-IDLE returns unknown value: {}", err);
                }
            },
        };

        loop {
            if let Ok(res) = worker.as_ref().unwrap().try_recv() {
                handle_res(res);
                break;
            } else {
                let res = cvar.wait(watch).unwrap();
                watch = res;
                if *watch {
                    if let Ok(res) = worker.as_ref().unwrap().try_recv() {
                        handle_res(res);
                    } else {
                        info!(context, 0, "IMAP-IDLE interrupted");
                    }

                    drop(worker.take());
                    break;
                }
            }
        }

        *watch = false;
    }

    fn fake_idle(&self, context: &Context) {
        // Idle using timeouts. This is also needed if we're not yet configured -
        // in this case, we're waiting for a configure job
        let fake_idle_start_time = SystemTime::now();
        let mut wait_long = false;

        info!(context, 0, "IMAP-fake-IDLEing...");

        let mut do_fake_idle = true;
        while do_fake_idle {
            // wait a moment: every 5 seconds in the first 3 minutes after a new message, after that every 60 seconds.
            let seconds_to_wait =
                if fake_idle_start_time.elapsed().unwrap() < Duration::new(3 * 60, 0) && !wait_long
                {
                    Duration::new(5, 0)
                } else {
                    Duration::new(60, 0)
                };

            let &(ref lock, ref cvar) = &*self.watch.clone();
            let mut watch = lock.lock().unwrap();

            loop {
                let res = cvar.wait_timeout(watch, seconds_to_wait).unwrap();
                watch = res.0;
                if *watch {
                    do_fake_idle = false;
                }
                if *watch || res.1.timed_out() {
                    break;
                }
            }

            *watch = false;

            if !do_fake_idle {
                return;
            }

            // check for new messages. fetch_from_single_folder() has the side-effect that messages
            // are also downloaded, however, typically this would take place in the FETCH command
            // following IDLE otherwise, so this seems okay here.
            if self.setup_handle_if_needed(context) {
                if let Some(ref watch_folder) = self.config.read().unwrap().watch_folder {
                    if 0 != self.fetch_from_single_folder(context, watch_folder) {
                        do_fake_idle = false;
                    }
                }
            } else {
                // if we cannot connect, set the starting time to a small value which will
                // result in larger timeouts (60 instead of 5 seconds) for re-checking the availablility of network.
                // to get the _exact_ moment of re-available network, the ui should call interrupt_idle()
                wait_long = true;
            }
        }
    }

    pub fn interrupt_idle(&self) {
        // interrupt idle
        let &(ref lock, ref cvar) = &*self.watch.clone();
        let mut watch = lock.lock().unwrap();

        *watch = true;
        cvar.notify_one();
    }

    pub fn mv<S1: AsRef<str>, S2: AsRef<str>>(
        &self,
        context: &Context,
        folder: S1,
        uid: u32,
        dest_folder: S2,
        dest_uid: &mut u32,
    ) -> usize {
        let mut res = DC_RETRY_LATER;
        let set = format!("{}", uid);

        if uid == 0 {
            res = DC_FAILED;
        } else if folder.as_ref() == dest_folder.as_ref() {
            info!(
                context,
                0,
                "Skip moving message; message {}/{} is already in {}...",
                folder.as_ref(),
                uid,
                dest_folder.as_ref()
            );

            res = DC_ALREADY_DONE;
        } else {
            info!(
                context,
                0,
                "Moving message {}/{} to {}...",
                folder.as_ref(),
                uid,
                dest_folder.as_ref()
            );

            if self.select_folder(context, Some(folder.as_ref())) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder {} for moving message.",
                    folder.as_ref()
                );
            } else {
                let moved = if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                    match session.uid_mv(&set, &dest_folder) {
                        Ok(_) => {
                            res = DC_SUCCESS;
                            true
                        }
                        Err(err) => {
                            info!(
                                context,
                                0,
                                "Cannot move message, fallback to COPY/DELETE {}/{} to {}: {}",
                                folder.as_ref(),
                                uid,
                                dest_folder.as_ref(),
                                err
                            );

                            false
                        }
                    }
                } else {
                    unreachable!();
                };

                if !moved {
                    let copied = if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                        match session.uid_copy(&set, &dest_folder) {
                            Ok(_) => true,
                            Err(err) => {
                                eprintln!("error copy: {:?}", err);
                                info!(context, 0, "Cannot copy message.",);

                                false
                            }
                        }
                    } else {
                        unreachable!();
                    };

                    if copied {
                        if self.add_flag(context, uid, "\\Deleted") == 0 {
                            warn!(context, 0, "Cannot mark message as \"Deleted\".",);
                        }
                        self.config.write().unwrap().selected_folder_needs_expunge = true;
                        res = DC_SUCCESS;
                    }
                }
            }
        }

        if res == DC_SUCCESS {
            // TODO: is this correct?
            *dest_uid = uid;
        }

        if res == DC_RETRY_LATER {
            if self.should_reconnect() {
                DC_RETRY_LATER
            } else {
                DC_FAILED
            }
        } else {
            res
        }
    }

    fn add_flag<S: AsRef<str>>(&self, context: &Context, server_uid: u32, flag: S) -> usize {
        if server_uid == 0 {
            return 0;
        }
        if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
            let set = format!("{}", server_uid);
            let query = format!("+FLAGS ({})", flag.as_ref());
            match session.uid_store(&set, &query) {
                Ok(_) => {}
                Err(err) => {
                    warn!(
                        context,
                        0, "IMAP failed to store: ({}, {}) {:?}", set, query, err
                    );
                }
            }
        }

        // All non-connection states are treated as success - the mail may
        // already be deleted or moved away on the server.
        if self.should_reconnect() {
            0
        } else {
            1
        }
    }

    pub fn set_seen<S: AsRef<str>>(&self, context: &Context, folder: S, uid: u32) -> usize {
        let mut res = DC_RETRY_LATER;

        if uid == 0 {
            res = DC_FAILED
        } else if self.is_connected() {
            info!(
                context,
                0,
                "Marking message {}/{} as seen...",
                folder.as_ref(),
                uid,
            );

            if self.select_folder(context, Some(folder.as_ref())) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder {} for setting SEEN flag.",
                    folder.as_ref(),
                );
            } else if self.add_flag(context, uid, "\\Seen") == 0 {
                warn!(context, 0, "Cannot mark message as seen.",);
            } else {
                res = DC_SUCCESS
            }
        }

        if res == DC_RETRY_LATER {
            if self.should_reconnect() {
                DC_RETRY_LATER
            } else {
                DC_FAILED
            }
        } else {
            res
        }
    }

    pub fn set_mdnsent<S: AsRef<str>>(&self, context: &Context, folder: S, uid: u32) -> usize {
        // returns 0=job should be retried later, 1=job done, 2=job done and flag just set
        let mut res = DC_RETRY_LATER;
        let set = format!("{}", uid);

        if uid == 0 {
            res = DC_FAILED;
        } else if self.is_connected() {
            info!(
                context,
                0,
                "Marking message {}/{} as $MDNSent...",
                folder.as_ref(),
                uid,
            );

            if self.select_folder(context, Some(folder.as_ref())) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder {} for setting $MDNSent flag.",
                    folder.as_ref()
                );
            } else {
                // Check if the folder can handle the `$MDNSent` flag (see RFC 3503).  If so, and not
                // set: set the flags and return this information.
                // If the folder cannot handle the `$MDNSent` flag, we risk duplicated MDNs; it's up
                // to the receiving MUA to handle this then (eg. Delta Chat has no problem with this).

                let can_create_flag = self
                    .config
                    .read()
                    .unwrap()
                    .selected_mailbox
                    .as_ref()
                    .map(|mbox| {
                        // empty means, everything can be stored
                        mbox.permanent_flags.is_empty()
                            || mbox
                                .permanent_flags
                                .iter()
                                .find(|flag| match flag {
                                    imap::types::Flag::Custom(s) => s == "$MDNSent",
                                    _ => false,
                                })
                                .is_some()
                    })
                    .expect("just selected folder");

                if can_create_flag {
                    let fetched_msgs =
                        if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                            match session.uid_fetch(set, FETCH_FLAGS) {
                                Ok(res) => Some(res),
                                Err(err) => {
                                    eprintln!("fetch error: {:?}", err);
                                    None
                                }
                            }
                        } else {
                            unreachable!();
                        };

                    if let Some(msgs) = fetched_msgs {
                        let flag_set = msgs
                            .first()
                            .map(|msg| {
                                msg.flags()
                                    .iter()
                                    .find(|flag| match flag {
                                        imap::types::Flag::Custom(s) => s == "$MDNSent",
                                        _ => false,
                                    })
                                    .is_some()
                            })
                            .unwrap_or_else(|| false);

                        res = if flag_set {
                            DC_ALREADY_DONE
                        } else if self.add_flag(context, uid, "$MDNSent") != 0 {
                            DC_SUCCESS
                        } else {
                            res
                        };

                        if res == DC_SUCCESS {
                            info!(context, 0, "$MDNSent just set and MDN will be sent.");
                        } else {
                            info!(context, 0, "$MDNSent already set and MDN already sent.");
                        }
                    }
                } else {
                    res = DC_SUCCESS;
                    info!(
                        context,
                        0, "Cannot store $MDNSent flags, risk sending duplicate MDN.",
                    );
                }
            }
        }

        if res == DC_RETRY_LATER {
            if self.should_reconnect() {
                DC_RETRY_LATER
            } else {
                DC_FAILED
            }
        } else {
            res
        }
    }

    // only returns 0 on connection problems; we should try later again in this case *
    pub fn delete_msg<S1: AsRef<str>, S2: AsRef<str>>(
        &self,
        context: &Context,
        message_id: S1,
        folder: S2,
        server_uid: &mut u32,
    ) -> usize {
        let mut success = false;
        if *server_uid == 0 {
            success = true
        } else {
            info!(
                context,
                0,
                "Marking message \"{}\", {}/{} for deletion...",
                message_id.as_ref(),
                folder.as_ref(),
                server_uid,
            );

            if self.select_folder(context, Some(&folder)) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder {} for deleting message.",
                    folder.as_ref()
                );
            } else {
                let set = format!("{}", server_uid);
                if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                    match session.uid_fetch(set, PREFETCH_FLAGS) {
                        Ok(msgs) => {
                            if msgs.is_empty()
                                || msgs
                                    .first()
                                    .unwrap()
                                    .envelope()
                                    .expect("missing envelope")
                                    .message_id
                                    .expect("missing message id")
                                    != message_id.as_ref()
                            {
                                warn!(
                                    context,
                                    0,
                                    "Cannot delete on IMAP, {}/{} does not match {}.",
                                    folder.as_ref(),
                                    server_uid,
                                    message_id.as_ref(),
                                );
                                *server_uid = 0;
                            }
                        }
                        Err(err) => {
                            eprintln!("fetch error: {:?}", err);

                            warn!(
                                context,
                                0,
                                "Cannot delete on IMAP, {}/{} not found.",
                                folder.as_ref(),
                                server_uid,
                            );
                            *server_uid = 0;
                        }
                    }
                }

                // mark the message for deletion
                if self.add_flag(context, *server_uid, "\\Deleted") == 0 {
                    warn!(context, 0, "Cannot mark message as \"Deleted\".");
                } else {
                    self.config.write().unwrap().selected_folder_needs_expunge = true;
                    success = true
                }
            }
        }

        if success {
            1
        } else {
            self.is_connected() as usize
        }
    }

    pub fn configure_folders(&self, context: &Context, flags: libc::c_int) {
        if !self.is_connected() {
            return;
        }

        info!(context, 0, "Configuring IMAP-folders.");

        let folders = self.list_folders(context).unwrap();
        let delimiter = self.config.read().unwrap().imap_delimiter;
        let fallback_folder = format!("INBOX{}DeltaChat", delimiter);

        let mut mvbox_folder = folders
            .iter()
            .find(|folder| folder.name() == "DeltaChat" || folder.name() == fallback_folder)
            .map(|n| n.name().to_string());

        let sentbox_folder = folders
            .iter()
            .find(|folder| match get_folder_meaning(folder) {
                FolderMeaning::SentObjects => true,
                _ => false,
            });

        if mvbox_folder.is_none() && 0 != (flags as usize & DC_CREATE_MVBOX) {
            info!(context, 0, "Creating MVBOX-folder \"DeltaChat\"...",);

            if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
                match session.create("DeltaChat") {
                    Ok(_) => {
                        mvbox_folder = Some("DeltaChat".into());

                        info!(context, 0, "MVBOX-folder created.",);
                    }
                    Err(err) => {
                        eprintln!("create error: {:?}", err);
                        warn!(
                            context,
                            0, "Cannot create MVBOX-folder, using trying INBOX subfolder."
                        );

                        match session.create(&fallback_folder) {
                            Ok(_) => {
                                mvbox_folder = Some(fallback_folder);
                                info!(context, 0, "MVBOX-folder created as INBOX subfolder.",);
                            }
                            Err(err) => {
                                eprintln!("create error: {:?}", err);
                                warn!(context, 0, "Cannot create MVBOX-folder.",);
                            }
                        }
                    }
                }
                // SUBSCRIBE is needed to make the folder visible to the LSUB command
                // that may be used by other MUAs to list folders.
                // for the LIST command, the folder is always visible.
                if let Some(ref mvbox) = mvbox_folder {
                    // TODO: better error handling
                    session.subscribe(mvbox).expect("failed to subscribe");
                }
            }
        }

        context
            .sql
            .set_config_int(context, "folders_configured", 3)
            .ok();
        if let Some(ref mvbox_folder) = mvbox_folder {
            context
                .sql
                .set_config(context, "configured_mvbox_folder", Some(mvbox_folder))
                .ok();
        }
        if let Some(ref sentbox_folder) = sentbox_folder {
            context
                .sql
                .set_config(
                    context,
                    "configured_sentbox_folder",
                    Some(sentbox_folder.name()),
                )
                .ok();
        }
    }

    fn list_folders(
        &self,
        context: &Context,
    ) -> Option<imap::types::ZeroCopy<Vec<imap::types::Name>>> {
        if let Some(ref mut session) = &mut *self.session.lock().unwrap() {
            // TODO: use xlist when available
            match session.list(Some(""), Some("*")) {
                Ok(list) => {
                    if list.is_empty() {
                        warn!(context, 0, "Folder list is empty.",);
                    }
                    Some(list)
                }
                Err(err) => {
                    eprintln!("list error: {:?}", err);
                    warn!(context, 0, "Cannot get folder list.",);

                    None
                }
            }
        } else {
            None
        }
    }
}

/// Try to get the folder meaning by the name of the folder only used if the server does not support XLIST.
// TODO: lots languages missing - maybe there is a list somewhere on other MUAs?
// however, if we fail to find out the sent-folder,
// only watching this folder is not working. at least, this is no show stopper.
// CAVE: if possible, take care not to add a name here that is "sent" in one language
// but sth. different in others - a hard job.
fn get_folder_meaning_by_name(folder_name: &imap::types::Name) -> FolderMeaning {
    let sent_names = vec!["sent", "sent objects", "gesendet"];
    let lower = folder_name.name().to_lowercase();

    if sent_names.into_iter().find(|s| *s == lower).is_some() {
        FolderMeaning::SentObjects
    } else {
        FolderMeaning::Unknown
    }
}

fn get_folder_meaning(folder_name: &imap::types::Name) -> FolderMeaning {
    if folder_name.attributes().is_empty() {
        return FolderMeaning::Unknown;
    }

    let mut res = FolderMeaning::Unknown;
    let special_names = vec!["\\Spam", "\\Trash", "\\Drafts", "\\Junk"];

    for attr in folder_name.attributes() {
        match attr {
            imap::types::NameAttribute::Custom(ref label) => {
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
