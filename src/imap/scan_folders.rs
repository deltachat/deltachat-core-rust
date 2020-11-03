use std::time::Instant;

use crate::context::Context;
use anyhow::Context as _;

use crate::error::Result;
use crate::imap::Imap;
use async_std::prelude::*;

impl Imap {
    pub async fn scan_folders(&mut self, context: &Context) -> Result<()> {
        let mut last_scan = context.last_full_folder_scan.lock().await;
        if let Some(time) = *last_scan {
            if time.elapsed().as_secs() < 60 {
                return Ok(());
            }
        }
        last_scan.replace(Instant::now());

        self.setup_handle(context).await?;
        let session = self.session.as_mut();
        let session = session.context("scan_folders(): IMAP No Connection established")?;

        let folders: Vec<_> = session.list(Some(""), Some("*")).await?.collect().await;

        for folder in folders {
            // TODO Maybe exclude folders that are watched anyway
            let folder = folder?.name();
            let mailbox = session.select(folder).await?;
            let last_uidnext: u32 = context
                .sql
                .query_get_value_result(
                    "SELECT last_uidnext FROM imap_sync WHERE folder=?",
                    paramsv![folder],
                )
                .await?
                .unwrap_or_default();
            if mailbox.uid_next.unwrap() != last_uidnext { //TODO rm unwrap
                 //t
            }
        }
        Ok(())
    }
}
