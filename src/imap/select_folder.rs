use super::Imap;

use crate::context::Context;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IMAP Could not obtain imap-session object.")]
    NoSession,

    #[error("IMAP Connection Lost or no connection established")]
    ConnectionLost,

    #[error("IMAP Folder name invalid: {0}")]
    BadFolderName(String),

    #[error("IMAP close/expunge failed")]
    CloseExpungeFailed(#[from] async_imap::error::Error),

    #[error("IMAP other error: {0}")]
    Other(String),
}

impl Imap {
    /// Issues a CLOSE command to expunge selected folder.
    ///
    /// CLOSE is considerably faster than an EXPUNGE, see
    /// https://tools.ietf.org/html/rfc3501#section-6.4.2
    async fn close_folder(&self, context: &Context) -> Result<()> {
        if let Some(ref folder) = self.config.read().await.selected_folder {
            info!(context, "Expunge messages in \"{}\".", folder);

            if let Some(ref mut session) = &mut *self.session.lock().await {
                match session.close().await {
                    Ok(_) => {
                        info!(context, "close/expunge succeeded");
                    }
                    Err(err) => {
                        self.trigger_reconnect();
                        return Err(Error::CloseExpungeFailed(err));
                    }
                }
            } else {
                return Err(Error::NoSession);
            }
        }
        let mut cfg = self.config.write().await;
        cfg.selected_folder = None;
        cfg.selected_folder_needs_expunge = false;
        Ok(())
    }

    /// select a folder, possibly update uid_validity and, if needed,
    /// expunge the folder to remove delete-marked messages.
    pub(super) async fn select_folder<S: AsRef<str>>(
        &self,
        context: &Context,
        folder: S,
    ) -> Result<()> {
        if self.session.lock().await.is_none() {
            let mut cfg = self.config.write().await;
            cfg.selected_folder = None;
            cfg.selected_folder_needs_expunge = false;
            self.trigger_reconnect();
            return Err(Error::NoSession);
        }

        let needs_expunge = self.config.read().await.selected_folder_needs_expunge;
        if needs_expunge {
            self.close_folder(context).await?;
        }

        if self.config.read().await.selected_folder.as_deref() == Some(folder.as_ref()) {
            return Ok(());
        }

        // select new folder
        if let Some(ref mut session) = &mut *self.session.lock().await {
            let res = session.select(&folder).await;

            // https://tools.ietf.org/html/rfc3501#section-6.3.1
            // says that if the server reports select failure we are in
            // authenticated (not-select) state.

            match res {
                Ok(mailbox) => {
                    let mut config = self.config.write().await;
                    config.selected_folder = Some(folder.as_ref().to_string());
                    config.selected_mailbox = Some(mailbox);
                    Ok(())
                }
                Err(async_imap::error::Error::ConnectionLost) => {
                    self.trigger_reconnect();
                    self.config.write().await.selected_folder = None;
                    Err(Error::ConnectionLost)
                }
                Err(async_imap::error::Error::Validate(_)) => {
                    Err(Error::BadFolderName(folder.as_ref().to_string()))
                }
                Err(err) => {
                    self.config.write().await.selected_folder = None;
                    self.trigger_reconnect();
                    Err(Error::Other(err.to_string()))
                }
            }
        } else {
            Err(Error::NoSession)
        }
    }
}
