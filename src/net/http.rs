//! # HTTP module.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use mime::Mime;
use once_cell::sync::Lazy;

use crate::context::Context;
use crate::net::lookup_host_with_cache;
use crate::socks::Socks5Config;

static LETSENCRYPT_ROOT: Lazy<reqwest::tls::Certificate> = Lazy::new(|| {
    reqwest::tls::Certificate::from_der(include_bytes!(
        "../../assets/root-certificates/letsencrypt/isrgrootx1.der"
    ))
    .unwrap()
});

/// HTTP(S) GET response.
#[derive(Debug)]
pub struct Response {
    /// Response body.
    pub blob: Vec<u8>,

    /// MIME type extracted from the `Content-Type` header, if any.
    pub mimetype: Option<String>,

    /// Encoding extracted from the `Content-Type` header, if any.
    pub encoding: Option<String>,
}

/// Retrieves the text contents of URL using HTTP GET request.
pub async fn read_url(context: &Context, url: &str) -> Result<String> {
    Ok(read_url_inner(context, url).await?.text().await?)
}

/// Retrieves the binary contents of URL using HTTP GET request.
pub async fn read_url_blob(context: &Context, url: &str) -> Result<Response> {
    let response = read_url_inner(context, url).await?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<Mime>().ok());
    let mimetype = content_type
        .as_ref()
        .map(|mime| mime.essence_str().to_string());
    let encoding = content_type.as_ref().and_then(|mime| {
        mime.get_param(mime::CHARSET)
            .map(|charset| charset.as_str().to_string())
    });
    let blob: Vec<u8> = response.bytes().await?.into();
    Ok(Response {
        blob,
        mimetype,
        encoding,
    })
}

async fn read_url_inner(context: &Context, url: &str) -> Result<reqwest::Response> {
    // It is safe to use cached IP addresses
    // for HTTPS URLs, but for HTTP URLs
    // better resolve from scratch each time to prevent
    // cache poisoning attacks from having lasting effects.
    let load_cache = url.starts_with("https://");

    let client = get_client(context, load_cache).await?;
    let mut url = url.to_string();

    // Follow up to 10 http-redirects
    for _i in 0..10 {
        let response = client.get(&url).send().await?;
        if response.status().is_redirection() {
            let headers = response.headers();
            let header = headers
                .get_all("location")
                .iter()
                .last()
                .ok_or_else(|| anyhow!("Redirection doesn't have a target location"))?
                .to_str()?;
            info!(context, "Following redirect to {}", header);
            url = header.to_string();
            continue;
        }

        return Ok(response);
    }

    Err(anyhow!("Followed 10 redirections"))
}

struct CustomResolver {
    context: Context,

    /// Whether to return cached results or not.
    /// If resolver can be used for URLs
    /// without TLS, e.g. HTTP URLs from HTML email,
    /// this must be false. If TLS is used
    /// and certificate hostnames are checked,
    /// it is safe to load cache.
    load_cache: bool,
}

impl CustomResolver {
    fn new(context: Context, load_cache: bool) -> Self {
        Self {
            context,
            load_cache,
        }
    }
}

impl reqwest::dns::Resolve for CustomResolver {
    fn resolve(&self, hostname: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let context = self.context.clone();
        let load_cache = self.load_cache;
        Box::pin(async move {
            let port = 443; // Actual port does not matter.

            let socket_addrs =
                lookup_host_with_cache(&context, hostname.as_str(), port, "", load_cache).await;
            match socket_addrs {
                Ok(socket_addrs) => {
                    let addrs: reqwest::dns::Addrs = Box::new(socket_addrs.into_iter());

                    Ok(addrs)
                }
                Err(err) => Err(err.into()),
            }
        })
    }
}

pub(crate) async fn get_client(context: &Context, load_cache: bool) -> Result<reqwest::Client> {
    let socks5_config = Socks5Config::from_database(&context.sql).await?;
    let resolver = Arc::new(CustomResolver::new(context.clone(), load_cache));

    let builder = reqwest::ClientBuilder::new()
        .timeout(super::TIMEOUT)
        .add_root_certificate(LETSENCRYPT_ROOT.clone())
        .dns_resolver(resolver);

    let builder = if let Some(socks5_config) = socks5_config {
        let proxy = reqwest::Proxy::all(socks5_config.to_url())?;
        builder.proxy(proxy)
    } else {
        // Disable usage of "system" proxy configured via environment variables.
        // It is enabled by default in `reqwest`, see
        // <https://docs.rs/reqwest/0.11.14/reqwest/struct.ClientBuilder.html#method.no_proxy>
        // for documentation.
        builder.no_proxy()
    };
    Ok(builder.build()?)
}
