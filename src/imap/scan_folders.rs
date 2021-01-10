use std::time::Instant;

use crate::{config::Config, context::Context};
use anyhow::Context as _;

use crate::error::Result;
use crate::imap::Imap;
use async_std::prelude::*;

use super::{get_folder_meaning, get_folder_meaning_by_name, FolderMeaning};

impl Imap {
    pub async fn scan_folders(&mut self, context: &Context) -> Result<()> {
        // First of all, debounce to once per minute:
        let mut last_scan = context.last_full_folder_scan.lock().await;
        if let Some(last_scan) = *last_scan {
            let elapsed_secs = last_scan.elapsed().as_secs();
            let debounce_secs = context
                .get_config_u64(Config::ScanAllFoldersDebounceSecs)
                .await;

            if elapsed_secs < debounce_secs {
                info!(context, "Not scanning, we scanned {}s ago", elapsed_secs);
                return Ok(());
            }
        }
        info!(context, "Starting full folder scan");

        self.setup_handle(context).await?;
        let session = self.session.as_mut();
        let session = session.context("scan_folders(): IMAP No Connection established")?;
        let folders: Vec<_> = session.list(Some(""), Some("*")).await?.collect().await;

        let mut sentbox_folder = None;
        let mut spam_folder = None;

        for folder in folders {
            let folder = match folder {
                Ok(f) => f,
                Err(e) => {
                    warn!(context, "Can't get folder: {}", e);
                    continue;
                }
            };
            let foldername = folder.name();
            info!(context, "Scanning folder: {}", foldername);

            let folder_meaning = get_folder_meaning(&folder);
            let folder_name_meaning = get_folder_meaning_by_name(&foldername);

            if folder_meaning == FolderMeaning::SentObjects {
                // Always takes precedent
                sentbox_folder = Some(folder.name().to_string());
            } else if folder_meaning == FolderMeaning::Spam {
                spam_folder = Some(folder.name().to_string());
            } else if folder_name_meaning == FolderMeaning::SentObjects {
                // only set iff none has been already set
                if sentbox_folder.is_none() {
                    sentbox_folder = Some(folder.name().to_string());
                }
            } else if folder_name_meaning == FolderMeaning::Spam && spam_folder.is_none() {
                spam_folder = Some(folder.name().to_string());
            }

            if let Err(e) = self.fetch_new_messages(context, foldername, false).await {
                warn!(context, "Can't fetch new msgs in scanned folder: {:#}", e);
            }
        }

        context
            .set_config(Config::ConfiguredSentboxFolder, sentbox_folder.as_deref())
            .await?;
        context
            .set_config(Config::ConfiguredSpamFolder, spam_folder.as_deref())
            .await?;

        last_scan.replace(Instant::now());
        Ok(())
    }
}
