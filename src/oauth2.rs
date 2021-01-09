//! OAuth 2 module

use std::collections::HashMap;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;

use crate::context::Context;
use crate::dc_tools::*;
use crate::provider;
use crate::provider::Oauth2Authorizer;

const OAUTH2_GMAIL: Oauth2 = Oauth2 {
    // see https://developers.google.com/identity/protocols/OAuth2InstalledApp
    client_id: "959970109878-4mvtgf6feshskf7695nfln6002mom908.apps.googleusercontent.com",
    get_code: "https://accounts.google.com/o/oauth2/auth?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F%20email&access_type=offline",
    init_token: "https://accounts.google.com/o/oauth2/token?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&code=$CODE&grant_type=authorization_code",
    refresh_token: "https://accounts.google.com/o/oauth2/token?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&refresh_token=$REFRESH_TOKEN&grant_type=refresh_token",
    get_userinfo: Some("https://www.googleapis.com/oauth2/v1/userinfo?alt=json&access_token=$ACCESS_TOKEN"),
};

const OAUTH2_YANDEX: Oauth2 = Oauth2 {
    // see https://tech.yandex.com/oauth/doc/dg/reference/auto-code-client-docpage/
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

/// OAuth 2 Access Token Response
#[derive(Debug, Deserialize)]
struct Response {
    // Should always be there according to: https://www.oauth.com/oauth2-servers/access-tokens/access-token-response/
    // but previous code handled its abscense.
    access_token: Option<String>,
    token_type: String,
    /// Duration of time the token is granted for, in seconds
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

pub async fn dc_get_oauth2_url(
    context: &Context,
    addr: impl AsRef<str>,
    redirect_uri: impl AsRef<str>,
) -> Option<String> {
    if let Some(oauth2) = Oauth2::from_address(addr).await {
        if context
            .sql
            .set_raw_config(
                context,
                "oauth2_pending_redirect_uri",
                Some(redirect_uri.as_ref()),
            )
            .await
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

pub async fn dc_get_oauth2_access_token(
    context: &Context,
    addr: impl AsRef<str>,
    code: impl AsRef<str>,
    regenerate: bool,
) -> Option<String> {
    if let Some(oauth2) = Oauth2::from_address(addr).await {
        let lock = context.oauth2_mutex.lock().await;

        // read generated token
        if !regenerate && !is_expired(context).await {
            let access_token = context
                .sql
                .get_raw_config(context, "oauth2_access_token")
                .await;
            if access_token.is_some() {
                // success
                return access_token;
            }
        }

        // generate new token: build & call auth url
        let refresh_token = context
            .sql
            .get_raw_config(context, "oauth2_refresh_token")
            .await;
        let refresh_token_for = context
            .sql
            .get_raw_config(context, "oauth2_refresh_token_for")
            .await
            .unwrap_or_else(|| "unset".into());

        let (redirect_uri, token_url, update_redirect_uri_on_success) =
            if refresh_token.is_none() || refresh_token_for != code.as_ref() {
                info!(context, "Generate OAuth2 refresh_token and access_token...",);
                (
                    context
                        .sql
                        .get_raw_config(context, "oauth2_pending_redirect_uri")
                        .await
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
                        .await
                        .unwrap_or_else(|| "unset".into()),
                    oauth2.refresh_token,
                    false,
                )
            };

        // to allow easier specification of different configurations,
        // token_url is in GET-method-format, sth. as https://domain?param1=val1&param2=val2 -
        // convert this to POST-format ...
        let mut parts = token_url.splitn(2, '?');
        let post_url = parts.next().unwrap_or_default();
        let post_args = parts.next().unwrap_or_default();
        let mut post_param = HashMap::new();
        for key_value_pair in post_args.split('&') {
            let mut parts = key_value_pair.splitn(2, '=');
            let key = parts.next().unwrap_or_default();
            let mut value = parts.next().unwrap_or_default();

            if value == "$CLIENT_ID" {
                value = oauth2.client_id;
            } else if value == "$REDIRECT_URI" {
                value = &redirect_uri;
            } else if value == "$CODE" {
                value = code.as_ref();
            } else if value == "$REFRESH_TOKEN" && refresh_token.is_some() {
                value = refresh_token.as_ref().unwrap();
            }

            post_param.insert(key, value);
        }

        // ... and POST
        let mut req = surf::post(post_url).build();
        if let Err(err) = req.body_form(&post_param) {
            warn!(context, "Error calling OAuth2 at {}: {:?}", token_url, err);
            return None;
        }

        let client = surf::Client::new();
        let parsed: Result<Response, _> = client.recv_json(req).await;
        if parsed.is_err() {
            warn!(
                context,
                "Failed to parse OAuth2 JSON response from {}: error: {:?}", token_url, parsed
            );
            return None;
        }

        // update refresh_token if given, typically on the first round, but we update it later as well.
        let response = parsed.unwrap();
        if let Some(ref token) = response.refresh_token {
            context
                .sql
                .set_raw_config(context, "oauth2_refresh_token", Some(token))
                .await
                .ok();
            context
                .sql
                .set_raw_config(context, "oauth2_refresh_token_for", Some(code.as_ref()))
                .await
                .ok();
        }

        // after that, save the access token.
        // if it's unset, we may get it in the next round as we have the refresh_token now.
        if let Some(ref token) = response.access_token {
            context
                .sql
                .set_raw_config(context, "oauth2_access_token", Some(token))
                .await
                .ok();
            let expires_in = response
                .expires_in
                // refresh a bit before
                .map(|t| time() + t as i64 - 5)
                .unwrap_or_else(|| 0);
            context
                .sql
                .set_raw_config_int64(context, "oauth2_timestamp_expires", expires_in)
                .await
                .ok();

            if update_redirect_uri_on_success {
                context
                    .sql
                    .set_raw_config(context, "oauth2_redirect_uri", Some(redirect_uri.as_ref()))
                    .await
                    .ok();
            }
        } else {
            warn!(context, "Failed to find OAuth2 access token");
        }

        drop(lock);

        response.access_token
    } else {
        warn!(context, "Internal OAuth2 error: 2");

        None
    }
}

pub async fn dc_get_oauth2_addr(
    context: &Context,
    addr: impl AsRef<str>,
    code: impl AsRef<str>,
) -> Option<String> {
    let oauth2 = Oauth2::from_address(addr.as_ref()).await?;
    oauth2.get_userinfo?;

    if let Some(access_token) =
        dc_get_oauth2_access_token(context, addr.as_ref(), code.as_ref(), false).await
    {
        let addr_out = oauth2.get_addr(context, access_token).await;
        if addr_out.is_none() {
            // regenerate
            if let Some(access_token) = dc_get_oauth2_access_token(context, addr, code, true).await
            {
                oauth2.get_addr(context, access_token).await
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
    async fn from_address(addr: impl AsRef<str>) -> Option<Self> {
        let addr_normalized = normalize_addr(addr.as_ref());
        if let Some(domain) = addr_normalized
            .find('@')
            .map(|index| addr_normalized.split_at(index + 1).1)
        {
            if let Some(oauth2_authorizer) = provider::get_provider_info(&domain)
                .await
                .and_then(|provider| provider.oauth2_authorizer.as_ref())
            {
                return Some(match oauth2_authorizer {
                    Oauth2Authorizer::Gmail => OAUTH2_GMAIL,
                    Oauth2Authorizer::Yandex => OAUTH2_YANDEX,
                });
            }
        }
        None
    }

    async fn get_addr(&self, context: &Context, access_token: impl AsRef<str>) -> Option<String> {
        let userinfo_url = self.get_userinfo.unwrap_or("");
        let userinfo_url = replace_in_uri(&userinfo_url, "$ACCESS_TOKEN", access_token);

        // should returns sth. as
        // {
        //   "id": "100000000831024152393",
        //   "email": "NAME@gmail.com",
        //   "verified_email": true,
        //   "picture": "https://lh4.googleusercontent.com/-Gj5jh_9R0BY/AAAAAAAAAAI/AAAAAAAAAAA/IAjtjfjtjNA/photo.jpg"
        // }
        let response: Result<HashMap<String, serde_json::Value>, surf::Error> =
            surf::get(userinfo_url).recv_json().await;
        if response.is_err() {
            warn!(context, "Error getting userinfo: {:?}", response);
            return None;
        }

        let parsed = response.unwrap();
        // CAVE: serde_json::Value.as_str() removes the quotes of json-strings
        // but serde_json::Value.to_string() does not!
        if let Some(addr) = parsed.get("email") {
            if let Some(s) = addr.as_str() {
                Some(s.to_string())
            } else {
                warn!(context, "E-mail in userinfo is not a string: {}", addr);
                None
            }
        } else {
            warn!(context, "E-mail missing in userinfo.");
            None
        }
    }
}

async fn is_expired(context: &Context) -> bool {
    let expire_timestamp = context
        .sql
        .get_raw_config_int64(context, "oauth2_timestamp_expires")
        .await
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

    use crate::test_utils::*;

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

    #[async_std::test]
    async fn test_oauth_from_address() {
        assert_eq!(
            Oauth2::from_address("hello@gmail.com").await,
            Some(OAUTH2_GMAIL)
        );
        assert_eq!(
            Oauth2::from_address("hello@googlemail.com").await,
            Some(OAUTH2_GMAIL)
        );
        assert_eq!(
            Oauth2::from_address("hello@yandex.com").await,
            Some(OAUTH2_YANDEX)
        );
        assert_eq!(
            Oauth2::from_address("hello@yandex.ru").await,
            Some(OAUTH2_YANDEX)
        );

        assert_eq!(Oauth2::from_address("hello@web.de").await, None);
    }

    #[async_std::test]
    async fn test_oauth_from_mx() {
        assert_eq!(
            Oauth2::from_address("hello@google.com").await,
            Some(OAUTH2_GMAIL)
        );
    }

    #[async_std::test]
    async fn test_dc_get_oauth2_addr() {
        let ctx = TestContext::new().await;
        let addr = "dignifiedquire@gmail.com";
        let code = "fail";
        let res = dc_get_oauth2_addr(&ctx.ctx, addr, code).await;
        // this should fail as it is an invalid password
        assert_eq!(res, None);
    }

    #[async_std::test]
    async fn test_dc_get_oauth2_url() {
        let ctx = TestContext::new().await;
        let addr = "dignifiedquire@gmail.com";
        let redirect_uri = "chat.delta:/com.b44t.messenger";
        let res = dc_get_oauth2_url(&ctx.ctx, addr, redirect_uri).await;

        assert_eq!(res, Some("https://accounts.google.com/o/oauth2/auth?client_id=959970109878%2D4mvtgf6feshskf7695nfln6002mom908%2Eapps%2Egoogleusercontent%2Ecom&redirect_uri=chat%2Edelta%3A%2Fcom%2Eb44t%2Emessenger&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F%20email&access_type=offline".into()));
    }

    #[async_std::test]
    async fn test_dc_get_oauth2_token() {
        let ctx = TestContext::new().await;
        let addr = "dignifiedquire@gmail.com";
        let code = "fail";
        let res = dc_get_oauth2_access_token(&ctx.ctx, addr, code, false).await;
        // this should fail as it is an invalid password
        assert_eq!(res, None);
    }
}
