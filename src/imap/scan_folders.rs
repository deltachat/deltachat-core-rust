use std::{collections::BTreeMap, time::Instant};

use anyhow::{Context as _, Result};

use crate::imap::Imap;
use crate::{config::Config, log::LogExt};
use crate::{context::Context, imap::FolderMeaning};
use async_std::prelude::*;

use super::{get_folder_meaning, get_folder_meaning_by_name};

impl Imap {
    pub(crate) async fn scan_folders(&mut self, context: &Context) -> Result<()> {
        // First of all, debounce to once per minute:
        let mut last_scan = context.last_full_folder_scan.lock().await;
        if let Some(last_scan) = *last_scan {
            let elapsed_secs = last_scan.elapsed().as_secs();
            let debounce_secs = context
                .get_config_u64(Config::ScanAllFoldersDebounceSecs)
                .await?;

            if elapsed_secs < debounce_secs {
                return Ok(());
            }
        }
        info!(context, "Starting full folder scan");

        self.prepare(context).await?;
        let session = self.session.as_mut();
        let session = session.context("scan_folders(): IMAP No Connection established")?;
        let folders: Vec<_> = session.list(Some(""), Some("*")).await?.collect().await;
        let watched_folders = get_watched_folders(context).await?;

        let mut folder_configs = BTreeMap::new();

        for folder in folders {
            let folder = match folder {
                Ok(f) => f,
                Err(e) => {
                    warn!(context, "Can't get folder: {}", e);
                    continue;
                }
            };

            // Gmail labels are not folders and should be skipped. For example,
            // emails appear in the inbox and under "All Mail" as soon as it is
            // received. The code used to wrongly conclude that the email had
            // already been moved and left it in the inbox.
            let folder_name = folder.name();
            if folder_name.starts_with("[Gmail]") {
                continue;
            }

            let folder_meaning = get_folder_meaning(&folder);
            let folder_name_meaning = get_folder_meaning_by_name(folder.name());

            if let Some(config) = folder_meaning.to_config() {
                // Always takes precedence
                folder_configs.insert(config, folder.name().to_string());
            } else if let Some(config) = folder_name_meaning.to_config() {
                // only set if none has been already set
                folder_configs
                    .entry(config)
                    .or_insert_with(|| folder.name().to_string());
            }

            let is_drafts = folder_meaning == FolderMeaning::Drafts
                || (folder_meaning == FolderMeaning::Unknown
                    && folder_name_meaning == FolderMeaning::Drafts);

            // Don't scan folders that are watched anyway
            if !watched_folders.contains(&folder.name().to_string()) && !is_drafts {
                // Drain leftover unsolicited EXISTS messages
                self.server_sent_unsolicited_exists(context);

                loop {
                    self.fetch_move_delete(context, folder.name())
                        .await
                        .ok_or_log_msg(context, "Can't fetch new msgs in scanned folder");

                    // If the server sent an unsocicited EXISTS during the fetch, we need to fetch again
                    if !self.server_sent_unsolicited_exists(context) {
                        break;
                    }
                }
            }
        }

        // We iterate over both folder meanings to make sure that if e.g. the "Sent" folder was deleted,
        // `ConfiguredSentboxFolder` is set to `None`:
        for config in &[
            Config::ConfiguredSentboxFolder,
            Config::ConfiguredSpamFolder,
        ] {
            context
                .set_config(*config, folder_configs.get(config).map(|s| s.as_str()))
                .await?;
        }

        last_scan.replace(Instant::now());
        Ok(())
    }
}

pub(crate) async fn get_watched_folders(context: &Context) -> Result<Vec<String>> {
    let mut res = Vec::new();
    if let Some(inbox_folder) = context.get_config(Config::ConfiguredInboxFolder).await? {
        res.push(inbox_folder);
    }
    let folder_watched_configured = &[
        (Config::SentboxWatch, Config::ConfiguredSentboxFolder),
        (Config::MvboxMove, Config::ConfiguredMvboxFolder),
    ];
    for (watched, configured) in folder_watched_configured {
        if context.get_config_bool(*watched).await? {
            if let Some(folder) = context.get_config(*configured).await? {
                res.push(folder);
            }
        }
    }
    Ok(res)
}
