use std::borrow::Cow;

use crate::context::Context;
use crate::sql::Sql;

#[derive(Default, Debug)]
pub struct dc_loginparam_t {
    pub addr: String,
    pub mail_server: String,
    pub mail_user: String,
    pub mail_pw: String,
    pub mail_port: i32,
    pub send_server: String,
    pub send_user: String,
    pub send_pw: String,
    pub send_port: i32,
    pub server_flags: i32,
}

impl dc_loginparam_t {
    pub fn addr_str(&self) -> &str {
        self.addr.as_str()
    }
}

pub fn dc_loginparam_new() -> dc_loginparam_t {
    Default::default()
}

pub fn dc_loginparam_read(
    context: &Context,
    sql: &Sql,
    prefix: impl AsRef<str>,
) -> dc_loginparam_t {
    let prefix = prefix.as_ref();

    let key = format!("{}addr", prefix);
    let addr = sql
        .get_config(context, key, None)
        .unwrap_or_default()
        .trim()
        .to_string();

    let key = format!("{}mail_server", prefix);
    let mail_server = sql.get_config(context, key, None).unwrap_or_default();

    let key = format!("{}mail_port", prefix);
    let mail_port = sql.get_config_int(context, key, 0);

    let key = format!("{}mail_user", prefix);
    let mail_user = sql.get_config(context, key, None).unwrap_or_default();

    let key = format!("{}mail_pw", prefix);
    let mail_pw = sql.get_config(context, key, None).unwrap_or_default();

    let key = format!("{}send_server", prefix);
    let send_server = sql.get_config(context, key, None).unwrap_or_default();

    let key = format!("{}send_port", prefix);
    let send_port = sql.get_config_int(context, key, 0);

    let key = format!("{}send_user", prefix);
    let send_user = sql.get_config(context, key, None).unwrap_or_default();

    let key = format!("{}send_pw", prefix);
    let send_pw = sql.get_config(context, key, None).unwrap_or_default();

    let key = format!("{}server_flags", prefix);
    let server_flags = sql.get_config_int(context, key, 0);

    dc_loginparam_t {
        addr: addr.to_string(),
        mail_server,
        mail_user,
        mail_pw,
        mail_port,
        send_server,
        send_user,
        send_pw,
        send_port,
        server_flags,
    }
}

pub fn dc_loginparam_write(
    context: &Context,
    loginparam: &dc_loginparam_t,
    sql: &Sql,
    prefix: impl AsRef<str>,
) {
    let prefix = prefix.as_ref();

    let key = format!("{}addr", prefix);
    sql.set_config(context, key, Some(&loginparam.addr));

    let key = format!("{}mail_server", prefix);
    sql.set_config(context, key, Some(&loginparam.mail_server));

    let key = format!("{}mail_port", prefix);
    sql.set_config_int(context, key, loginparam.mail_port);

    let key = format!("{}mail_user", prefix);
    sql.set_config(context, key, Some(&loginparam.mail_user));

    let key = format!("{}mail_pw", prefix);
    sql.set_config(context, key, Some(&loginparam.mail_pw));

    let key = format!("{}send_server", prefix);
    sql.set_config(context, key, Some(&loginparam.send_server));

    let key = format!("{}send_port", prefix);
    sql.set_config_int(context, key, loginparam.send_port);

    let key = format!("{}send_user", prefix);
    sql.set_config(context, key, Some(&loginparam.send_user));

    let key = format!("{}send_pw", prefix);
    sql.set_config(context, key, Some(&loginparam.send_pw));

    let key = format!("{}server_flags", prefix);
    sql.set_config_int(context, key, loginparam.server_flags);
}

fn unset_empty(s: &String) -> Cow<String> {
    if s.is_empty() {
        Cow::Owned("unset".to_string())
    } else {
        Cow::Borrowed(s)
    }
}

pub fn dc_loginparam_get_readable(loginparam: &dc_loginparam_t) -> String {
    let unset = "0";
    let pw = "***";

    let flags_readable = get_readable_flags(loginparam.server_flags);

    format!(
        "{} {}:{}:{}:{} {}:{}:{}:{} {}",
        unset_empty(&loginparam.addr),
        unset_empty(&loginparam.mail_user),
        if !loginparam.mail_pw.is_empty() {
            pw
        } else {
            unset
        },
        unset_empty(&loginparam.mail_server),
        loginparam.mail_port,
        unset_empty(&loginparam.send_user),
        if !loginparam.send_pw.is_empty() {
            pw
        } else {
            unset
        },
        unset_empty(&loginparam.send_server),
        loginparam.send_port,
        flags_readable,
    )
}

fn get_readable_flags(flags: i32) -> String {
    let mut res = String::new();
    for bit in 0..31 {
        if 0 != flags & 1 << bit {
            let mut flag_added = 0;
            if 1 << bit == 0x2 {
                res += "OAUTH2 ";
                flag_added = 1;
            }
            if 1 << bit == 0x4 {
                res += "AUTH_NORMAL ";
                flag_added = 1;
            }
            if 1 << bit == 0x100 {
                res += "IMAP_STARTTLS ";
                flag_added = 1;
            }
            if 1 << bit == 0x200 {
                res += "IMAP_SSL ";
                flag_added = 1;
            }
            if 1 << bit == 0x400 {
                res += "IMAP_PLAIN ";
                flag_added = 1;
            }
            if 1 << bit == 0x10000 {
                res += "SMTP_STARTTLS ";
                flag_added = 1
            }
            if 1 << bit == 0x20000 {
                res += "SMTP_SSL ";
                flag_added = 1
            }
            if 1 << bit == 0x40000 {
                res += "SMTP_PLAIN ";
                flag_added = 1
            }
            if 0 == flag_added {
                res += &format!("{:#0x}", 1 << bit);
            }
        }
    }
    if res.is_empty() {
        res += "0";
    }

    res
}
