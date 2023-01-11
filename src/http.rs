//! # HTTP module.

use anyhow::Result;
use std::time::Duration;

const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn get_client() -> Result<reqwest::Client> {
    Ok(reqwest::ClientBuilder::new()
        .timeout(HTTP_TIMEOUT)
        .build()?)
}
