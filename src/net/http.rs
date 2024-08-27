//! # HTTP module.

use anyhow::{anyhow, bail, Context as _, Result};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;
use mime::Mime;
use serde::Serialize;

use crate::context::Context;
use crate::net::session::SessionStream;
use crate::net::tls::wrap_tls;
use crate::socks::Socks5Config;

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
    let response = read_url_blob(context, url).await?;
    let text = String::from_utf8_lossy(&response.blob);
    Ok(text.to_string())
}

async fn get_http_sender<B>(
    context: &Context,
    parsed_url: hyper::Uri,
) -> Result<hyper::client::conn::http1::SendRequest<B>>
where
    B: hyper::body::Body + 'static + Send,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let scheme = parsed_url.scheme_str().context("URL has no scheme")?;
    let host = parsed_url.host().context("URL has no host")?;
    let socks5_config_opt = Socks5Config::from_database(&context.sql).await?;

    let stream: Box<dyn SessionStream> = match scheme {
        "http" => {
            let port = parsed_url.port_u16().unwrap_or(80);

            // It is safe to use cached IP addresses
            // for HTTPS URLs, but for HTTP URLs
            // better resolve from scratch each time to prevent
            // cache poisoning attacks from having lasting effects.
            let load_cache = false;
            if let Some(socks5_config) = socks5_config_opt {
                let socks5_stream = socks5_config
                    .connect(context, host, port, load_cache)
                    .await?;
                Box::new(socks5_stream)
            } else {
                let tcp_stream = crate::net::connect_tcp(context, host, port, load_cache).await?;
                Box::new(tcp_stream)
            }
        }
        "https" => {
            let port = parsed_url.port_u16().unwrap_or(443);
            let load_cache = true;
            let strict_tls = true;

            if let Some(socks5_config) = socks5_config_opt {
                let socks5_stream = socks5_config
                    .connect(context, host, port, load_cache)
                    .await?;
                let tls_stream = wrap_tls(strict_tls, host, &[], socks5_stream).await?;
                Box::new(tls_stream)
            } else {
                let tcp_stream = crate::net::connect_tcp(context, host, port, load_cache).await?;
                let tls_stream = wrap_tls(strict_tls, host, &[], tcp_stream).await?;
                Box::new(tls_stream)
            }
        }
        _ => bail!("Unknown URL scheme"),
    };

    let io = TokioIo::new(stream);
    let (sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(conn);

    Ok(sender)
}

/// Retrieves the binary contents of URL using HTTP GET request.
pub async fn read_url_blob(context: &Context, url: &str) -> Result<Response> {
    let mut url = url.to_string();

    // Follow up to 10 http-redirects
    for _i in 0..10 {
        let parsed_url = url
            .parse::<hyper::Uri>()
            .with_context(|| format!("Failed to parse URL {url:?}"))?;

        let mut sender = get_http_sender(context, parsed_url.clone()).await?;
        let authority = parsed_url
            .authority()
            .context("URL has no authority")?
            .clone();

        let req = hyper::Request::builder()
            .uri(parsed_url.path())
            .header(hyper::header::HOST, authority.as_str())
            .body(http_body_util::Empty::<Bytes>::new())?;
        let response = sender.send_request(req).await?;

        if response.status().is_redirection() {
            let header = response
                .headers()
                .get_all("location")
                .iter()
                .last()
                .ok_or_else(|| anyhow!("Redirection doesn't have a target location"))?
                .to_str()?;
            info!(context, "Following redirect to {}", header);
            url = header.to_string();
            continue;
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());
        let mimetype = content_type
            .as_ref()
            .map(|mime| mime.essence_str().to_string());
        let encoding = content_type.as_ref().and_then(|mime| {
            mime.get_param(mime::CHARSET)
                .map(|charset| charset.as_str().to_string())
        });
        let body = response.collect().await?.to_bytes();
        let blob: Vec<u8> = body.to_vec();
        return Ok(Response {
            blob,
            mimetype,
            encoding,
        });
    }

    Err(anyhow!("Followed 10 redirections"))
}

/// Sends an empty POST request to the URL.
///
/// Returns response text and whether request was successful or not.
///
/// Does not follow redirects.
pub(crate) async fn post_empty(context: &Context, url: &str) -> Result<(String, bool)> {
    let parsed_url = url
        .parse::<hyper::Uri>()
        .with_context(|| format!("Failed to parse URL {url:?}"))?;
    let scheme = parsed_url.scheme_str().context("URL has no scheme")?;
    if scheme != "https" {
        bail!("POST requests to non-HTTPS URLs are not allowed");
    }

    let mut sender = get_http_sender(context, parsed_url.clone()).await?;
    let authority = parsed_url
        .authority()
        .context("URL has no authority")?
        .clone();
    let req = hyper::Request::post(parsed_url.path())
        .header(hyper::header::HOST, authority.as_str())
        .body(http_body_util::Empty::<Bytes>::new())?;

    let response = sender.send_request(req).await?;

    let response_status = response.status();
    let body = response.collect().await?.to_bytes();
    let text = String::from_utf8_lossy(&body);
    let response_text = text.to_string();

    Ok((response_text, response_status.is_success()))
}

/// Posts string to the given URL.
///
/// Returns true if successful HTTP response code was returned.
///
/// Does not follow redirects.
#[allow(dead_code)]
pub(crate) async fn post_string(context: &Context, url: &str, body: String) -> Result<bool> {
    let parsed_url = url
        .parse::<hyper::Uri>()
        .with_context(|| format!("Failed to parse URL {url:?}"))?;
    let scheme = parsed_url.scheme_str().context("URL has no scheme")?;
    if scheme != "https" {
        bail!("POST requests to non-HTTPS URLs are not allowed");
    }

    let mut sender = get_http_sender(context, parsed_url.clone()).await?;
    let authority = parsed_url
        .authority()
        .context("URL has no authority")?
        .clone();

    let request = hyper::Request::post(parsed_url.path())
        .header(hyper::header::HOST, authority.as_str())
        .body(body)?;
    let response = sender.send_request(request).await?;

    Ok(response.status().is_success())
}

/// Sends a POST request with x-www-form-urlencoded data.
///
/// Does not follow redirects.
pub(crate) async fn post_form<T: Serialize + ?Sized>(
    context: &Context,
    url: &str,
    form: &T,
) -> Result<Bytes> {
    let parsed_url = url
        .parse::<hyper::Uri>()
        .with_context(|| format!("Failed to parse URL {url:?}"))?;
    let scheme = parsed_url.scheme_str().context("URL has no scheme")?;
    if scheme != "https" {
        bail!("POST requests to non-HTTPS URLs are not allowed");
    }

    let encoded_body = serde_urlencoded::to_string(form).context("Failed to encode data")?;
    let mut sender = get_http_sender(context, parsed_url.clone()).await?;
    let authority = parsed_url
        .authority()
        .context("URL has no authority")?
        .clone();
    let request = hyper::Request::post(parsed_url.path())
        .header(hyper::header::HOST, authority.as_str())
        .header("content-type", "application/x-www-form-urlencoded")
        .body(encoded_body)?;
    let response = sender.send_request(request).await?;
    let bytes = response.collect().await?.to_bytes();
    Ok(bytes)
}
