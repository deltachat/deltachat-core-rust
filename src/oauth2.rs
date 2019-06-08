use std::ffi::CString;

use percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use serde::Deserialize;

use crate::context::Context;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::imap::DC_REGENERATE;

const OAUTH2_GMAIL: Oauth2 = Oauth2 {
    client_id: "959970109878-4mvtgf6feshskf7695nfln6002mom908.apps.googleusercontent.com",
    get_code: "https://accounts.google.com/o/oauth2/auth?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F%20email&access_type=offline",
    init_token: "https://accounts.google.com/o/oauth2/token?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&code=$CODE&grant_type=authorization_code",
    refresh_token: "https://accounts.google.com/o/oauth2/token?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&refresh_token=$REFRESH_TOKEN&grant_type=refresh_token",
    get_userinfo: Some("https://www.googleapis.com/oauth2/v1/userinfo?alt=json&access_token=$ACCESS_TOKEN"),
};

const OAUTH2_YANDEX: Oauth2 = Oauth2 {
    client_id: "c4d0b6735fc8420a816d7e1303469341",
    get_code: "https://oauth.yandex.com/authorize?client_id=$CLIENT_ID&response_type=code&scope=mail%3Aimap_full%20mail%3Asmtp&force_confirm=true",
    init_token: "https://oauth.yandex.com/token?grant_type=authorization_code&code=$CODE&client_id=$CLIENT_ID&client_secret=58b8c6e94cf44fbe952da8511955dacf",
    refresh_token: "https://oauth.yandex.com/token?grant_type=refresh_token&refresh_token=$REFRESH_TOKEN&client_id=$CLIENT_ID&client_secret=58b8c6e94cf44fbe952da8511955dacf",
    get_userinfo: None,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oauth2 {
    client_id: &'static str,
    get_code: &'static str,
    init_token: &'static str,
    refresh_token: &'static str,
    get_userinfo: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    // Should always be there according to: https://www.oauth.com/oauth2-servers/access-tokens/access-token-response/
    // but previous code handled its abscense.
    access_token: Option<String>,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserinfoResponse {
    email: Option<String>,
}

pub fn dc_get_oauth2_url(
    context: &Context,
    addr: impl AsRef<str>,
    redirect_uri: impl AsRef<str>,
) -> Option<String> {
    if let Some(oauth2) = Oauth2::from_address(addr) {
        set_config(
            context,
            "oauth2_pending_redirect_uri",
            redirect_uri.as_ref(),
        );
        let oauth2_url = replace_in_uri(&oauth2.get_code, "$CLIENT_ID", &oauth2.client_id);
        let oauth2_url = replace_in_uri(&oauth2_url, "$REDIRECT_URI", redirect_uri.as_ref());

        Some(oauth2_url)
    } else {
        None
    }
}

// The following function may block due http-requests;
// must not be called from the main thread or by the ui!
pub fn dc_get_oauth2_access_token(
    context: &Context,
    addr: impl AsRef<str>,
    code: impl AsRef<str>,
    flags: usize,
) -> Option<String> {
    if let Some(oauth2) = Oauth2::from_address(addr) {
        let lock = context.oauth2_critical.clone();
        let _l = lock.lock().unwrap();

        // read generated token
        if 0 == (flags & DC_REGENERATE) && !is_expired(context) {
            let access_token = get_config(context, "oauth2_access_token");
            if access_token.is_some() {
                // success
                return access_token;
            }
        }

        // generate new token: build & call auth url
        let refresh_token = get_config(context, "oauth2_refresh_token");
        let refresh_token_for =
            get_config(context, "oauth2_refresh_token_for").unwrap_or_else(|| "unset".into());

        let (redirect_uri, token_url, update_redirect_uri_on_success) =
            if refresh_token.is_none() || refresh_token_for != code.as_ref() {
                info!(
                    context,
                    0, "Generate OAuth2 refresh_token and access_token...",
                );
                (
                    get_config(context, "oauth2_pending_redirect_uri")
                        .unwrap_or_else(|| "unset".into()),
                    oauth2.init_token,
                    true,
                )
            } else {
                info!(
                    context,
                    0, "Regenerate OAuth2 access_token by refresh_token...",
                );
                (
                    get_config(context, "oauth2_redirect_uri").unwrap_or_else(|| "unset".into()),
                    oauth2.refresh_token,
                    false,
                )
            };

        // create url to query as `domain?param1=value1&param2=value2`
        // (this format allows easier printing, handling and is compatible with GET)
        let mut token_url = replace_in_uri(&token_url, "$CLIENT_ID", oauth2.client_id);
        token_url = replace_in_uri(&token_url, "$REDIRECT_URI", &redirect_uri);
        token_url = replace_in_uri(&token_url, "$CODE", code.as_ref());
        if let Some(ref token) = refresh_token {
            token_url = replace_in_uri(&token_url, "$REFRESH_TOKEN", token);
        }

        // split url into domain an parameters and POST
        let parts: Vec<&str> = token_url.split('?').collect();
        let post_url = parts[0];
        let parts = parts[1].split('&');
        let mut post_param: Vec<(&str,&str)> = Vec::new();
        for part in parts {
            let part: Vec<&str> = part.split('=').collect();
            post_param.push((part[0], part[1]));
        }
        println!("{} {:#?}", post_url, post_param);

        let response = reqwest::Client::new().post(post_url).form(&post_param).send();
        if response.is_err() {
            warn!(context, 0, "Error calling OAuth2 at {}: {:?}", token_url, response);
            return None;
        }

        let mut response = response.unwrap();
        if !response.status().is_success() {
            warn!(context, 0, "Error calling OAuth2 at {}: {:?}: {:?}",
                  token_url, response.status(), response.text()
            );
            return None;
        }

        let response: reqwest::Result<TokenResponse> = response.json();
        if response.is_err() {
            warn!(
                context,
                0, "Failed to parse OAuth2 JSON response from {}: error: {:?}", token_url, response
            );
            return None;
        }
        println!("response: {:?}", &response);

        // see whats in the json we got
        let response = response.unwrap();
        if let Some(ref token) = response.refresh_token {
            set_config(context, "oauth2_refresh_token", token);
            set_config(context, "oauth2_refresh_token_for", code.as_ref());
        }

        // after that, save the access token.
        // if it's unset, we may get it in the next round as we have the refresh_token now.
        if let Some(ref token) = response.access_token {
            set_config(context, "oauth2_access_token", token);
            let expires_in = response
                .expires_in
                // refresh a bet before
                .map(|t| time() + t as i64 - 5)
                .unwrap_or_else(|| 0);
            set_config_int64(context, "oauth2_timestamp_expires", expires_in);

            if update_redirect_uri_on_success {
                set_config(context, "oauth2_redirect_uri", redirect_uri.as_ref());
            }
        } else {
            warn!(context, 0, "Failed to find OAuth2 access token");
        }

        response.access_token
    } else {
        warn!(context, 0, "Internal OAuth2 error: 2");

        None
    }
}

pub fn dc_get_oauth2_addr(
    context: &Context,
    addr: impl AsRef<str>,
    code: impl AsRef<str>,
) -> Option<String> {
    let oauth2 = Oauth2::from_address(addr.as_ref());
    if oauth2.is_none() {
        return None;
    }
    let oauth2 = oauth2.unwrap();
    if oauth2.get_userinfo.is_none() {
        return None;
    }

    if let Some(access_token) = dc_get_oauth2_access_token(context, addr.as_ref(), code.as_ref(), 0)
    {
        let addr_out = oauth2.get_addr(context, access_token);
        if addr_out.is_none() {
            // regenerate
            if let Some(access_token) = dc_get_oauth2_access_token(context, addr, code, 0x1) {
                oauth2.get_addr(context, access_token)
            } else {
                None
            }
        } else {
            addr_out
        }
    } else {
        None
    }
}

impl Oauth2 {
    fn from_address(addr: impl AsRef<str>) -> Option<Self> {
        let addr_normalized = normalize_addr(addr.as_ref());
        if let Some(domain) = addr_normalized
            .find('@')
            .map(|index| addr_normalized.split_at(index + 1).1)
        {
            match domain {
                "gmail.com" | "googlemail.com" => Some(OAUTH2_GMAIL),
                "yandex.com" | "yandex.ru" | "yandex.ua" => Some(OAUTH2_YANDEX),
                _ => None,
            }
        } else {
            None
        }
    }

    fn get_addr(&self, context: &Context, access_token: impl AsRef<str>) -> Option<String> {
        let userinfo_url = self.get_userinfo.unwrap_or_else(|| "");
        let userinfo_url = replace_in_uri(&userinfo_url, "$ACCESS_TOKEN", access_token);

        // should returns sth. as
        // {
        //   "id": "100000000831024152393",
        //   "email": "NAME@gmail.com",
        //   "verified_email": true,
        //   "picture": "https://lh4.googleusercontent.com/-Gj5jh_9R0BY/AAAAAAAAAAI/AAAAAAAAAAA/IAjtjfjtjNA/photo.jpg"
        // }
        let response = reqwest::Client::new().get(&userinfo_url).send();
        if response.is_err() {
            warn!(context, 0, "Error getting userinfo: {:?}", response);
            return None;
        }

        let mut response = response.unwrap();
        if !response.status().is_success() {
            warn!(context, 0, "Error getting userinfo: {:?}", response.status());
            return None;
        }

        let response: reqwest::Result<UserinfoResponse> = response.json();
        if response.is_err() {
            warn!(context, 0, "Failed to parse userinfo JSON response: {:?}", response);
            return None;
        }

        let response = response.unwrap();
        if response.email.is_none() {
            return None;
        }

        if let Some(email) = response.email {
            if !email.is_empty() {
                info!(context, 0, "Got userinfo: {}", email);
                return Some(email);
            }
        }

        None
    }
}

fn get_config(context: &Context, key: &str) -> Option<String> {
    let key_c = CString::new(key).unwrap();
    let res = unsafe {
        dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            key_c.as_ptr(),
            std::ptr::null(),
        )
    };
    if res.is_null() {
        return None;
    }

    Some(to_string(res))
}

fn set_config(context: &Context, key: &str, value: &str) {
    let key_c = CString::new(key).unwrap();
    let value_c = CString::new(value).unwrap();
    unsafe {
        dc_sqlite3_set_config(
            context,
            &context.sql.clone().read().unwrap(),
            key_c.as_ptr(),
            value_c.as_ptr(),
        )
    };
}

fn set_config_int64(context: &Context, key: &str, value: i64) {
    let key_c = CString::new(key).unwrap();
    unsafe {
        dc_sqlite3_set_config_int64(
            context,
            &context.sql.clone().read().unwrap(),
            key_c.as_ptr(),
            value,
        )
    };
}

fn is_expired(context: &Context) -> bool {
    let expire_timestamp = dc_sqlite3_get_config_int64(
        context,
        &context.sql.clone().read().unwrap(),
        b"oauth2_timestamp_expires\x00" as *const u8 as *const libc::c_char,
        0i32 as int64_t,
    );

    if expire_timestamp <= 0 {
        return false;
    }
    if expire_timestamp > time() {
        return false;
    }

    true
}

fn replace_in_uri(uri: impl AsRef<str>, key: impl AsRef<str>, value: impl AsRef<str>) -> String {
    let value_urlencoded = utf8_percent_encode(value.as_ref(), DEFAULT_ENCODE_SET).to_string();
    uri.as_ref().replace(key.as_ref(), &value_urlencoded)
}

fn normalize_addr(addr: &str) -> &str {
    let normalized = addr.trim();
    normalized.trim_start_matches("mailto:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_addr() {
        assert_eq!(normalize_addr(" hello@mail.de  "), "hello@mail.de");
        assert_eq!(normalize_addr("mailto:hello@mail.de  "), "hello@mail.de");
    }

    #[test]
    fn test_replace_in_uri() {
        assert_eq!(
            replace_in_uri("helloworld", "world", "a-b c"),
            "helloa-b%20c"
        );
    }

    #[test]
    fn test_oauth_from_address() {
        assert_eq!(Oauth2::from_address("hello@gmail.com"), Some(OAUTH2_GMAIL));
        assert_eq!(
            Oauth2::from_address("hello@googlemail.com"),
            Some(OAUTH2_GMAIL)
        );
        assert_eq!(
            Oauth2::from_address("hello@yandex.com"),
            Some(OAUTH2_YANDEX)
        );
        assert_eq!(Oauth2::from_address("hello@yandex.ru"), Some(OAUTH2_YANDEX));

        assert_eq!(Oauth2::from_address("hello@web.de"), None);
    }
}
