use std::collections::BTreeMap;
use std::ffi::CStr;
use std::str::FromStr;
use std::{fmt, str};

use mmime::mailimf::types::*;

use crate::constants::*;
use crate::contact::*;
use crate::key::*;

/// Possible values for encryption preference
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum EncryptPreference {
    NoPreference = 0,
    Mutual = 1,
    Reset = 20,
}

impl Default for EncryptPreference {
    fn default() -> Self {
        EncryptPreference::NoPreference
    }
}

impl fmt::Display for EncryptPreference {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EncryptPreference::Mutual => write!(fmt, "mutual"),
            EncryptPreference::NoPreference => write!(fmt, "nopreference"),
            EncryptPreference::Reset => write!(fmt, "reset"),
        }
    }
}

impl str::FromStr for EncryptPreference {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mutual" => Ok(EncryptPreference::Mutual),
            "reset" => Ok(EncryptPreference::Reset),
            _ => Ok(EncryptPreference::NoPreference),
        }
    }
}

/// Parse and create [Autocrypt-headers](https://autocrypt.org/en/latest/level1.html#the-autocrypt-header).
#[derive(Debug)]
pub struct Aheader {
    pub addr: String,
    pub public_key: Key,
    pub prefer_encrypt: EncryptPreference,
}

impl Aheader {
    pub fn new(addr: String, public_key: Key, prefer_encrypt: EncryptPreference) -> Self {
        Aheader {
            addr,
            public_key,
            prefer_encrypt,
        }
    }

    pub fn from_imffields(wanted_from: &str, header: *const mailimf_fields) -> Option<Self> {
        if header.is_null() {
            return None;
        }

        let mut fine_header = None;
        let mut cur = unsafe { (*(*header).fld_list).first };

        while !cur.is_null() {
            let field = unsafe { (*cur).data as *mut mailimf_field };
            if !field.is_null()
                && unsafe { (*field).fld_type } == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
            {
                let optional_field = unsafe { (*field).fld_data.fld_optional_field };
                if !optional_field.is_null()
                    && unsafe { !(*optional_field).fld_name.is_null() }
                    && unsafe { CStr::from_ptr((*optional_field).fld_name).to_string_lossy() }
                        == "Autocrypt"
                {
                    let value =
                        unsafe { CStr::from_ptr((*optional_field).fld_value).to_string_lossy() };

                    if let Ok(test) = Self::from_str(&value) {
                        if addr_cmp(&test.addr, wanted_from) {
                            if fine_header.is_none() {
                                fine_header = Some(test);
                            } else {
                                // TODO: figure out what kind of error case this is
                                return None;
                            }
                        }
                    }
                }
            }

            cur = unsafe { (*cur).next };
        }

        fine_header
    }
}

impl fmt::Display for Aheader {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // TODO replace 78 with enum /rtn
        // adds a whitespace every 78 characters, this allows libEtPan to
        // wrap the lines according to RFC 5322
        // (which may insert a linebreak before every whitespace)
        let keydata = self.public_key.to_base64(78);
        write!(
            fmt,
            "addr={}; prefer-encrypt={}; keydata={}",
            self.addr, self.prefer_encrypt, keydata
        )
    }
}

impl str::FromStr for Aheader {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut attributes: BTreeMap<String, String> = s
            .split(';')
            .filter_map(|a| {
                let attribute: Vec<&str> = a.trim().splitn(2, '=').collect();
                if attribute.len() < 2 {
                    return None;
                }

                Some((
                    attribute[0].trim().to_string(),
                    attribute[1].trim().to_string(),
                ))
            })
            .collect();

        let addr = match attributes.remove("addr") {
            Some(addr) => addr,
            None => {
                return Err(());
            }
        };

        let public_key = match attributes
            .remove("keydata")
            .and_then(|raw| Key::from_base64(&raw, KeyType::Public))
        {
            Some(key) => {
                if key.verify() {
                    key
                } else {
                    return Err(());
                }
            }
            None => {
                return Err(());
            }
        };

        let prefer_encrypt = match attributes
            .remove("prefer-encrypt")
            .and_then(|raw| raw.parse().ok())
        {
            Some(pref) => pref,
            None => EncryptPreference::NoPreference,
        };

        // Autocrypt-Level0: unknown attributes starting with an underscore can be safely ignored
        // Autocrypt-Level0: unknown attribute, treat the header as invalid
        if attributes.keys().any(|k| !k.starts_with('_')) {
            return Err(());
        }

