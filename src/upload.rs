use crate::context::Context;
use crate::error::{bail, Result};
use async_std::path::PathBuf;

/// Upload file to a HTTP upload endpoint.
pub async fn upload_file(_context: &Context, endpoint: String, file: PathBuf) -> Result<String> {
    // TODO: Use tokens for upload, encrypt file with PGP.
    let response = surf::post(endpoint).body_file(file)?.await;
    if let Err(err) = response {
        bail!("Upload failed: {}", err);
    }
    let mut response = response.unwrap();
    match response.body_string().await {
        Ok(string) => Ok(string),
        Err(err) => bail!("Invalid response from upload: {}", err),
    }
}
