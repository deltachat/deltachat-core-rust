use crate::context::Context;
use crate::error::{bail, Result};
use async_std::path::PathBuf;
use rand::Rng;

/// Upload file to a HTTP upload endpoint.
pub async fn upload_file(_context: &Context, url: String, filepath: PathBuf) -> Result<String> {
    // TODO: Use tokens for upload, encrypt file with PGP.
    let response = surf::put(url).body_file(filepath)?.await;
    if let Err(err) = response {
        bail!("Upload failed: {}", err);
    }
    let mut response = response.unwrap();
    match response.body_string().await {
        Ok(string) => Ok(string),
        Err(err) => bail!("Invalid response from upload: {}", err),
    }
}

/// Generate a random URL based on the provided endpoint.
pub fn generate_upload_url(_context: &Context, endpoint: String) -> String {
    const CROCKFORD_ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";
    const FILENAME_LEN: usize = 27;
    let mut rng = rand::thread_rng();
    let filename: String = (0..FILENAME_LEN)
        .map(|_| {
            let idx = rng.gen_range(0, CROCKFORD_ALPHABET.len());
            CROCKFORD_ALPHABET[idx] as char
        })
        .collect();
    format!("{}{}", endpoint, filename)
}
