//! TLS support.

use anyhow::Result;
use async_native_tls::{Certificate, Protocol, TlsConnector, TlsStream};
use once_cell::sync::Lazy;
use tokio::io::{AsyncRead, AsyncWrite};

// this certificate is missing on older android devices (eg. lg with android6 from 2017)
// certificate downloaded from https://letsencrypt.org/certificates/
static LETSENCRYPT_ROOT: Lazy<Certificate> = Lazy::new(|| {
    Certificate::from_der(include_bytes!(
        "../../assets/root-certificates/letsencrypt/isrgrootx1.der"
    ))
    .unwrap()
});

static IMAP_NAUTA_CU: Lazy<Certificate> = Lazy::new(|| {
    Certificate::from_der(include_bytes!(
        "../../assets/certificates/imap.nauta.cu.der"
    ))
    .unwrap()
});

static SMTP_NAUTA_CU: Lazy<Certificate> = Lazy::new(|| {
    Certificate::from_der(include_bytes!(
        "../../assets/certificates/smtp.nauta.cu.der"
    ))
    .unwrap()
});

fn build_tls(strict_tls: bool, hostname: &str, alpns: &[&str]) -> TlsConnector {
    let tls_builder = TlsConnector::new()
        .min_protocol_version(Some(Protocol::Tlsv12))
        .request_alpns(alpns)
        .add_root_certificate(LETSENCRYPT_ROOT.clone());

    // Add self-signed certificates for known hostnames.
    let tls_builder = match hostname {
        "imap.nauta.cu" => tls_builder.add_root_certificate(IMAP_NAUTA_CU.clone()),
        "smtp.nauta.cu" => tls_builder.add_root_certificate(SMTP_NAUTA_CU.clone()),
        _ => tls_builder,
    };

    if strict_tls {
        tls_builder
    } else {
        tls_builder
            .danger_accept_invalid_hostnames(true)
            .danger_accept_invalid_certs(true)
    }
}

pub async fn wrap_tls<T: AsyncRead + AsyncWrite + Unpin>(
    strict_tls: bool,
    hostname: &str,
    alpn: &[&str],
    stream: T,
) -> Result<TlsStream<T>> {
    let tls = build_tls(strict_tls, hostname, alpn);
    let tls_stream = tls.connect(hostname, stream).await?;
    Ok(tls_stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tls() {
        // we are using some additional root certificates.
        // make sure, they do not break construction of TlsConnector
        let _ = build_tls(true, "example.org", &[]);
        let _ = build_tls(false, "example.org", &[]);
    }
}
