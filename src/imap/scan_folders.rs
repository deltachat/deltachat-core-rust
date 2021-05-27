use std::{collections::BTreeMap, time::Instant};

use anyhow::{Context as _, Result};

use crate::context::Context;
use crate::imap::Imap;
use crate::{config::Config, log::LogExt};
use async_std::prelude::*;

use super::{get_folder_meaning, get_folder_meaning_by_name};

impl Imap {
    pub async fn scan_folders(&mut self, context: &Context) -> Result<()> {
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

        self.connect_configured(context).await?;
        let session = self.session.as_mut();
        let session = session.context("scan_folders(): IMAP No Connection established")?;
        let folders: Vec<_> = session.list(Some(""), Some("*")).await?.collect().await;
        let watched_folders = get_watched_folders(context).await;

        let mut folder_configs = BTreeMap::new();

        for folder in folders {
            let folder = match folder {
                Ok(f) => f,
                Err(e) => {
                    warn!(context, "Can't get folder: {}", e);
                    continue;
                }
            };

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

            // Don't scan folders that are watched anyway
            if !watched_folders.contains(&folder.name().to_string())
                && !context.is_drafts_folder(&folder.name().to_string()).await?
            {
                self.fetch_new_messages(context, folder.name(), false)
                    .await
                    .ok_or_log_msg(context, "Can't fetch new msgs in scanned folder");
            }
        }

        // We iterate over all 3 folder meanings to make sure that if e.g. the "Sent" folder was deleted,
        // `ConfiguredSentboxFolder` is set to `None`:
        for config in &[
            Config::ConfiguredSentboxFolder,
            Config::ConfiguredSpamFolder,
            Config::ConfiguredDraftsFolder,
        ] {
            context
                .set_config(*config, folder_configs.get(config).map(|s| s.as_str()))
                .await?;
        }

        last_scan.replace(Instant::now());
        Ok(())
    }
}

async fn get_watched_folders(context: &Context) -> Vec<String> {
    let mut res = Vec::new();
    let folder_watched_configured = &[
        (Config::SentboxWatch, Config::ConfiguredSentboxFolder),
        (Config::MvboxWatch, Config::ConfiguredMvboxFolder),
        (Config::InboxWatch, Config::ConfiguredInboxFolder),
    ];
    for (watched, configured) in folder_watched_configured {
        if context.get_config_bool(*watched).await.unwrap_or_default() {
            if let Ok(Some(folder)) = context.get_config(*configured).await {
                res.push(folder);
            }
        }
    }
    res
}