        Ok(Aheader {
            addr,
            public_key,
            prefer_encrypt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rawkey() -> String {
        "xsBNBFzG3j0BCAC6iNhT8zydvCXi8LI/gFnkadMbfmSE/rTJskRRra/utGbLyDta/yTrJgWL7O3y/g4HdDW/dN2z26Y6W13IMzx9gLInn1KQZChtqWAcr/ReUucXcymwcfg1mdkBGk3TSLeLihN6CJx8Wsv8ig+kgAzte4f5rqEEAJVQ9WZHuti7UiYs6oRzqTo06CRe9owVXxzdMf0VDQtf7ZFm9dpzKKbhH7Lu8880iiotQ9/yRCkDGp9fNThsrLdZiK6OIAcIBAqi2rI89aS1dAmnRbktQieCx5izzyYkR1KvVL3gTTllHOzfKVEC2asmtWu2e4se/+O4WMIS1eGrn7GeWVb0Vwc5ABEBAAHNETxhQEBiLmV4YW1wbGUuZGU+wsCJBBABCAAzAhkBBQJcxt5FAhsDBAsJCAcGFQgJCgsCAxYCARYhBI4xxYKBgH3ANh5cufaKrc9mtiMLAAoJEPaKrc9mtiML938H/18F+3Wf9/JaAy/8hCO1v4S2PVBhxaKCokaNFtkfaMRne2l087LscCFPiFNyb4mv6Z3YeK8Xpxlp2sI0ecvdiqLUOGfnxS6tQrj+83EjtIrZ/hXOk1h121QFWH9Zg2VNHtODXjAgdLDC0NWUrclR0ZOqEDQHeo0ibTILdokVfXFN25wakPmGaYJP2y729cb1ve7RzvIvwn+Dddfxo3ao72rBfLi7l4NQ4S0KsY4cw+/6l5bRCKYCP77wZtvCwUvfVVosLdT43agtSiBI49+ayqvZ8OCvSJa61i+v81brTiEy9GBod4eAp45Ibsuemkw+gon4ZOvUXHTjwFB+h63MrozOwE0EXMbePQEIAL/vauf1zK8JgCu3V+G+SOX0iWw5xUlCPX+ERpBbWfwu3uAqn4wYXD3JDE/fVAF668xiV4eTPtlSUd5h0mn+G7uXMMOtkb+20SoEt50f8zw8TrL9t+ZsV11GKZWJpCar5AhXWsn6EEi8I2hLL5vn55ZZmHuGgN4jjmkRl3ToKCLhaXwTBjCJem7N5EH7F75wErEITa55v4Lb4Nfca7vnvtYrI1OA446xa8gHra0SINelTD09/JM/Fw4sWVPBaRZmJK/Tnu79N23No9XBUubmFPv1pNexZsQclicnTpt/BEWhiun7d6lfGB63K1aoHRTR1pcrWvBuALuuz0gqar2zlI0AEQEAAcLAdgQYAQgAIAUCXMbeRQIbDBYhBI4xxYKBgH3ANh5cufaKrc9mtiMLAAoJEPaKrc9mtiMLKSEIAIyLCRO2OyZ0IYRvRPpMn4p7E+7Pfcz/0mSkOy+1hshgJnqivXurm8zwGrwdMqeV4eslKR9H1RUdWGUQJNbtwmmjrt5DHpIhYHl5t3FpCBaGbV20Omo00Q38lBl9MtrmZkZw+ktEk6X+0xCKssMF+2MADkSOIufbR5HrDVB89VZOHCO9DeXvCUUAw2hyJiL/LHmLzJ40zYoTmb+F//f0k0j+tRdbkefyRoCmwG7YGiT+2hnCdgcezswnzah5J3ZKlrg7jOGo1LxtbvNUzxNBbC6S/aNgwm6qxo7xegRhmEl5uZ16zwyj4qz+xkjGy25Of5mWfUDoNw7OT7sjUbHOOMc=".into()
    }

    #[test]
    fn test_from_str() {
        let h: Aheader = format!(
            "addr=me@mail.com; prefer-encrypt=mutual; keydata={}",
            rawkey()
        )
        .parse()
        .expect("failed to parse");

        assert_eq!(h.addr, "me@mail.com");
        assert_eq!(h.prefer_encrypt, EncryptPreference::Mutual);
    }

    #[test]
    fn test_from_str_non_critical() {
        let raw = format!("addr=me@mail.com; _foo=one; _bar=two; keydata={}", rawkey());
        let h: Aheader = raw.parse().expect("failed to parse");

        assert_eq!(h.addr, "me@mail.com");
        assert_eq!(h.prefer_encrypt, EncryptPreference::NoPreference);
    }

    #[test]
    fn test_from_str_superflous_critical() {
        let raw = format!(
            "addr=me@mail.com; _foo=one; _bar=two; other=me; keydata={}",
            rawkey()
        );
        assert!(raw.parse::<Aheader>().is_err());
    }

    #[test]
    fn test_good_headers() {
        let fixed_header = "addr=a@b.example.org; prefer-encrypt=mutual; keydata=xsBNBFzG3j0BCAC6iNhT8zydvCXi8LI/gFnkadMbfmSE/rTJskRRra/utGbLyDta/yTrJgWL7O3y/g 4HdDW/dN2z26Y6W13IMzx9gLInn1KQZChtqWAcr/ReUucXcymwcfg1mdkBGk3TSLeLihN6CJx8Wsv8 ig+kgAzte4f5rqEEAJVQ9WZHuti7UiYs6oRzqTo06CRe9owVXxzdMf0VDQtf7ZFm9dpzKKbhH7Lu88 80iiotQ9/yRCkDGp9fNThsrLdZiK6OIAcIBAqi2rI89aS1dAmnRbktQieCx5izzyYkR1KvVL3gTTll HOzfKVEC2asmtWu2e4se/+O4WMIS1eGrn7GeWVb0Vwc5ABEBAAHNETxhQEBiLmV4YW1wbGUuZGU+ws CJBBABCAAzAhkBBQJcxt5FAhsDBAsJCAcGFQgJCgsCAxYCARYhBI4xxYKBgH3ANh5cufaKrc9mtiML AAoJEPaKrc9mtiML938H/18F+3Wf9/JaAy/8hCO1v4S2PVBhxaKCokaNFtkfaMRne2l087LscCFPiF Nyb4mv6Z3YeK8Xpxlp2sI0ecvdiqLUOGfnxS6tQrj+83EjtIrZ/hXOk1h121QFWH9Zg2VNHtODXjAg dLDC0NWUrclR0ZOqEDQHeo0ibTILdokVfXFN25wakPmGaYJP2y729cb1ve7RzvIvwn+Dddfxo3ao72 rBfLi7l4NQ4S0KsY4cw+/6l5bRCKYCP77wZtvCwUvfVVosLdT43agtSiBI49+ayqvZ8OCvSJa61i+v 81brTiEy9GBod4eAp45Ibsuemkw+gon4ZOvUXHTjwFB+h63MrozOwE0EXMbePQEIAL/vauf1zK8JgC u3V+G+SOX0iWw5xUlCPX+ERpBbWfwu3uAqn4wYXD3JDE/fVAF668xiV4eTPtlSUd5h0mn+G7uXMMOt kb+20SoEt50f8zw8TrL9t+ZsV11GKZWJpCar5AhXWsn6EEi8I2hLL5vn55ZZmHuGgN4jjmkRl3ToKC LhaXwTBjCJem7N5EH7F75wErEITa55v4Lb4Nfca7vnvtYrI1OA446xa8gHra0SINelTD09/JM/Fw4s WVPBaRZmJK/Tnu79N23No9XBUubmFPv1pNexZsQclicnTpt/BEWhiun7d6lfGB63K1aoHRTR1pcrWv BuALuuz0gqar2zlI0AEQEAAcLAdgQYAQgAIAUCXMbeRQIbDBYhBI4xxYKBgH3ANh5cufaKrc9mtiML AAoJEPaKrc9mtiMLKSEIAIyLCRO2OyZ0IYRvRPpMn4p7E+7Pfcz/0mSkOy+1hshgJnqivXurm8zwGr wdMqeV4eslKR9H1RUdWGUQJNbtwmmjrt5DHpIhYHl5t3FpCBaGbV20Omo00Q38lBl9MtrmZkZw+ktE k6X+0xCKssMF+2MADkSOIufbR5HrDVB89VZOHCO9DeXvCUUAw2hyJiL/LHmLzJ40zYoTmb+F//f0k0 j+tRdbkefyRoCmwG7YGiT+2hnCdgcezswnzah5J3ZKlrg7jOGo1LxtbvNUzxNBbC6S/aNgwm6qxo7x egRhmEl5uZ16zwyj4qz+xkjGy25Of5mWfUDoNw7OT7sjUbHOOMc=";

        let ah = Aheader::from_str(fixed_header).expect("failed to parse");
        assert_eq!(ah.addr, "a@b.example.org");
        assert_eq!(ah.prefer_encrypt, EncryptPreference::Mutual);

        let rendered = ah.to_string();
        assert_eq!(rendered, fixed_header);

        let ah = Aheader::from_str(&format!(" _foo; __FOO=BAR ;;; addr = a@b.example.org ;\r\n   prefer-encrypt = mutual ; keydata = {}", rawkey())).expect("failed to parse");
        assert_eq!(ah.addr, "a@b.example.org");
        assert_eq!(ah.prefer_encrypt, EncryptPreference::Mutual);

        Aheader::from_str(&format!(
            "addr=a@b.example.org; prefer-encrypt=ignoreUnknownValues; keydata={}",
            rawkey()
        ))
        .expect("failed to parse");

        Aheader::from_str(&format!("addr=a@b.example.org; keydata={}", rawkey()))
            .expect("failed to parse");
    }

    #[test]
    fn test_bad_headers() {
        assert!(Aheader::from_str("").is_err());
        assert!(Aheader::from_str("foo").is_err());
        assert!(Aheader::from_str("\n\n\n").is_err());
        assert!(Aheader::from_str(" ;;").is_err());
        assert!(Aheader::from_str("addr=a@t.de; unknwon=1; keydata=jau").is_err());
    }
}
