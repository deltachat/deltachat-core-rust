use std::collections::HashMap;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;

use crate::context::Context;
use crate::dc_tools::*;

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
struct Oauth2 {
    client_id: &'static str,
    get_code: &'static str,
    init_token: &'static str,
    refresh_token: &'static str,
    get_userinfo: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
struct Response {
    // Should always be there according to: https://www.oauth.com/oauth2-servers/access-tokens/access-token-response/
    // but previous code handled its abscense.
    access_token: Option<String>,
    token_type: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

pub fn dc_get_oauth2_url(
    context: &Context,
    addr: impl AsRef<str>,
    redirect_uri: impl AsRef<str>,
) -> Option<String> {
    if let Some(oauth2) = Oauth2::from_address(addr) {
        if context
            .sql
            .set_raw_config(
                context,
                "oauth2_pending_redirect_uri",
                Some(redirect_uri.as_ref()),
            )
            .is_err()
        {
            return None;
        }
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
    regenerate: bool,
) -> Option<String> {
    if let Some(oauth2) = Oauth2::from_address(addr) {
        let lock = context.oauth2_critical.clone();
        let _l = lock.lock().unwrap();

        // read generated token
        if !regenerate && !is_expired(context) {
            let access_token = context.sql.get_raw_config(context, "oauth2_access_token");
            if access_token.is_some() {
                // success
                return access_token;
            }
        }

        let refresh_token = context.sql.get_raw_config(context, "oauth2_refresh_token");
        let refresh_token_for = context
            .sql
            .get_raw_config(context, "oauth2_refresh_token_for")
            .unwrap_or_else(|| "unset".into());

        let (redirect_uri, token_url, update_redirect_uri_on_success) =
            if refresh_token.is_none() || refresh_token_for != code.as_ref() {
                info!(context, "Generate OAuth2 refresh_token and access_token...",);
                (
                    context
                        .sql
                        .get_raw_config(context, "oauth2_pending_redirect_uri")
                        .unwrap_or_else(|| "unset".into()),
                    oauth2.init_token,
                    true,
                )
            } else {
                info!(
                    context,
                    "Regenerate OAuth2 access_token by refresh_token...",
                );
                (
                    context
                        .sql
                        .get_raw_config(context, "oauth2_redirect_uri")
                        .unwrap_or_else(|| "unset".into()),
                    oauth2.refresh_token,
                    false,
                )
            };
        let mut token_url = replace_in_uri(&token_url, "$CLIENT_ID", oauth2.client_id);
        token_url = replace_in_uri(&token_url, "$REDIRECT_URI", &redirect_uri);
        token_url = replace_in_uri(&token_url, "$CODE", code.as_ref());
        if let Some(ref token) = refresh_token {
            token_url = replace_in_uri(&token_url, "$REFRESH_TOKEN", token);
        }

        let response = reqwest::Client::new().post(&token_url).send();
        if response.is_err() {
            warn!(
                context,
                "Error calling OAuth2 at {}: {:?}", token_url, response
            );
            return None;
        }
        let mut response = response.unwrap();
        if !response.status().is_success() {
            warn!(
                context,
                "Error calling OAuth2 at {}: {:?}",
                token_url,
                response.status()
            );
            return None;
        }

        let parsed: reqwest::Result<Response> = response.json();
        if parsed.is_err() {
            warn!(
                context,
                "Failed to parse OAuth2 JSON response from {}: error: {:?}", token_url, parsed
            );
            return None;
        }
        println!("response: {:?}", &parsed);
        let response = parsed.unwrap();
        if let Some(ref token) = response.refresh_token {
            context
                .sql
                .set_raw_config(context, "oauth2_refresh_token", Some(token))
                .ok();
            context
                .sql
                .set_raw_config(context, "oauth2_refresh_token_for", Some(code.as_ref()))
                .ok();
        }

        // after that, save the access token.
        // if it's unset, we may get it in the next round as we have the refresh_token now.
        if let Some(ref token) = response.access_token {
            context
                .sql
                .set_raw_config(context, "oauth2_access_token", Some(token))
                .ok();
            let expires_in = response
                .expires_in
                // refresh a bet before
                .map(|t| time() + t as i64 - 5)
                .unwrap_or_else(|| 0);
            context
                .sql
                .set_raw_config_int64(context, "oauth2_timestamp_expires", expires_in)
                .ok();

            if update_redirect_uri_on_success {
                context
                    .sql
                    .set_raw_config(context, "oauth2_redirect_uri", Some(redirect_uri.as_ref()))
                    .ok();
            }
        } else {
            warn!(context, "Failed to find OAuth2 access token");
        }

        response.access_token
    } else {
        warn!(context, "Internal OAuth2 error: 2");

        None
    }
}

pub fn dc_get_oauth2_addr(
    context: &Context,
    addr: impl AsRef<str>,
    code: impl AsRef<str>,
) -> Option<String> {
    let oauth2 = Oauth2::from_address(addr.as_ref())?;
    oauth2.get_userinfo?;

    if let Some(access_token) =
        dc_get_oauth2_access_token(context, addr.as_ref(), code.as_ref(), false)
    {
        let addr_out = oauth2.get_addr(context, access_token);
        if addr_out.is_none() {
            // regenerate
            if let Some(access_token) = dc_get_oauth2_access_token(context, addr, code, true) {
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
            warn!(context, "Error getting userinfo: {:?}", response);
            return None;
        }
        let mut response = response.unwrap();
        if !response.status().is_success() {
            warn!(context, "Error getting userinfo: {:?}", response.status());
            return None;
        }

        let parsed: reqwest::Result<HashMap<String, String>> = response.json();
        if parsed.is_err() {
            warn!(
                context,
                "Failed to parse userinfo JSON response: {:?}", parsed
            );
            return None;
        }
        if let Ok(response) = parsed {
            let addr = response.get("email");
            if addr.is_none() {
                warn!(context, "E-mail missing in userinfo.");
            }

            addr.map(|addr| addr.to_string())
        } else {
            warn!(context, "Failed to parse userinfo.");
            None
        }
    }
}

fn is_expired(context: &Context) -> bool {
    let expire_timestamp = context
        .sql
        .get_raw_config_int64(context, "oauth2_timestamp_expires")
        .unwrap_or_default();

    if expire_timestamp <= 0 {
        return false;
    }
    if expire_timestamp > time() {
        return false;
    }

    true
}

fn replace_in_uri(uri: impl AsRef<str>, key: impl AsRef<str>, value: impl AsRef<str>) -> String {
    let value_urlencoded = utf8_percent_encode(value.as_ref(), NON_ALPHANUMERIC).to_string();
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
            "helloa%2Db%20c"
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
