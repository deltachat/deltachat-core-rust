use crate::context::Context;
use anyhow::{bail, Result};
use async_std::sync::Arc;
use std::convert::TryInto;
use surf::{Client, Config};

pub async fn read_url(context: &Context, url: &str) -> Result<String> {
    info!(context, "Requesting URL {}", url);

    let client: Client = Config::new()
        .set_tls_config(Some(Arc::new(crate::login_param::dc_build_tls(true))))
        .try_into()?;

    match client.get(url).recv_string().await {
        Ok(res) => Ok(res),
        Err(err) => {
            info!(context, "Can\'t read URL {}: {}", url, err);

            bail!("URL request error");
        }
    }
}
