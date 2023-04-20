//! # HTTP module.

use std::time::Duration;

use anyhow::{anyhow, Result};
use mime::Mime;

use crate::context::Context;
use crate::socks::Socks5Config;

const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP(S) GET response.
#[derive(Debug)]
pub struct Response {
    /// Response body.
    pub blob: Vec<u8>,

    /// MIME type exntracted from the `Content-Type` header, if any.
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
    let socks5_config = Socks5Config::from_database(&context.sql).await?;
    let client = get_client(socks5_config)?;
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

pub(crate) fn get_client(socks5_config: Option<Socks5Config>) -> Result<reqwest::Client> {
    let builder = reqwest::ClientBuilder::new().timeout(HTTP_TIMEOUT);
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
