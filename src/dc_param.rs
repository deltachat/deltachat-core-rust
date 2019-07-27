use std::collections::BTreeMap;
use std::fmt;
use std::str;

use num_traits::ToPrimitive;

use crate::error::{self, Result};

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Param {
    File = 'f' as u8,
    Width = 'w' as u8,
    Height = 'h' as u8,
    Duration = 'd' as u8,
    MimeType = 'm' as u8,
    GuranteeE2ee = 'c' as u8,
    ErroneousE2ee = 'e' as u8,
    ForcePlaintext = 'u' as u8,
    WantsMdn = 'r' as u8,
    Forwarded = 'a' as u8,
    Cmd = 'S' as u8,
    Arg = 'E' as u8,
    Arg2 = 'F' as u8,
    Arg3 = 'G' as u8,
    Arg4 = 'H' as u8,
    Error = 'L' as u8,
    PrepForwards = 'P' as u8,
    SetLatitude = 'l' as u8,
    SetLongitude = 'n' as u8,
    ServerFolder = 'Z' as u8,
    ServerUid = 'z' as u8,
    AlsoMove = 'M' as u8,
    Recipients = 'R' as u8,
    Unpromoted = 'U' as u8,
    ProfileImage = 'i' as u8,
    Selftalk = 'K' as u8,
    Auth = 's' as u8,
    Grpid = 'x' as u8,
    UrlEncoded = 'g' as u8,
}
// values for DC_PARAM_FORCE_PLAINTEXT
pub const DC_FP_ADD_AUTOCRYPT_HEADER: u8 = 1;
pub const DC_FP_NO_AUTOCRYPT_HEADER: u8 = 2;

/// An object for handling key=value parameter lists; for the key, currently only
/// a single character is allowed.
///
/// The object is used eg. by Chat or dc_msg_t.
///
/// Only for library-internal use.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct dc_param_t {
    inner: BTreeMap<Param, String>,
}

impl fmt::Display for dc_param_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in self.inner.entries() {
            write!(f, "{}={}\n", key, value)?;
        }
        Ok(())
    }
}

impl str::FromStr for dc_param_t {
    type Err = error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut inner = BTreeMap::new();
        for pair in s.trim().lines() {
            let key = Param::from_u8(pair[0])?;
            ensure_eq!(pair[1], '=', "invalid separator");
            let value = pair[2..];
            inner.insert(key, value.to_string());
        }

        Ok(dc_param_t { inner })
    }
}

impl dc_param_t {
    pub fn get(&self, key: Param) -> Option<&str> {
        self.inner.get(&key).map(|s| s.as_str())
    }

    pub fn exists(&self, key: Param) -> bool {
        self.inner.contains_key(&key)
    }

    pub fn set(&self, key: Param, value: impl AsRef<str>) {
        self.inner.insert(key, value.as_ref().to_string());
    }

    pub fn remove(&mut self, key: Param) {
        self.inner.remove(key);
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

pub fn dc_param_exists(param: &dc_param_t, key: Param) -> bool {
    param.exists(key)
}

pub fn dc_param_get(param: &dc_param_t, key: Param) -> Option<&str> {
    param.get(key)
}

pub fn dc_param_get_int(param: &dc_param_t, key: Param) -> Option<i32> {
    param.get(key).map(|s| s.parse().ok())
}

pub fn dc_param_get_float(param: &dc_param_t, key: Param) -> Option<f32> {
    param.get(key).map(|s| s.parse().ok())
}

pub fn dc_param_set(param: &mut dc_param_t, key: Param, value: impl AsRef<str>) {
    param.set(key, value);
}

pub fn dc_param_remove(param: &mut dc_param_t, key: Param) {
    param.remove(key);
}

pub fn dc_param_set_int(param: &mut dc_param_t, key: Param, value: i32) {
    param.set(key, format!("{}", value));
}

pub fn dc_param_set_float(param: &mut dc_param_t, key: Param, value: f32) {
    param.set(key, format!("{}", value));
}

pub fn dc_param_new() -> dc_param_t {
    Default::default()
}

pub fn dc_param_set_packed(param: &mut dc_param_t, packed: impl AsRef<str>) -> Result<()> {
    *param = packed.as_ref().parse()?;
    Ok(())
}

pub fn dc_param_set_urlencoded(param: &mut dc_param_t, urlencoded: impl AsRef<str>) -> Result<()> {
    dc_param_set_packed(urlencoded.replace('&', '\n'))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_dc_param() {
        let mut p1 = dc_param_new();
        dc_param_set_packed(&mut p1, "\r\n\r\na=1\nf=2\n\nc = 3 ");
        assert_eq!(p1.len(), 3);

        assert_eq!(dc_param_get_int(&p1, Param::Forwarded), 1);
        assert_eq!(dc_param_get_int(&p1, Param::File), 2);
        assert_eq!(dc_param_get_int(&p1, Param::Height), None);
        assert!(dc_param_exists(&p1, Param::Height));

        dc_param_set_int(&mut p1, Param::Duration, 4);

        assert_eq!(dc_param_get_int(&p1, Param::Duration), 4);

        let mut p1 = dc_param_new();
        dc_param_set(&mut p1, Param::Forwarded, "foo");
        dc_param_set_int(&mut p1, Param::File, 2);
        dc_param_remove(&mut p1, Param::GuranteeE2ee);
        dc_param_set_int(&mut p1, Param::Duration, 4);

        assert_eq!(p1.to_string(), "a=foo\nf=2\nd=4");

        dc_param_remove(&mut p1, Param::File);

        assert_eq!(p1.to_string(), "a=foo\nd=4",);
        assert_eq!(p1.len(), 2);

        dc_param_remove(&mut p1, Param::Forwarded);
        dc_param_remove(&mut p1, Param::Duration);

        assert_eq!(p1.to_string(), "",);

        assert!(p1.is_empty());
        assert!(p1.len(), 0)
    }
}
