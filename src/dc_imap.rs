use std::ffi::{CStr, CString};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, SystemTime};

use crate::constants::*;
use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_sqlite3::*;
use crate::types::*;

pub const DC_IMAP_SEEN: usize = 0x0001;

pub const DC_SUCCESS: usize = 3;
pub const DC_ALREADY_DONE: usize = 2;
pub const DC_RETRY_LATER: usize = 1;
pub const DC_FAILED: usize = 0;

const PREFETCH_FLAGS: &'static str = "(UID ENVELOPE)";
const BODY_FLAGS: &'static str = "(FLAGS BODY.PEEK[])";
const FETCH_FLAGS: &'static str = "(FLAGS)";

#[repr(C)]
pub struct Imap {
    config: Arc<RwLock<ImapConfig>>,
    watch: Arc<(Mutex<bool>, Condvar)>,

    get_config: dc_get_config_t,
    set_config: dc_set_config_t,
    precheck_imf: dc_precheck_imf_t,
    receive_imf: dc_receive_imf_t,

    session: Arc<Mutex<Option<Session>>>,
    // idle: Arc<Mutex<Option<RentSession>>>,
}

// rental! {
//     pub mod rent {
//         use crate::dc_imap::{Session, IdleHandle};

//         #[rental_mut]
//         pub struct RentSession {
//             session: Box<Session>,
//             idle: IdleHandle<'session>,
//         }
//     }
// }

// use rent::*;

#[derive(Debug)]
pub enum FolderMeaning {
    Unknown,
    SentObjects,
    Other,
}

pub enum Client {
    Secure(imap::Client<native_tls::TlsStream<std::net::TcpStream>>),
    Insecure(imap::Client<std::net::TcpStream>),
}

pub enum Session {
    Secure(imap::Session<native_tls::TlsStream<std::net::TcpStream>>),
    Insecure(imap::Session<std::net::TcpStream>),
}

