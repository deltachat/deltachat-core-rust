use crate::context::Context;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("URL request error")]
    GetError(#[from] reqwest::Error),
}

pub fn read_url(context: &Context, url: &str) -> Result<String, Error> {
    info!(context, "Requesting URL {}", url);

    match reqwest::blocking::Client::new()
        .get(url)
        .send()
        .and_then(|res| res.text())
    {
        Ok(res) => Ok(res),
        Err(err) => {
            info!(context, "Can\'t read URL {}", url);

            Err(Error::GetError(err))
        }
    }
}
