use crate::context::Context;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("URL request error")]
    GetError(surf::Error),
}

pub async fn read_url(context: &Context, url: &str) -> Result<String, Error> {
    info!(context, "Requesting URL {}", url);

    match surf::get(url).recv_string().await {
        Ok(res) => Ok(res),
        Err(err) => {
            info!(context, "Can\'t read URL {}: {}", url, err);

            Err(Error::GetError(err))
        }
    }
}
