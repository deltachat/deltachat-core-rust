//! # TLS support using Rustls.
#![cfg(feature = "tls-rustls")]

use std::sync::Arc;

use anyhow::{Context as _, Result};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::{
    client::TlsStream,
    rustls::client::ServerName,
    rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore},
    TlsConnector,
};

fn build_tls(_strict_tls: bool) -> TlsConnector {
    let mut root_store = RootCertStore::empty();
    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

pub async fn wrap_tls<T: AsyncRead + AsyncWrite + Unpin>(
    strict_tls: bool,
    hostname: &str,
    stream: T,
) -> Result<TlsStream<T>> {
    let tls = build_tls(strict_tls);
    let server_name = ServerName::try_from(hostname).context("invalid DNS name")?;
    let tls_stream = tls.connect(server_name, stream).await?;
    Ok(tls_stream)
}
