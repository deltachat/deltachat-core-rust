use std::time::{Instant, SystemTime};

use crate::{config::Config, context::Context};
use anyhow::Context as _;

use crate::error::Result;
use crate::imap::Imap;
use async_std::prelude::*;

use super::{get_folder_meaning, get_folder_meaning_by_name, FolderMeaning};

impl Imap {
    pub async fn scan_folders(&mut self, context: &Context) -> Result<()> {
        use crate::config::Config::*;

        // First of all, debounce to once per minute:
        let mut last_scan = context.last_full_folder_scan.lock().await;
        if let Some(last_scan) = *last_scan {
            if last_scan.elapsed().as_secs() < 60 {
                // For the first day after installation, we only debounce to 2s:
                let configure = context.get_config(Config::ConfiguredTimestamp).await;
                let configure = configure.context("scan_folders: not configured")?;
                let configure: SystemTime = serde_json::from_str(&configure)?;

                if configure.elapsed().unwrap().as_secs() > 24 * 60 * 60
                    || last_scan.elapsed().as_secs() < 2
                // For the first day after installation, only debounce to 2s
                {
                    info!(context, "Not scanning, we recently already scanned");
                    return Ok(());
                }
            }
        }
        info!(context, "Starting full folder scan");
        last_scan.replace(Instant::now());

        self.setup_handle(context).await?;
        let session = self.session.as_mut();
        let session = session.context("scan_folders(): IMAP No Connection established")?;
        let folders: Vec<_> = session.list(Some(""), Some("*")).await?.collect().await;

        for folder in folders {
            // TODO Maybe exclude folders that are watched anyway
            let folder = folder?;
            let foldername = folder.name();
            info!(context, "Scanning folder: {}", foldername);

            let folder_meaning = get_folder_meaning(&folder);
            let folder_name_meaning = get_folder_meaning_by_name(&foldername);
            // If there are two folders with the \Sent or \Spam flag, then the sent/spam folder will change all the time.
            // This should not be a problem though, worst thing that can happen is that messages are moved to different folders.
            if folder_meaning == FolderMeaning::SentObjects {
                context
                    .set_config(ConfiguredSentboxFolder, Some(folder.name()))
                    .await?;
            } else if folder_meaning == FolderMeaning::Spam {
                context
                    .set_config(ConfiguredSpamFolder, Some(folder.name()))
                    .await?;
            } else if folder_name_meaning == FolderMeaning::SentObjects {
                // only set iff none has been already set
                if context.get_config(ConfiguredSentboxFolder).await.is_none() {
                    context
                        .set_config(ConfiguredSentboxFolder, Some(folder.name()))
                        .await?;
                }
            } else if folder_name_meaning == FolderMeaning::Spam
                && context.get_config(ConfiguredSpamFolder).await.is_none()
            {
                context
                    .set_config(ConfiguredSpamFolder, Some(folder.name()))
                    .await?;
            }

            if let Err(e) = self.fetch_new_messages(context, foldername, false).await {
                warn!(context, "Can't fetch new msgs in scanned folder: {:#}", e);
            }
        }
        Ok(())
    }
}
