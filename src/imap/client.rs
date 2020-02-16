use async_imap::{
    error::{Error as ImapError, Result as ImapResult},
    extensions::idle::Handle as ImapIdleHandle,
    types::{Capabilities, Fetch, Mailbox, Name},
    Client as ImapClient, Session as ImapSession,
};
use async_native_tls::TlsStream;
use async_std::net::{self, TcpStream};
use async_std::prelude::*;

use crate::login_param::{dc_build_tls, CertificateChecks};

#[derive(Debug)]
pub(crate) enum Client {
    Secure(ImapClient<TlsStream<TcpStream>>),
    Insecure(ImapClient<TcpStream>),
}

#[derive(Debug)]
pub(crate) enum Session {
    Secure(ImapSession<TlsStream<TcpStream>>),
    Insecure(ImapSession<TcpStream>),
}

#[derive(Debug)]
pub(crate) enum IdleHandle {
    Secure(ImapIdleHandle<TlsStream<TcpStream>>),
    Insecure(ImapIdleHandle<TcpStream>),
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

impl Session {
    pub async fn capabilities(&mut self) -> ImapResult<Capabilities> {
        let res = match self {
            Session::Secure(i) => i.capabilities().await?,
            Session::Insecure(i) => i.capabilities().await?,
        };

        Ok(res)
    }

    pub async fn list(
        &mut self,
        reference_name: Option<&str>,
        mailbox_pattern: Option<&str>,
    ) -> ImapResult<Vec<Name>> {
        let res = match self {
            Session::Secure(i) => {
                i.list(reference_name, mailbox_pattern)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
            Session::Insecure(i) => {
                i.list(reference_name, mailbox_pattern)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
        };
        Ok(res)
    }

    pub async fn create<S: AsRef<str>>(&mut self, mailbox_name: S) -> ImapResult<()> {
        match self {
            Session::Secure(i) => i.create(mailbox_name).await?,
            Session::Insecure(i) => i.create(mailbox_name).await?,
        }
        Ok(())
    }

    pub async fn subscribe<S: AsRef<str>>(&mut self, mailbox: S) -> ImapResult<()> {
        match self {
            Session::Secure(i) => i.subscribe(mailbox).await?,
            Session::Insecure(i) => i.subscribe(mailbox).await?,
        }
        Ok(())
    }

    pub async fn close(&mut self) -> ImapResult<()> {
        match self {
            Session::Secure(i) => i.close().await?,
            Session::Insecure(i) => i.close().await?,
        }
        Ok(())
    }

    pub async fn select<S: AsRef<str>>(&mut self, mailbox_name: S) -> ImapResult<Mailbox> {
        let mbox = match self {
            Session::Secure(i) => i.select(mailbox_name).await?,
            Session::Insecure(i) => i.select(mailbox_name).await?,
        };

        Ok(mbox)
    }

    pub async fn fetch<S1, S2>(&mut self, sequence_set: S1, query: S2) -> ImapResult<Vec<Fetch>>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        let res = match self {
            Session::Secure(i) => {
                i.fetch(sequence_set, query)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
            Session::Insecure(i) => {
                i.fetch(sequence_set, query)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
        };
        Ok(res)
    }

    pub async fn uid_fetch<S1, S2>(&mut self, uid_set: S1, query: S2) -> ImapResult<Vec<Fetch>>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        let res = match self {
            Session::Secure(i) => {
                i.uid_fetch(uid_set, query)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
            Session::Insecure(i) => {
                i.uid_fetch(uid_set, query)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
        };

        Ok(res)
    }

    pub fn idle(self) -> IdleHandle {
        match self {
            Session::Secure(i) => {
                let h = i.idle();
                IdleHandle::Secure(h)
            }
            Session::Insecure(i) => {
                let h = i.idle();
                IdleHandle::Insecure(h)
            }
        }
    }

    pub async fn uid_store<S1, S2>(&mut self, uid_set: S1, query: S2) -> ImapResult<Vec<Fetch>>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        let res = match self {
            Session::Secure(i) => {
                i.uid_store(uid_set, query)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
            Session::Insecure(i) => {
                i.uid_store(uid_set, query)
                    .await?
                    .collect::<ImapResult<_>>()
                    .await?
            }
        };
        Ok(res)
    }

    pub async fn uid_mv<S1: AsRef<str>, S2: AsRef<str>>(
        &mut self,
        uid_set: S1,
        mailbox_name: S2,
    ) -> ImapResult<()> {
        match self {
            Session::Secure(i) => i.uid_mv(uid_set, mailbox_name).await?,
            Session::Insecure(i) => i.uid_mv(uid_set, mailbox_name).await?,
        }
        Ok(())
    }

    pub async fn uid_copy<S1: AsRef<str>, S2: AsRef<str>>(
        &mut self,
        uid_set: S1,
        mailbox_name: S2,
    ) -> ImapResult<()> {
        match self {
            Session::Secure(i) => i.uid_copy(uid_set, mailbox_name).await?,
            Session::Insecure(i) => i.uid_copy(uid_set, mailbox_name).await?,
        }

        Ok(())
    }
}
