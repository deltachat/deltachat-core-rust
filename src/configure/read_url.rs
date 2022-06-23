use anyhow::{anyhow, format_err};

use crate::context::Context;

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

pub async fn read_url_inner(context: &Context, url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
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

        return response.text().await.map_err(Into::into);
    }

    Err(format_err!("Followed 10 redirections"))
}
