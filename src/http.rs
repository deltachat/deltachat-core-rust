//! # HTTP module.

use std::time::Duration;

use anyhow::Result;

use crate::socks::Socks5Config;

const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

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
