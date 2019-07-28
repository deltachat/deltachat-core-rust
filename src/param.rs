use std::collections::BTreeMap;
use std::fmt;
use std::str;

use num_traits::FromPrimitive;

use crate::error;

/// Available param keys.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, PartialOrd, Ord, FromPrimitive)]
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
    GroupId = 'x' as u8,
    GroupName = 'g' as u8,
}

/// Possible values for `Param::ForcePlaintext`.
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum ForcePlaintext {
    AddAutocryptHeader = 1,
    NoAutocryptHeader = 2,
}

/// An object for handling key=value parameter lists.
///
/// The structure is serialized by calling `to_string()` on it.
///
/// Only for library-internal use.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Params {
    inner: BTreeMap<Param, String>,
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (key, value)) in self.inner.iter().enumerate() {
            if i > 0 {
                write!(f, "\n")?;
            }
            write!(f, "{}={}", *key as u8 as char, value)?;
        }
        Ok(())
    }
}

impl str::FromStr for Params {
    type Err = error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut inner = BTreeMap::new();
        for pair in s.trim().lines() {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            // TODO: probably nicer using a regex
            ensure!(pair.len() > 2, "Invalid key pair: '{}'", pair);
            let mut split = pair.splitn(2, '=');
            let key = split.next();
            let value = split.next();

            ensure!(key.is_some(), "Missing key");
            ensure!(value.is_some(), "Missing value");

            let key = key.unwrap().trim();
            let value = value.unwrap().trim();

            if let Some(key) = Param::from_u8(key.as_bytes()[0]) {
                inner.insert(key, value.to_string());
            } else {
                bail!("Unknown key: {}", key);
            }
        }

        Ok(Params { inner })
    }
}

impl Params {
    /// Get the value of the given key, return `None` if no value is set.
    pub fn get(&self, key: Param) -> Option<&str> {
        self.inner.get(&key).map(|s| s.as_str())
    }

    /// Check if the given key is set.
    pub fn exists(&self, key: Param) -> bool {
        self.inner.contains_key(&key)
    }

    /// Set the given key to the passed in value.
    pub fn set(&mut self, key: Param, value: impl AsRef<str>) {
        self.inner.insert(key, value.as_ref().to_string());
    }

    /// Removes the given key, if it exists.
    pub fn remove(&mut self, key: Param) {
        self.inner.remove(&key);
    }

    /// Check if there are any values in this.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns how many key-value pairs are set.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

pub fn dc_param_get_int(param: &Params, key: Param) -> Option<i32> {
    param.get(key).and_then(|s| s.parse().ok())
}

pub fn dc_param_get_float(param: &Params, key: Param) -> Option<f64> {
    param.get(key).and_then(|s| s.parse().ok())
}

pub fn dc_param_set(param: &mut Params, key: Param, value: impl AsRef<str>) {
    param.set(key, value);
}

pub fn dc_param_remove(param: &mut Params, key: Param) {
    param.remove(key);
}

pub fn dc_param_set_int(param: &mut Params, key: Param, value: i32) {
    param.set(key, format!("{}", value));
}

pub fn dc_param_set_float(param: &mut Params, key: Param, value: f64) {
    param.set(key, format!("{}", value));
}

pub fn dc_param_new() -> Params {
    Default::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_param() {
        let mut p1: Params = "\r\n\r\na=1\nf=2\n\nc = 3 ".parse().unwrap();

        assert_eq!(dc_param_get_int(&p1, Param::Forwarded), Some(1));
        assert_eq!(dc_param_get_int(&p1, Param::File), Some(2));
        assert_eq!(dc_param_get_int(&p1, Param::Height), None);
        assert!(!p1.exists(Param::Height));

        dc_param_set_int(&mut p1, Param::Duration, 4);

        assert_eq!(dc_param_get_int(&p1, Param::Duration), Some(4));

        let mut p1 = dc_param_new();
        dc_param_set(&mut p1, Param::Forwarded, "foo");
        dc_param_set_int(&mut p1, Param::File, 2);
        dc_param_remove(&mut p1, Param::GuranteeE2ee);
        dc_param_set_int(&mut p1, Param::Duration, 4);

        assert_eq!(p1.to_string(), "a=foo\nd=4\nf=2");

        dc_param_remove(&mut p1, Param::File);

        assert_eq!(p1.to_string(), "a=foo\nd=4",);
        assert_eq!(p1.len(), 2);

        dc_param_remove(&mut p1, Param::Forwarded);
        dc_param_remove(&mut p1, Param::Duration);

        assert_eq!(p1.to_string(), "",);

        assert!(p1.is_empty());
        assert_eq!(p1.len(), 0)
    }
}
