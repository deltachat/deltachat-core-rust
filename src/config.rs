use std::str::FromStr;

use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::constants::DC_VERSION_STR;
use crate::context::Context;
use crate::dc_job::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::x::*;

/// The available configuration keys.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr, EnumIter, EnumProperty,
)]
#[strum(serialize_all = "snake_case")]
pub enum Config {
    Addr,
    MailServer,
    MailUser,
    MailPw,
    MailPort,
    SendServer,
    SendUser,
    SendPw,
    SendPort,
    ServerFlags,
    #[strum(props(default = "INBOX"))]
    ImapFolder,
    Displayname,
    Selfstatus,
    Selfavatar,
    #[strum(props(default = "1"))]
    E2eeEnabled,
    #[strum(props(default = "1"))]
    MdnsEnabled,
    InboxWatch,
    #[strum(props(default = "1"))]
    SentboxWatch,
    #[strum(props(default = "1"))]
    MvboxWatch,
    #[strum(props(default = "1"))]
    MvboxMove,
    #[strum(props(default = "0"))]
    ShowEmails,
    SaveMimeHeaders,
    ConfiguredAddr,
    ConfiguredMailServer,
    ConfiguredMailUser,
    ConfiguredMailPw,
    ConfiguredMailPort,
    ConfiguredSendServer,
    ConfiguredSendUser,
    ConfiguredSendPw,
    ConfiguredSendPort,
    ConfiguredServerFlags,
    Configured,
}

// deprecated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr, EnumIter)]
pub enum SysConfig {
    #[strum(serialize = "sys.version")]
    Version,
    #[strum(serialize = "sys.msgsize_max_recommended")]
    MsgsizeMaxRecommended,
    #[strum(serialize = "sys.config_keys")]
    ConfigKeys,
}

/// Get a configuration key.
/// Returns "" when the key is invalid, or no default was found.
pub fn get(context: &Context, key: impl AsRef<str>) -> String {
    let key = key.as_ref();

    if key.starts_with("sys.") {
        return get_sys_config_str(key);
    }

    match Config::from_str(key) {
        Ok(config_key) => {
            let value = match config_key {
                Config::Selfavatar => {
                    let rel_path = context.sql.get_config(context, key, None);
                    rel_path.map(|p| {
                        let v = unsafe { dc_get_abs_path(context, to_cstring(p).as_ptr()) };
                        let r = to_string(v);
                        unsafe { free(v as *mut _) };
                        r
                    })
                }
                _ => context.sql.get_config(context, key, None),
            };

            if value.is_some() {
                return value.unwrap();
            }

            // Default values
            match config_key {
                Config::Selfstatus => {
                    let s = unsafe { dc_stock_str(context, 13) };
                    let res = to_string(s);
                    unsafe { free(s as *mut _) };
                    res
                }
                _ => config_key
                    .get_str("default")
                    .unwrap_or_default()
                    .to_string(),
            }
        }
        Err(_) => "".into(),
    }
}

fn get_sys_config_str(key: impl AsRef<str>) -> String {
    match SysConfig::from_str(key.as_ref()) {
        Ok(SysConfig::Version) => std::str::from_utf8(DC_VERSION_STR).unwrap().into(),
        Ok(SysConfig::MsgsizeMaxRecommended) => format!("{}", 24 * 1024 * 1024 / 4 * 3),
        Ok(SysConfig::ConfigKeys) => get_config_keys_str(),
        Err(_) => "".into(),
    }
}

fn get_config_keys_str() -> String {
    let keys = Config::iter().fold(String::new(), |mut acc, key| {
        acc += key.as_ref();
        acc += " ";
        acc
    });

    let sys_keys = SysConfig::iter().fold(String::new(), |mut acc, key| {
        acc += key.as_ref();
        acc += " ";
        acc
    });

    format!(" {} {} ", keys, sys_keys)
}

/// Set the given config key.
/// Returns `1` on success and `0` on failure.
pub fn set(context: &Context, key: impl AsRef<str>, value: Option<&str>) -> libc::c_int {
    let mut ret = 0;

    // regular keys
    match Config::from_str(key.as_ref()) {
        Ok(Config::Selfavatar) if value.is_some() => {
            let mut rel_path = unsafe { dc_strdup(to_cstring(value.unwrap()).as_ptr()) };
            if 0 != unsafe { dc_make_rel_and_copy(context, &mut rel_path) } {
                ret = context.sql.set_config(context, key, Some(as_str(rel_path)));
            }
            unsafe { free(rel_path as *mut libc::c_void) };
        }
        Ok(Config::InboxWatch) => {
            ret = context.sql.set_config(context, key, value);
            unsafe { dc_interrupt_imap_idle(context) };
        }
        Ok(Config::SentboxWatch) => {
            ret = context.sql.set_config(context, key, value);
            unsafe { dc_interrupt_sentbox_idle(context) };
        }
        Ok(Config::MvboxWatch) => {
            ret = context.sql.set_config(context, key, value);
            unsafe { dc_interrupt_mvbox_idle(context) };
        }
        Ok(Config::Selfstatus) => {
            let def = unsafe { dc_stock_str(context, 13) };
            let val = if value.is_none() || value.unwrap() == as_str(def) {
                None
            } else {
                value
            };

            ret = context.sql.set_config(context, key, val);
            unsafe { free(def as *mut libc::c_void) };
        }
        Ok(_) => {
            ret = context.sql.set_config(context, key, value);
        }
        Err(_) => {}
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;
    use std::string::ToString;

    #[test]
    fn test_to_string() {
        assert_eq!(Config::MailServer.to_string(), "mail_server");
        assert_eq!(Config::from_str("mail_server"), Ok(Config::MailServer));

        assert_eq!(SysConfig::ConfigKeys.to_string(), "sys.config_keys");
        assert_eq!(
            SysConfig::from_str("sys.config_keys"),
            Ok(SysConfig::ConfigKeys)
        );
    }

    #[test]
    fn test_default_prop() {
        assert_eq!(Config::ImapFolder.get_str("default"), Some("INBOX"));
    }
}
