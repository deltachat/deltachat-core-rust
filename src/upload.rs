use crate::context::Context;
use crate::error::{bail, Result};
use crate::pgp::{symm_decrypt, symm_encrypt_bytes};
use async_std::fs;
use async_std::path::PathBuf;
use rand::Rng;
use std::io::Cursor;
use url::Url;

/// Upload file to a HTTP upload endpoint.
pub async fn upload_file(
    context: &Context,
    url: impl AsRef<str>,
    filepath: PathBuf,
) -> Result<String> {
    let (passphrase, url) = parse_upload_url(url)?;

    let content = fs::read(filepath).await?;
    let encrypted = symm_encrypt_bytes(&passphrase, &content).await?;

    // TODO: Use tokens for upload.
    info!(context, "uploading encrypted file to {}", &url);
    let response = surf::put(url).body_bytes(encrypted).await;
    if let Err(err) = response {
        bail!("Upload failed: {}", err);
    }
    let mut response = response.unwrap();
    match response.body_string().await {
        Ok(string) => Ok(string),
        Err(err) => bail!("Invalid response from upload: {}", err),
    }
}

/// Download and decrypt a file from a HTTP endpoint.
/// TODO: Use this.
#[allow(dead_code)]
pub async fn download_file(context: &Context, url: String) -> Result<Vec<u8>> {
    let (passphrase, url) = parse_upload_url(url)?;
    info!(context, "downloading file from {}", &url);
    let response = surf::get(url).recv_bytes().await;
    if let Err(err) = response {
        bail!("Download failed: {}", err);
    }
    let reader = Cursor::new(response.unwrap());
    let decrypted = symm_decrypt(&passphrase, reader).await?;
    Ok(decrypted)
}

/// Parse a URL from a string and take out the hash fragment.
fn parse_upload_url(url: impl AsRef<str>) -> Result<(String, Url)> {
    let mut url = url::Url::parse(url.as_ref())?;
    let passphrase = url.fragment();
    if passphrase.is_none() {
        bail!("Missing passphrase for upload URL");
    }
    let passphrase = passphrase.unwrap().to_string();
    url.set_fragment(None);
    Ok((passphrase, url))
}

/// Generate a random URL based on the provided endpoint.
pub fn generate_upload_url(_context: &Context, mut endpoint: String) -> String {
    // equals at least 16 random bytes (base32 takes 160% of binary size).
    const FILENAME_LEN: usize = 26;
    // equals at least 32 random bytes.
    const PASSPHRASE_LEN: usize = 52;

    if endpoint.chars().last() == Some('/') {
        endpoint.pop();
    }
    let passphrase = generate_token_string(PASSPHRASE_LEN);
    let filename = generate_token_string(FILENAME_LEN);
    format!("{}/{}#{}", endpoint, filename, passphrase)
}

/// Generate a random string encoded in base32.
/// Len is the desired string length of the result.
/// TODO: There's likely better methods to create random tokens.
pub fn generate_token_string(len: usize) -> String {
    const CROCKFORD_ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let mut rng = rand::thread_rng();
    let token: String = (0..len)
        .map(|_| {
            let idx = rng.gen_range(0, CROCKFORD_ALPHABET.len());
            CROCKFORD_ALPHABET[idx] as char
        })
        .collect();
    token
}
