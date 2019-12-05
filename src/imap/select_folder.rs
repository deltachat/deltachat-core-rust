use super::Imap;

use crate::context::Context;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "IMAP Could not obtain imap-session object.")]
    NoSession,

    #[fail(display = "IMAP Connection Lost or no connection established")]
    ConnectionLost,

    #[fail(display = "IMAP Folder name invalid: {:?}", _0)]
    BadFolderName(String),

    #[fail(display = "IMAP close/expunge failed: {}", _0)]
    CloseExpungeFailed(#[cause] async_imap::error::Error),

    #[fail(display = "IMAP other error: {:?}", _0)]
    Other(String),
}

impl Imap {
    /// select a folder, possibly update uid_validity and, if needed,
    /// expunge the folder to remove delete-marked messages.
    pub(super) async fn select_folder<S: AsRef<str>>(
        &self,
        context: &Context,
        folder: Option<S>,
    ) -> Result<()> {
        if self.session.lock().await.is_none() {
            let mut cfg = self.config.write().await;
            cfg.selected_folder = None;
            cfg.selected_folder_needs_expunge = false;
            self.trigger_reconnect();
            return Err(Error::NoSession);
        }

        // if there is a new folder and the new folder is equal to the selected one, there's nothing to do.
        // if there is _no_ new folder, we continue as we might want to expunge below.
        if let Some(ref folder) = folder {
            if let Some(ref selected_folder) = self.config.read().await.selected_folder {
                if folder.as_ref() == selected_folder {
                    return Ok(());
                }
            }
        }

        // deselect existing folder, if needed (it's also done implicitly by SELECT, however, without EXPUNGE then)
        let needs_expunge = { self.config.read().await.selected_folder_needs_expunge };
        if needs_expunge {
            if let Some(ref folder) = self.config.read().await.selected_folder {
                info!(context, "Expunge messages in \"{}\".", folder);

                // A CLOSE-SELECT is considerably faster than an EXPUNGE-SELECT, see
                // https://tools.ietf.org/html/rfc3501#section-6.4.2
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
            self.config.write().await.selected_folder_needs_expunge = false;
        }

        // select new folder
        if let Some(ref folder) = folder {
            if let Some(ref mut session) = &mut *self.session.lock().await {
                let res = session.select(folder).await;

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
        } else {
            Ok(())
        }
    }
}
