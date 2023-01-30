use anyhow::Context as _;

use super::session::Session as ImapSession;
use crate::context::Context;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IMAP Connection Lost or no connection established")]
    ConnectionLost,

    #[error("IMAP Folder name invalid: {0}")]
    BadFolderName(String),

    #[error("Got a NO response when trying to select {0}, usually this means that it doesn't exist: {1}")]
    NoFolder(String, String),

    #[error("IMAP other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Error {
        Error::Other(format!("{err:#}"))
    }
}

impl ImapSession {
    /// Issues a CLOSE command if selected folder needs expunge,
    /// i.e. if Delta Chat marked a message there as deleted previously.
    ///
    /// CLOSE is considerably faster than an EXPUNGE, see
    /// <https://tools.ietf.org/html/rfc3501#section-6.4.2>
    pub(super) async fn maybe_close_folder(&mut self, context: &Context) -> anyhow::Result<()> {
        if let Some(folder) = &self.selected_folder {
            if self.selected_folder_needs_expunge {
                info!(context, "Expunge messages in \"{}\".", folder);

                self.close().await.context("IMAP close/expunge failed")?;
                info!(context, "close/expunge succeeded");
                self.selected_folder = None;
                self.selected_folder_needs_expunge = false;
            }
        }
        Ok(())
    }

    /// Selects a folder, possibly updating uid_validity and, if needed,
    /// expunging the folder to remove delete-marked messages.
    /// Returns whether a new folder was selected.
    pub(super) async fn select_folder(
        &mut self,
        context: &Context,
        folder: Option<&str>,
    ) -> Result<NewlySelected> {
        // if there is a new folder and the new folder is equal to the selected one, there's nothing to do.
        // if there is _no_ new folder, we continue as we might want to expunge below.
        if let Some(folder) = folder {
            if let Some(selected_folder) = &self.selected_folder {
                if folder == selected_folder {
                    return Ok(NewlySelected::No);
                }
            }
        }

        // deselect existing folder, if needed (it's also done implicitly by SELECT, however, without EXPUNGE then)
        self.maybe_close_folder(context).await?;

        // select new folder
        if let Some(folder) = folder {
            let res = if self.can_condstore() {
                self.select_condstore(folder).await
            } else {
                self.select(folder).await
            };

            // <https://tools.ietf.org/html/rfc3501#section-6.3.1>
            // says that if the server reports select failure we are in
            // authenticated (not-select) state.

            match res {
                Ok(mailbox) => {
                    self.selected_folder = Some(folder.to_string());
                    self.selected_mailbox = Some(mailbox);
                    Ok(NewlySelected::Yes)
                }
                Err(async_imap::error::Error::ConnectionLost) => Err(Error::ConnectionLost),
                Err(async_imap::error::Error::Validate(_)) => {
                    Err(Error::BadFolderName(folder.to_string()))
                }
                Err(async_imap::error::Error::No(response)) => {
                    Err(Error::NoFolder(folder.to_string(), response))
                }
                Err(err) => Err(Error::Other(err.to_string())),
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
    ) -> anyhow::Result<NewlySelected> {
        match self.select_folder(context, Some(folder)).await {
            Ok(newly_selected) => Ok(newly_selected),
            Err(err) => match err {
                Error::NoFolder(..) => {
                    info!(context, "Failed to select folder {} because it does not exist, trying to create it.", folder);
                    self.create(folder).await.with_context(|| {
                        format!("Couldn't select folder ('{err}'), then create() failed")
                    })?;

                    Ok(self.select_folder(context, Some(folder)).await.with_context(|| format!("failed to select newely created folder {folder}"))?)
                }
                _ => Err(err).with_context(|| format!("failed to select folder {folder} with error other than NO, not trying to create it")),
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
