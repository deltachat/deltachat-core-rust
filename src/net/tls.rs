//! TLS support.
use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::Lazy;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::net::session::SessionStream;

// this certificate is missing on older android devices (eg. lg with android6 from 2017)
// certificate downloaded from https://letsencrypt.org/certificates/
static LETSENCRYPT_ROOT: Lazy<async_native_tls::Certificate> = Lazy::new(|| {
    async_native_tls::Certificate::from_der(include_bytes!(
        "../../assets/root-certificates/letsencrypt/isrgrootx1.der"
    ))
    .unwrap()
});

pub async fn wrap_tls(
    strict_tls: bool,
    hostname: &str,
    alpn: &[&str],
    stream: impl SessionStream,
) -> Result<impl SessionStream> {
    let tls_builder = async_native_tls::TlsConnector::new()
        .min_protocol_version(Some(async_native_tls::Protocol::Tlsv12))
        .request_alpns(alpn)
        .add_root_certificate(LETSENCRYPT_ROOT.clone());
    let tls = if strict_tls {
        tls_builder
    } else {
        tls_builder
            .danger_accept_invalid_hostnames(true)
            .danger_accept_invalid_certs(true)
    };
    let tls_stream = tls.connect(hostname, stream).await?;
    Ok(tls_stream)
}

pub async fn wrap_rustls<T: AsyncRead + AsyncWrite + Unpin>(
    hostname: &str,
    alpn: &[&str],
    stream: T,
) -> Result<tokio_rustls::client::TlsStream<T>> {
    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    config.alpn_protocols = alpn.iter().map(|s| s.as_bytes().to_vec()).collect();

    let tls = tokio_rustls::TlsConnector::from(Arc::new(config));
    let name = rustls_pki_types::ServerName::try_from(hostname)?.to_owned();
    let tls_stream = tls.connect(name, stream).await?;
    Ok(tls_stream)
}
