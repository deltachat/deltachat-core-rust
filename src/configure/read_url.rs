use crate::context::Context;

use anyhow::format_err;
use anyhow::Context as _;

pub async fn read_url(context: &Context, url: &str) -> anyhow::Result<String> {
    match read_url_inner(context, url).await {
        Ok(s) => {
            info!(context, "Successfully read url {}", url);
            Ok(s)
        }
        Err(e) => {
            info!(context, "Can't read URL {}: {:#}", url, e);
            Err(format_err!("Can't read URL {}: {:#}", url, e))
        }
    }
}

pub async fn read_url_inner(context: &Context, mut url: &str) -> anyhow::Result<String> {
    let mut _temp; // For the borrow checker

    // Follow up to 10 http-redirects
    for _i in 0..10 {
        let mut response = surf::get(url).send().await.map_err(|e| e.into_inner())?;
        if response.status().is_redirection() {
            _temp = response
                .header("location")
                .context("Redirection doesn't have a target location")?
                .last()
                .to_string();
            info!(context, "Following redirect to {}", _temp);
            url = &_temp;
            continue;
        }

        return response.body_string().await.map_err(|e| e.into_inner());
    }

    Err(format_err!("Followed 10 redirections"))
}
