use std::{collections::BTreeMap, time::Instant};

use anyhow::{Context as _, Result};
use futures::stream::StreamExt;

use super::{get_folder_meaning_by_attrs, get_folder_meaning_by_name};
use crate::config::Config;
use crate::imap::Imap;
use crate::log::LogExt;
use crate::{context::Context, imap::FolderMeaning};

impl Imap {
    /// Returns true if folders were scanned, false if scanning was postponed.
    pub(crate) async fn scan_folders(&mut self, context: &Context) -> Result<bool> {
        // First of all, debounce to once per minute:
        let mut last_scan = context.last_full_folder_scan.lock().await;
        if let Some(last_scan) = *last_scan {
            let elapsed_secs = last_scan.elapsed().as_secs();
            let debounce_secs = context
                .get_config_u64(Config::ScanAllFoldersDebounceSecs)
                .await?;

            if elapsed_secs < debounce_secs {
                return Ok(false);
            }
        }
        info!(context, "Starting full folder scan");

        self.prepare(context).await?;
        let folders = self.list_folders(context).await?;
        let watched_folders = get_watched_folders(context).await?;

        let mut folder_configs = BTreeMap::new();

        for folder in folders {
            let folder_meaning = get_folder_meaning_by_attrs(folder.attributes());
            if folder_meaning == FolderMeaning::Virtual {
                // Gmail has virtual folders that should be skipped. For example,
                // emails appear in the inbox and under "All Mail" as soon as it is
                // received. The code used to wrongly conclude that the email had
                // already been moved and left it in the inbox.
                continue;
            }
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

            let folder_meaning = match folder_meaning {
                FolderMeaning::Unknown => folder_name_meaning,
                _ => folder_meaning,
            };

            // Don't scan folders that are watched anyway
            if !watched_folders.contains(&folder.name().to_string())
                && folder_meaning != FolderMeaning::Drafts
                && folder_meaning != FolderMeaning::Trash
            {
                let session = self.session.as_mut().context("no session")?;
                // Drain leftover unsolicited EXISTS messages
                session.server_sent_unsolicited_exists(context)?;

                loop {
                    self.fetch_move_delete(context, folder.name(), folder_meaning)
                        .await
                        .context("Can't fetch new msgs in scanned folder")
                        .log_err(context)
                        .ok();

                    let session = self.session.as_mut().context("no session")?;
                    // If the server sent an unsocicited EXISTS during the fetch, we need to fetch again
                    if !session.server_sent_unsolicited_exists(context)? {
                        break;
                    }
                }
            }
        }

        // Set configs for necessary folders. Or reset if the folder was deleted.
        for conf in [
            Config::ConfiguredSentboxFolder,
            Config::ConfiguredTrashFolder,
        ] {
            context
                .set_config(conf, folder_configs.get(&conf).map(|s| s.as_str()))
                .await?;
        }

        last_scan.replace(Instant::now());
        Ok(true)
    }

    /// Returns the names of all folders on the IMAP server.
    pub async fn list_folders(
        self: &mut Imap,
        context: &Context,
    ) -> Result<Vec<async_imap::types::Name>> {
        let session = self.session.as_mut();
        let session = session.context("No IMAP connection")?;
        let list = session
            .list(Some(""), Some("*"))
            .await?
            .filter_map(|f| async {
                f.context("list_folders() can't get folder")
                    .log_err(context)
                    .ok()
            });
        Ok(list.collect().await)
    }
}

pub(crate) async fn get_watched_folder_configs(context: &Context) -> Result<Vec<Config>> {
    let mut res = vec![Config::ConfiguredInboxFolder];
    if context.get_config_bool(Config::SentboxWatch).await? {
        res.push(Config::ConfiguredSentboxFolder);
    }
    if context.should_watch_mvbox().await? {
        res.push(Config::ConfiguredMvboxFolder);
    }
    Ok(res)
}

pub(crate) async fn get_watched_folders(context: &Context) -> Result<Vec<String>> {
    let mut res = Vec::new();
    for folder_config in get_watched_folder_configs(context).await? {
        if let Some(folder) = context.get_config(folder_config).await? {
            res.push(folder);
        }
    }
    Ok(res)
}
