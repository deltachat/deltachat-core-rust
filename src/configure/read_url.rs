use crate::context::Context;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "URL request error")]
    GetError(#[cause] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn read_url(context: &Context, url: &str) -> Result<String> {
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
