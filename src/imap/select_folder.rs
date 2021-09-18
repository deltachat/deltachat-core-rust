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

    #[error("IMAP folder does not exist: {0}")]
    NoFolder(String),

    #[error("IMAP close/expunge failed")]
    CloseExpungeFailed(#[from] async_imap::error::Error),

    #[error("IMAP other error: {0}")]
    Other(String),
}

impl Imap {
    /// Issues a CLOSE command to expunge selected folder.
    ///
    /// CLOSE is considerably faster than an EXPUNGE, see
    /// <https://tools.ietf.org/html/rfc3501#section-6.4.2>
    pub(super) async fn close_folder(&mut self, context: &Context) -> Result<()> {
        if let Some(ref folder) = self.config.selected_folder {
            info!(context, "Expunge messages in \"{}\".", folder);

            if let Some(ref mut session) = self.session {
                match session.close().await {
                    Ok(_) => {
                        info!(context, "close/expunge succeeded");
                    }
                    Err(err) => {
                        self.trigger_reconnect(context).await;
                        return Err(Error::CloseExpungeFailed(err));
                    }
                }
            } else {
                return Err(Error::NoSession);
            }
        }
        self.config.selected_folder = None;
        self.config.selected_folder_needs_expunge = false;

        Ok(())
    }

    /// Issues a CLOSE command if selected folder needs expunge.
    pub(crate) async fn maybe_close_folder(&mut self, context: &Context) -> Result<()> {
        if self.config.selected_folder_needs_expunge {
            self.close_folder(context).await?;
        }
        Ok(())
    }

    /// select a folder, possibly update uid_validity and, if needed,
    /// expunge the folder to remove delete-marked messages.
    /// Returns whether a new folder was selected.
    pub(super) async fn select_folder(
        &mut self,
        context: &Context,
        folder: Option<&str>,
    ) -> Result<NewlySelected> {
        if self.session.is_none() {
            self.config.selected_folder = None;
            self.config.selected_folder_needs_expunge = false;
            self.trigger_reconnect(context).await;
            return Err(Error::NoSession);
        }

        // if there is a new folder and the new folder is equal to the selected one, there's nothing to do.
        // if there is _no_ new folder, we continue as we might want to expunge below.
        if let Some(folder) = folder {
            if let Some(ref selected_folder) = self.config.selected_folder {
                if folder == selected_folder {
                    return Ok(NewlySelected::No);
                }
            }
        }

        // deselect existing folder, if needed (it's also done implicitly by SELECT, however, without EXPUNGE then)
        self.maybe_close_folder(context).await?;

        // select new folder
        if let Some(folder) = folder {
            if let Some(ref mut session) = &mut self.session {
                let res = session.select(folder).await;

                // <https://tools.ietf.org/html/rfc3501#section-6.3.1>
                // says that if the server reports select failure we are in
                // authenticated (not-select) state.

                match res {
                    Ok(mailbox) => {
                        self.config.selected_folder = Some(folder.to_string());
                        self.config.selected_mailbox = Some(mailbox);
                        Ok(NewlySelected::Yes)
                    }
                    Err(async_imap::error::Error::ConnectionLost) => {
                        self.trigger_reconnect(context).await;
                        self.config.selected_folder = None;
                        Err(Error::ConnectionLost)
                    }
                    Err(async_imap::error::Error::Validate(_)) => {
                        Err(Error::BadFolderName(folder.to_string()))
                    }
                    Err(async_imap::error::Error::No(_)) => {
                        Err(Error::NoFolder(folder.to_string()))
                    }
                    Err(err) => {
                        self.config.selected_folder = None;
                        self.trigger_reconnect(context).await;
                        Err(Error::Other(err.to_string()))
                    }
                }
            } else {
                Err(Error::NoSession)
            }
        } else {
            Ok(NewlySelected::No)
        }
    }

    /// Selects a folder. Tries to create it once and select again if the folder does not exist.
    pub(super) async fn select_or_create_folder(
        &mut self,
        context: &Context,
        folder: &str,
    ) -> Result<NewlySelected> {
        match self.select_folder(context, Some(folder)).await {
            Ok(newly_selected) => Ok(newly_selected),
            Err(err) => match err {
                Error::NoFolder(_) => {
                    if let Some(ref mut session) = self.session {
                        session.create(folder).await?;
                    } else {
                        return Err(Error::NoSession);
                    }
                    self.select_folder(context, Some(folder)).await
                }
                _ => Err(err),
            },
        }
    }
}
#[derive(PartialEq, Debug, Copy, Clone, Eq)]
pub(super) enum NewlySelected {
    /// The folder was newly selected during this call to select_folder().
    Yes,
    /// No SELECT command was run because the folder already was selected
    /// and self.config.selected_mailbox was not updated (so, e.g. it may contain an outdated uid_next)
    No,
}