pub enum IdleHandle<'a> {
    Secure(imap::extensions::idle::Handle<'a, native_tls::TlsStream<std::net::TcpStream>>),
    Insecure(imap::extensions::idle::Handle<'a, std::net::TcpStream>),
}

impl From<imap::Client<native_tls::TlsStream<std::net::TcpStream>>> for Client {
    fn from(client: imap::Client<native_tls::TlsStream<std::net::TcpStream>>) -> Self {
        Client::Secure(client)
    }
}

impl From<imap::Client<std::net::TcpStream>> for Client {
    fn from(client: imap::Client<std::net::TcpStream>) -> Self {
        Client::Insecure(client)
    }
}

impl From<imap::Session<native_tls::TlsStream<std::net::TcpStream>>> for Session {
    fn from(session: imap::Session<native_tls::TlsStream<std::net::TcpStream>>) -> Self {
        Session::Secure(session)
    }
}

impl From<imap::Session<std::net::TcpStream>> for Session {
    fn from(session: imap::Session<std::net::TcpStream>) -> Self {
        Session::Insecure(session)
    }
}

impl<'a> From<imap::extensions::idle::Handle<'a, native_tls::TlsStream<std::net::TcpStream>>>
    for IdleHandle<'a>
{
    fn from(
        handle: imap::extensions::idle::Handle<'a, native_tls::TlsStream<std::net::TcpStream>>,
    ) -> Self {
        IdleHandle::Secure(handle)
    }
}

impl<'a> From<imap::extensions::idle::Handle<'a, std::net::TcpStream>> for IdleHandle<'a> {
    fn from(handle: imap::extensions::idle::Handle<'a, std::net::TcpStream>) -> Self {
        IdleHandle::Insecure(handle)
    }
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
    pub fn login<U: AsRef<str>, P: AsRef<str>>(
        self,
        username: U,
        password: P,
    ) -> Result<Session, (imap::error::Error, Client)> {
        match self {
            Client::Secure(i) => i
                .login(username, password)
                .map(Into::into)
                .map_err(|(e, c)| (e, c.into())),
            Client::Insecure(i) => i
                .login(username, password)
                .map(Into::into)
                .map_err(|(e, c)| (e, c.into())),
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
            Session::Secure(i) => i.idle().map(Into::into),
            Session::Insecure(i) => i.idle().map(Into::into),
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
    pub addr: Option<String>,
    pub imap_server: Option<String>,
    pub imap_port: Option<usize>,
    pub imap_user: Option<String>,
    pub imap_pw: Option<String>,
    pub server_flags: Option<usize>,
    pub selected_folder: Option<String>,
    pub selected_mailbox: Option<imap::types::Mailbox>,
    pub selected_folder_needs_expunge: bool,
    pub should_reconnect: bool,
    pub can_idle: bool,
    pub has_xlist: bool,
    pub imap_delimiter: char,
    pub watch_folder: Option<String>,
}

impl Default for ImapConfig {
    fn default() -> Self {
        let cfg = ImapConfig {
            addr: None,
            imap_server: None,
            imap_port: None,
            imap_user: None,
            imap_pw: None,
            server_flags: None,
            selected_folder: None,
            selected_mailbox: None,
            selected_folder_needs_expunge: false,
            should_reconnect: false,
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
            // idle: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(ImapConfig::default())),
            watch: Arc::new((Mutex::new(false), Condvar::new())),
            get_config,
            set_config,
            precheck_imf,
            receive_imf,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }

    pub fn should_reconnect(&self) -> bool {
        self.config.read().unwrap().should_reconnect
    }

    pub fn connect(&self, context: &dc_context_t, lp: *const dc_loginparam_t) -> libc::c_int {
        if lp.is_null() {
            return 0;
        }
        let lp = unsafe { *lp };
        if lp.mail_server.is_null() || lp.mail_user.is_null() || lp.mail_pw.is_null() {
            return 0;
        }

        if self.is_connected() {
            return 1;
        }

        let addr = to_str(lp.addr);
        let imap_server = to_str(lp.mail_server);
        let imap_port = lp.mail_port as u16;
        let imap_user = to_str(lp.mail_user);
        let imap_pw = to_str(lp.mail_pw);
        let server_flags = lp.server_flags as usize;

        let connection_res: imap::error::Result<Client> =
            if (server_flags & (DC_LP_IMAP_SOCKET_STARTTLS | DC_LP_IMAP_SOCKET_PLAIN)) != 0 {
                imap::connect_insecure((imap_server, imap_port)).and_then(|client| {
                    if (server_flags & DC_LP_IMAP_SOCKET_STARTTLS) != 0 {
                        let tls = native_tls::TlsConnector::builder()
                            // FIXME: unfortunately this is needed to make things work on macos + testrun.org
                            .danger_accept_invalid_hostnames(true)
                            .build()
                            .unwrap();
                        client.secure(imap_server, &tls).map(Into::into)
                    } else {
                        Ok(client.into())
                    }
                })
            } else {
                let tls = native_tls::TlsConnector::builder()
                    // FIXME: unfortunately this is needed to make things work on macos + testrun.org
                    .danger_accept_invalid_hostnames(true)
                    .build()
                    .unwrap();
                imap::connect((imap_server, imap_port), imap_server, &tls).map(Into::into)
            };

        match connection_res {
            Ok(client) => {
                // TODO: handle oauth2
                match client.login(imap_user, imap_pw) {
                    Ok(mut session) => {
                        // TODO: error handling
                        let caps = session.capabilities().unwrap();
                        let can_idle = caps.has("IDLE");
                        let has_xlist = caps.has("XLIST");

                        let caps_list = caps.iter().fold(String::new(), |mut s, c| {
                            s += " ";
                            s += c;
                            s
                        });
                        let caps_list_c = std::ffi::CString::new(caps_list).unwrap();

                        info!(context, 0, "IMAP-capabilities:%s", caps_list_c.as_ptr());

                        let mut config = self.config.write().unwrap();
                        config.can_idle = can_idle;
                        config.has_xlist = has_xlist;
                        config.addr = Some(addr.into());
                        config.imap_server = Some(imap_server.into());
                        config.imap_port = Some(imap_port.into());
                        config.imap_user = Some(imap_user.into());
                        config.imap_pw = Some(imap_pw.into());
                        config.server_flags = Some(server_flags);

                        *self.session.lock().unwrap() = Some(session);

                        1
                    }
                    Err((err, _)) => {
                        eprintln!("failed to login: {:?}", err);

                        unsafe {
                            dc_log_event_seq(
                                context,
                                Event::ERROR_NETWORK,
                                &mut 0 as *mut i32,
                                b"Cannot login\x00" as *const u8 as *const libc::c_char,
                            )
                        };

                        0
                    }
                }
            }
            Err(err) => {
                eprintln!("failed to connect: {:?}", err);
                unsafe {
                    dc_log_event_seq(
                        context,
                        Event::ERROR_NETWORK,
                        &mut 0 as *mut i32,
                        b"Could not connect to IMAP-server %s:%i.\x00" as *const u8
                            as *const libc::c_char,
                        imap_server,
                        imap_port as usize as libc::c_int,
                    )
                };

                0
            }
        }
    }

    pub fn disconnect(&self, context: &dc_context_t) {
        let session = self.session.lock().unwrap().take();
        if session.is_some() {
            match session.unwrap().close() {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("failed to close connection: {:?}", err);
                }
            }
        }

        let mut cfg = self.config.write().unwrap();

        cfg.addr = None;
        cfg.imap_server = None;
        cfg.imap_user = None;
        cfg.imap_pw = None;
        cfg.imap_port = None;

        cfg.can_idle = false;
        cfg.has_xlist = false;

        cfg.watch_folder = None;
        cfg.selected_folder = None;
        cfg.selected_mailbox = None;
        info!(context, 0, "IMAP disconnected.",);
    }

    pub fn set_watch_folder(&self, watch_folder: *const libc::c_char) {
        self.config.write().unwrap().watch_folder = Some(to_string(watch_folder));
    }

    pub fn fetch(&self, context: &dc_context_t) -> libc::c_int {
        let mut success = 0;

        let watch_folder = self.config.read().unwrap().watch_folder.to_owned();
        if self.is_connected() && watch_folder.is_some() {
            let watch_folder = watch_folder.unwrap();
            loop {
                let cnt = self.fetch_from_single_folder(context, &watch_folder);
                if cnt == 0 {
                    break;
                }
            }
            success = 1;
        }

        success
    }

    fn select_folder<S: AsRef<str>>(&self, context: &dc_context_t, folder: Option<S>) -> usize {
        if !self.is_connected() {
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
                info!(
                    context,
                    0,
                    "Expunge messages in \"%s\".",
                    CString::new(folder.to_owned()).unwrap().as_ptr()
                );

                // a CLOSE-SELECT is considerably faster than an EXPUNGE-SELECT, see https://tools.ietf.org/html/rfc3501#section-6.4.2
                if let Some(ref mut session) = *self.session.lock().unwrap() {
                    session.close().expect("failed to expunge");
                } else {
                    return 0;
                }
            }
        }

        // select new folder
        if let Some(folder) = folder {
            if let Some(ref mut session) = *self.session.lock().unwrap() {
                match session.select(folder) {
                    Ok(mailbox) => {
                        self.config.write().unwrap().selected_mailbox = Some(mailbox);
                    }
                    Err(err) => {
                        eprintln!("select error: {:?}", err);
                        info!(context, 0, "Cannot select folder.");
                        self.config.write().unwrap().selected_folder = None;
                    }
                }
            } else {
                return 0;
            }
        }

        1
    }

    fn get_config_last_seen_uid<S: AsRef<str>>(
        &self,
        context: &dc_context_t,
        folder: S,
    ) -> (u32, u32) {
        let key = format!("imap.mailbox.{}", folder.as_ref());
        let val1 = unsafe {
            (self.get_config)(
                context,
                CString::new(key).unwrap().as_ptr(),
                0 as *const libc::c_char,
            )
        };
        if val1.is_null() {
            return (0, 0);
        }
        let entry = to_str(val1);

        // the entry has the format `imap.mailbox.<folder>=<uidvalidity>:<lastseenuid>`
        let mut parts = entry.split(':');
        (
            parts.next().unwrap().parse().unwrap_or_else(|_| 0),
            parts.next().unwrap().parse().unwrap_or_else(|_| 0),
        )
    }

    fn fetch_from_single_folder<S: AsRef<str>>(&self, context: &dc_context_t, folder: S) -> usize {
        if !self.is_connected() {
            info!(
                context,
                0,
                "Cannot fetch from \"%s\" - not connected.",
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
            );

            return 0;
        }

        if self.select_folder(context, Some(&folder)) == 0 {
            info!(
                context,
                0,
                "Cannot select folder \"%s\" for fetching.",
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
            );

            return 0;
        }

        let (mut uid_validity, mut last_seen_uid) = self.get_config_last_seen_uid(context, &folder);

        let config = self.config.read().unwrap();
        let mailbox = config.selected_mailbox.as_ref().expect("just selected");

        if mailbox.uid_validity.is_none() {
            error!(
                context,
                0,
                "Cannot get UIDVALIDITY for folder \"%s\".",
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
            );

            return 0;
        }

        if mailbox.uid_validity.unwrap() != uid_validity {
            // first time this folder is selected or UIDVALIDITY has changed, init lastseenuid and save it to config

            if mailbox.exists == 0 {
                info!(
                    context,
                    0,
                    "Folder \"%s\" is empty.",
                    CString::new(folder.as_ref().to_owned()).unwrap().as_ptr()
                );

                // set lastseenuid=0 for empty folders.
                // id we do not do this here, we'll miss the first message
                // as we will get in here again and fetch from lastseenuid+1 then

                self.set_config_last_seen_uid(context, &folder, mailbox.uid_validity.unwrap(), 0);
                return 0;
            }

            let list = if let Some(ref mut session) = *self.session.lock().unwrap() {
                // `FETCH <message sequence number> (UID)`
                let set = format!("{}", mailbox.exists);
                match session.fetch(set, PREFETCH_FLAGS) {
                    Ok(list) => list,
                    Err(err) => {
                        eprintln!("fetch error: {:?}", err);
                        info!(
                            context,
                            0,
                            "No result returned for folder \"%s\".",
                            CString::new(folder.as_ref().to_owned()).unwrap().as_ptr()
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
                "lastseenuid initialized to %i for %s@%i",
                last_seen_uid as libc::c_int,
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                uid_validity as libc::c_int
            );
        }

        let mut read_cnt = 0;
        let mut read_errors = 0;
        let mut new_last_seen_uid = 0;

        let list = if let Some(ref mut session) = *self.session.lock().unwrap() {
            // fetch messages with larger UID than the last one seen
            // (`UID FETCH lastseenuid+1:*)`, see RFC 4549
            let set = format!("{}:*", last_seen_uid + 1);
            match session.uid_fetch(set, PREFETCH_FLAGS) {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("fetch err: {:?}", err);
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

                let message_id_c = CString::new(message_id).unwrap();
                let folder_c = CString::new(folder.as_ref().to_owned()).unwrap();
                if 0 == unsafe {
                    (self.precheck_imf)(context, message_id_c.as_ptr(), folder_c.as_ptr(), cur_uid)
                } {
                    // check passed, go fetch the rest
                    if self.fetch_single_msg(context, &folder, cur_uid) == 0 {
                        info!(
                            context,
                            0,
                            "Read error for message %s from \"%s\", trying over later.",
                            message_id_c.as_ptr(),
                            folder_c.as_ptr()
                        );

                        read_errors += 1;
                    }
                } else {
                    // check failed
                    info!(
                        context,
                        0,
                        "Skipping message %s from \"%s\" by precheck.",
                        message_id_c.as_ptr(),
                        folder_c.as_ptr()
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
                "%i mails read from \"%s\" with %i errors.",
                read_cnt as libc::c_int,
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                read_errors as libc::c_int,
            );
        } else {
            info!(
                context,
                0,
                "%i mails read from \"%s\".",
                read_cnt as libc::c_int,
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr()
            );
        }

        read_cnt
    }

    fn set_config_last_seen_uid<S: AsRef<str>>(
        &self,
        context: &dc_context_t,
        folder: S,
        uidvalidity: u32,
        lastseenuid: u32,
    ) {
        let key = format!("imap.mailbox.{}", folder.as_ref());
        let val = format!("{}:{}", uidvalidity, lastseenuid);

        unsafe {
            (self.set_config)(
                context,
                CString::new(key).unwrap().as_ptr(),
                CString::new(val).unwrap().as_ptr(),
            )
        };
    }

    fn fetch_single_msg<S: AsRef<str>>(
        &self,
        context: &dc_context_t,
        folder: S,
        server_uid: u32,
    ) -> usize {
        // the function returns:
        // 0  the caller should try over again later
        // or  1  if the messages should be treated as received, the caller should not try to read the message again (even if no database entries are returned)
        if !self.is_connected() {
            return 0;
        }

        let mut retry_later = false;

        let msgs = if let Some(ref mut session) = *self.session.lock().unwrap() {
            let set = format!("{}", server_uid);
            match session.uid_fetch(set, BODY_FLAGS) {
                Ok(msgs) => msgs,
                Err(err) => {
                    eprintln!("error fetch single: {:?}", err);
                    warn!(
                        context,
                        0,
                        "Error on fetching message #%i from folder \"%s\"; retry=%i.",
                        server_uid as libc::c_int,
                        CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                        self.should_reconnect() as libc::c_int,
                    );

                    if self.should_reconnect() {
                        retry_later = true;
                    }

                    return if retry_later { 0 } else { 1 };
                }
            }
        } else {
            return if retry_later { 0 } else { 1 };
        };

        if msgs.is_empty() {
            warn!(
                context,
                0,
                "Message #%i does not exist in folder \"%s\".",
                server_uid as libc::c_int,
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
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
                unsafe {
                    let folder_c = CString::new(folder.as_ref().to_owned()).unwrap();
                    (self.receive_imf)(
                        context,
                        msg.body().unwrap().as_ptr() as *const libc::c_char,
                        msg.body().unwrap().len(),
                        folder_c.as_ptr(),
                        server_uid,
                        flags as u32,
                    );
                }
            }
        }

        if retry_later {
            0
        } else {
            1
        }
    }

    pub fn idle(&self, context: &dc_context_t) {
        if !self.config.read().unwrap().can_idle {
            return self.fake_idle(context);
        }

        // TODO: reconnect in all methods that need it
        if !self.is_connected() {
            return;
        }

        let watch_folder = self.config.read().unwrap().watch_folder.clone();
        if self.select_folder(context, watch_folder.as_ref()) == 0 {
            warn!(context, 0, "IMAP-IDLE not setup.",);

            return self.fake_idle(context);
        }

        // let mut session = self.session.lock().unwrap().take().unwrap();

        // match RentSession::try_new(Box::new(session), |session| session.idle()) {
        //     Ok(idle) => {
        //         *self.idle.lock().unwrap() = Some(idle);
        //     }
        //     Err(err) => {
        //         eprintln!("imap idle error: {:?}", err.0);
        //         unsafe {
        //             dc_log_warning(
        //                 context,
        //                 0,
        //                 b"IMAP-IDLE: Cannot start.\x00" as *const u8 as *const libc::c_char,
        //             );
        //         }

        //         // put session back
        //         *self.session.lock().unwrap() = Some(*err.1);

        //         return self.fake_idle(context);
        //     }
        // }

        let mut session = self.session.lock().unwrap().take().unwrap();
        let mut idle = match session.idle() {
            Ok(idle) => idle,
            Err(err) => {
                eprintln!("imap idle error: {:?}", err);
                warn!(context, 0, "IMAP-IDLE: Cannot start.",);

                return self.fake_idle(context);
            }
        };

        // most servers do not allow more than ~28 minutes; stay clearly below that.
        // a good value that is also used by other MUAs is 23 minutes.
        // if needed, the ui can call dc_imap_interrupt_idle() to trigger a reconnect.
        idle.set_keepalive(Duration::from_secs(23 * 60));

        // TODO: proper logging of different states
        // TODO: reconnect if we timed out
        match idle.wait_keepalive() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("idle error: {:?}", err);
            }
        }

        // put session back
        *self.session.lock().unwrap() = Some(session);
    }

    fn fake_idle(&self, context: &dc_context_t) {
        // Idle using timeouts. This is also needed if we're not yet configured -
        // in this case, we're waiting for a configure job
        let fake_idle_start_time = SystemTime::now();

        info!(context, 0, "IMAP-fake-IDLEing...");

        let mut do_fake_idle = true;
        while do_fake_idle {
            let seconds_to_wait =
                if fake_idle_start_time.elapsed().unwrap() < Duration::new(3 * 60, 0) {
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

            // TODO: connect if needed
            if let Some(ref watch_folder) = self.config.read().unwrap().watch_folder {
                if 0 != self.fetch_from_single_folder(context, watch_folder) {
                    do_fake_idle = false;
                }
            }
        }
    }

    pub fn interrupt_idle(&self) {
        // TODO: interrupt real idle
        // ref: https://github.com/jonhoo/rust-imap/issues/121

        let &(ref lock, ref cvar) = &*self.watch.clone();
        let mut watch = lock.lock().unwrap();

        *watch = true;
        cvar.notify_one();
    }

    pub fn mv<S1: AsRef<str>, S2: AsRef<str>>(
        &self,
        context: &dc_context_t,
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
                "Skip moving message; message %s/%i is already in %s...",
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                uid as libc::c_int,
                CString::new(dest_folder.as_ref().to_owned())
                    .unwrap()
                    .as_ptr()
            );

            res = DC_ALREADY_DONE;
        } else {
            info!(
                context,
                0,
                "Moving message %s/%i to %s...",
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                uid as libc::c_int,
                CString::new(dest_folder.as_ref().to_owned())
                    .unwrap()
                    .as_ptr()
            );

            if self.select_folder(context, Some(folder.as_ref())) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder %s for moving message.",
                    CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                );
            } else {
                let moved = if let Some(ref mut session) = *self.session.lock().unwrap() {
                    match session.uid_mv(&set, &dest_folder) {
                        Ok(_) => {
                            res = DC_SUCCESS;
                            true
                        }
                        Err(err) => {
                            eprintln!("move error: {:?}", err);
                            info!(
                                context,
                                0,
                                "Cannot move message, fallback to COPY/DELETE %s/%i to %s...",
                                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                                uid as libc::c_int,
                                CString::new(dest_folder.as_ref().to_owned())
                                    .unwrap()
                                    .as_ptr()
                            );

                            false
                        }
                    }
                } else {
                    unreachable!();
                };

                if !moved {
                    let copied = if let Some(ref mut session) = *self.session.lock().unwrap() {
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
                        if self.add_flag(uid, "\\Deleted") == 0 {
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

    fn add_flag<S: AsRef<str>>(&self, server_uid: u32, flag: S) -> usize {
        if let Some(ref mut session) = *self.session.lock().unwrap() {
            let set = format!("{}", server_uid);
            let query = format!("+ FLAGS ({})", flag.as_ref());
            match session.uid_store(set, query) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("imap store error {:?}", err);
                }
            }
        }

        if self.should_reconnect() {
            0
        } else {
            1
        }
    }

    pub fn set_seen<S: AsRef<str>>(&self, context: &dc_context_t, folder: S, uid: u32) -> usize {
        let mut res = DC_RETRY_LATER;

        if uid == 0 {
            res = DC_FAILED
        } else if self.is_connected() {
            let folder_c = CString::new(folder.as_ref().to_owned()).unwrap();

            info!(
                context,
                0,
                "Marking message %s/%i as seen...",
                folder_c.as_ptr(),
                uid as libc::c_int
            );

            if self.select_folder(context, Some(folder)) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder %s for setting SEEN flag.",
                    folder_c.as_ptr(),
                );
            } else if self.add_flag(uid, "\\Seen") == 0 {
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

    pub fn set_mdnsent<S: AsRef<str>>(&self, context: &dc_context_t, folder: S, uid: u32) -> usize {
        // returns 0=job should be retried later, 1=job done, 2=job done and flag just set
        let mut res = DC_RETRY_LATER;
        let set = format!("{}", uid);

        if uid == 0 {
            res = DC_FAILED;
        } else if self.is_connected() {
            let folder_c = CString::new(folder.as_ref().to_owned()).unwrap();
            info!(
                context,
                0,
                "Marking message %s/%i as $MDNSent...",
                folder_c.as_ptr(),
                uid as libc::c_int
            );

            if self.select_folder(context, Some(folder)) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder %s for setting $MDNSent flag.",
                    folder_c.as_ptr(),
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
                    let fetched_msgs = if let Some(ref mut session) = *self.session.lock().unwrap()
                    {
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
                        } else if self.add_flag(uid, "$MDNSent") != 0 {
                            DC_SUCCESS
                        } else {
                            res
                        };

                        let msg = if res == DC_SUCCESS {
                            "$MDNSent just set and MDN will be sent."
                        } else {
                            "$MDNSent already set and MDN already sent."
                        };

                        info!(context, 0, msg);
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
        context: &dc_context_t,
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
                "Marking message \"%s\", %s/%i for deletion...",
                &message_id,
                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                *server_uid as libc::c_int
            );

            if self.select_folder(context, Some(&folder)) == 0 {
                warn!(
                    context,
                    0,
                    "Cannot select folder %s for deleting message.",
                    CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                );
            } else {
                let set = format!("{}", server_uid);
                if let Some(ref mut session) = *self.session.lock().unwrap() {
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
                                    "Cannot delete on IMAP, %s/%i does not match %s.",
                                    CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                                    *server_uid as libc::c_int,
                                    message_id,
                                );
                                *server_uid = 0;
                            }
                        }
                        Err(err) => {
                            eprintln!("fetch error: {:?}", err);

                            warn!(
                                context,
                                0,
                                "Cannot delete on IMAP, %s/%i not found.",
                                CString::new(folder.as_ref().to_owned()).unwrap().as_ptr(),
                                *server_uid as libc::c_int,
                            );
                            *server_uid = 0;
                        }
                    }
                }

                // mark the message for deletion
                if self.add_flag(*server_uid, "\\Deleted") == 0 {
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

    pub fn configure_folders(&self, context: &dc_context_t, flags: libc::c_int) {
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
            info!(
                context,
                0,
                "Creating MVBOX-folder \"%s\"...",
                b"DeltaChat\x00" as *const u8 as *const libc::c_char
            );

            if let Some(ref mut session) = *self.session.lock().unwrap() {
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

        unsafe {
            dc_sqlite3_set_config_int(
                context,
                &context.sql.read().unwrap(),
                b"folders_configured\x00" as *const u8 as *const libc::c_char,
                3,
            );
            if let Some(ref mvbox_folder) = mvbox_folder {
                dc_sqlite3_set_config(
                    context,
                    &context.sql.read().unwrap(),
                    b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                    CString::new(mvbox_folder.clone()).unwrap().as_ptr(),
                );
            }
            if let Some(ref sentbox_folder) = sentbox_folder {
                dc_sqlite3_set_config(
                    context,
                    &context.sql.read().unwrap(),
                    b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
                    CString::new(sentbox_folder.name()).unwrap().as_ptr(),
                );
            }
        }
    }

    fn list_folders(
        &self,
        context: &dc_context_t,
    ) -> Option<imap::types::ZeroCopy<Vec<imap::types::Name>>> {
        if let Some(ref mut session) = *self.session.lock().unwrap() {
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

fn to_string(str: *const libc::c_char) -> String {
    unsafe { CStr::from_ptr(str).to_str().unwrap().to_string() }
}

fn to_str<'a>(str: *const libc::c_char) -> &'a str {
    unsafe { CStr::from_ptr(str).to_str().unwrap() }
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
