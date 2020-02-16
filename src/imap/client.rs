use async_imap::{
    error::{Error as ImapError, Result as ImapResult},
    Client as ImapClient,
};
use async_native_tls::TlsStream;
use async_std::net::{self, TcpStream};

use super::session::Session;
use crate::login_param::{dc_build_tls, CertificateChecks};

#[derive(Debug)]
pub(crate) enum Client {
    Secure(ImapClient<TlsStream<TcpStream>>),
    Insecure(ImapClient<TcpStream>),
}

impl Client {
    pub async fn connect_secure<A: net::ToSocketAddrs, S: AsRef<str>>(
        addr: A,
        domain: S,
        certificate_checks: CertificateChecks,
    ) -> ImapResult<Self> {
        let stream = TcpStream::connect(addr).await?;
        let tls = dc_build_tls(certificate_checks);
        let tls_stream = tls.connect(domain.as_ref(), stream).await?;
        let mut client = ImapClient::new(tls_stream);
        if std::env::var(crate::DCC_IMAP_DEBUG).is_ok() {
            client.debug = true;
        }

        let _greeting = client
            .read_response()
            .await
            .ok_or_else(|| ImapError::Bad("failed to read greeting".to_string()))?;

        Ok(Client::Secure(client))
    }

    pub async fn connect_insecure<A: net::ToSocketAddrs>(addr: A) -> ImapResult<Self> {
        let stream = TcpStream::connect(addr).await?;

        let mut client = ImapClient::new(stream);
        if std::env::var(crate::DCC_IMAP_DEBUG).is_ok() {
            client.debug = true;
        }
        let _greeting = client
            .read_response()
            .await
            .ok_or_else(|| ImapError::Bad("failed to read greeting".to_string()))?;

        Ok(Client::Insecure(client))
    }

    pub async fn secure<S: AsRef<str>>(
        self,
        domain: S,
        certificate_checks: CertificateChecks,
    ) -> ImapResult<Client> {
        match self {
            Client::Insecure(client) => {
                let tls = dc_build_tls(certificate_checks);
                let client_sec = client.secure(domain, tls).await?;

                Ok(Client::Secure(client_sec))
            }
            // Nothing to do
            Client::Secure(_) => Ok(self),
        }
    }

    pub async fn authenticate<A: async_imap::Authenticator, S: AsRef<str>>(
        self,
        auth_type: S,
        authenticator: &A,
    ) -> Result<Session, (ImapError, Client)> {
        match self {
            Client::Secure(i) => match i.authenticate(auth_type, authenticator).await {
                Ok(session) => Ok(Session::Secure(session)),
                Err((err, c)) => Err((err, Client::Secure(c))),
            },
            Client::Insecure(i) => match i.authenticate(auth_type, authenticator).await {
                Ok(session) => Ok(Session::Insecure(session)),
                Err((err, c)) => Err((err, Client::Insecure(c))),
            },
        }
    }

    pub async fn login<U: AsRef<str>, P: AsRef<str>>(
        self,
        username: U,
        password: P,
    ) -> Result<Session, (ImapError, Client)> {
        match self {
            Client::Secure(i) => match i.login(username, password).await {
                Ok(session) => Ok(Session::Secure(session)),
                Err((err, c)) => Err((err, Client::Secure(c))),
            },
            Client::Insecure(i) => match i.login(username, password).await {
                Ok(session) => Ok(Session::Insecure(session)),
                Err((err, c)) => Err((err, Client::Insecure(c))),
            },
        }
    }
}
