//! # HTTP module.

use std::time::Duration;

use anyhow::Result;

const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn get_client() -> Result<reqwest::Client> {
    Ok(reqwest::ClientBuilder::new()
        .timeout(HTTP_TIMEOUT)
        .build()?)
}
