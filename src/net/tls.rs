//! TLS support.
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::net::session::SessionStream;

use tokio_rustls::rustls::client::ClientSessionStore;

pub async fn wrap_tls(
    strict_tls: bool,
    hostname: &str,
    port: u16,
    alpn: &str,
    stream: impl SessionStream + 'static,
) -> Result<impl SessionStream> {
    if strict_tls {
        let tls_stream = wrap_rustls(hostname, port, alpn, stream).await?;
        let boxed_stream: Box<dyn SessionStream> = Box::new(tls_stream);
        Ok(boxed_stream)
    } else {
        // We use native_tls because it accepts 1024-bit RSA keys.
        // Rustls does not support them even if
        // certificate checks are disabled: <https://github.com/rustls/rustls/issues/234>.
        let alpns = if alpn.is_empty() {
            Box::from([])
        } else {
            Box::from([alpn])
        };
        let tls = async_native_tls::TlsConnector::new()
            .min_protocol_version(Some(async_native_tls::Protocol::Tlsv12))
            .request_alpns(&alpns)
            .danger_accept_invalid_hostnames(true)
            .danger_accept_invalid_certs(true);
        let tls_stream = tls.connect(hostname, stream).await?;
        let boxed_stream: Box<dyn SessionStream> = Box::new(tls_stream);
        Ok(boxed_stream)
    }
}

type SessionMap = HashMap<(u16, String), Arc<dyn ClientSessionStore>>;

/// Map to store TLS session tickets.
///
/// Tickets are separated by port and ALPN
/// to avoid trying to use Postfix ticket for Dovecot and vice versa.
/// Doing so would not be a security issue,
/// but wastes the ticket and the opportunity to resume TLS session unnecessarily.
/// Rustls takes care of separating tickets that belong to different domain names.
static RESUMPTION_STORE: Lazy<Mutex<SessionMap>> = Lazy::new(Default::default);

pub async fn wrap_rustls(
    hostname: &str,
    port: u16,
    alpn: &str,
    stream: impl SessionStream,
) -> Result<impl SessionStream> {
    let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    config.alpn_protocols = if alpn.is_empty() {
        vec![]
    } else {
        vec![alpn.as_bytes().to_vec()]
    };

    // Enable TLS 1.3 session resumption
    // as defined in <https://www.rfc-editor.org/rfc/rfc8446#section-2.2>.
    //
    // Obsolete TLS 1.2 mechanisms defined in RFC 5246
    // and RFC 5077 have worse security
    // and are not worth increasing
    // attack surface: <https://words.filippo.io/we-need-to-talk-about-session-tickets/>.
    let resumption_store = Arc::clone(
        RESUMPTION_STORE
            .lock()
            .entry((port, alpn.to_string()))
            .or_insert_with(|| {
                // This is the default as of Rustls version 0.23.16,
                // but we want to create multiple caches
                // to separate them by port and ALPN.
                Arc::new(tokio_rustls::rustls::client::ClientSessionMemoryCache::new(
                    256,
                ))
            }),
    );

    let resumption = tokio_rustls::rustls::client::Resumption::store(resumption_store)
        .tls12_resumption(tokio_rustls::rustls::client::Tls12Resumption::Disabled);
    config.resumption = resumption;

    let tls = tokio_rustls::TlsConnector::from(Arc::new(config));
    let name = rustls_pki_types::ServerName::try_from(hostname)?.to_owned();
    let tls_stream = tls.connect(name, stream).await?;
    Ok(tls_stream)
}
