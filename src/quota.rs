//! # Support for IMAP QUOTA extension.

use anyhow::{anyhow, Result};
use async_imap::types::{Quota, QuotaResource};
use indexmap::IndexMap;

use crate::context::Context;
use crate::dc_tools::time;
use crate::imap::scan_folders::get_watched_folders;
use crate::imap::Imap;
use crate::job::{Action, Status};
use crate::param::Params;
use crate::{job, EventType};

/// warn about a nearly full mailbox after this usage percentage is reached.
/// quota icon is "yellow".
pub const QUOTA_WARN_THRESHOLD_PERCENTAGE: u64 = 80;

// warning is already issued at QUOTA_WARN_THRESHOLD_PERCENTAGE,
// this threshold only makes the quota icon "red".
pub const QUOTA_ERROR_THRESHOLD_PERCENTAGE: u64 = 99;

// if recent quota is older,
// it is re-fetched on dc_get_connectivity_html()
pub const QUOTA_MAX_AGE_SECONDS: i64 = 60;

#[derive(Debug)]
pub struct QuotaInfo {
    /// Recently loaded quota information.
    /// set to `Err()` if the provider does not support quota or on other errors,
    /// set to `Ok()` for valid quota information.
    /// Updated by `Action::UpdateRecentQuota`
    pub(crate) recent: Result<IndexMap<String, Vec<QuotaResource>>>,

    /// Timestamp when structure was modified.
    pub(crate) modified: i64,
}

async fn get_unique_quota_roots_and_usage(
    folders: Vec<String>,
    imap: &mut Imap,
) -> Result<IndexMap<String, Vec<QuotaResource>>> {
    let mut unique_quota_roots: IndexMap<String, Vec<QuotaResource>> = IndexMap::new();
    for folder in folders {
        let (quota_roots, quotas) = &imap.get_quota_roots(&folder).await?;
        // if there are new quota roots found in this imap folder, add them to the list
        for qr_entries in quota_roots {
            for quota_root_name in &qr_entries.quota_root_names {
                // the quota for that quota root
                let quota: Quota = quotas
                    .iter()
                    .find(|q| &q.root_name == quota_root_name)
                    .cloned()
                    .ok_or_else(|| anyhow!("quota_root should have a quota"))?;
                // replace old quotas, because between fetching quotaroots for folders,
                // messages could be recieved and so the usage could have been changed
                *unique_quota_roots
                    .entry(quota_root_name.clone())
                    .or_insert(vec![]) = quota.resources;
            }
        }
    }
    Ok(unique_quota_roots)
}

impl Context {
    // Adds a job to update `quota.recent`
    pub(crate) async fn schedule_quota_update(&self) {
        job::kill_action(self, Action::UpdateRecentQuota).await;
        job::add(
            self,
            job::Job::new(Action::UpdateRecentQuota, 0, Params::new(), 0),
        )
        .await;
    }

    /// Updates `quota.recent`, sets `quota.modified` to the current time
    /// and emits an event to let the UIs update connectivity view.
    ///
    /// Called in response to `Action::UpdateRecentQuota`.
    pub(crate) async fn update_recent_quota(&self, imap: &mut Imap) -> Status {
        if let Err(err) = imap.prepare(self).await {
            warn!(self, "could not connect: {:?}", err);
            return Status::RetryNow;
        }

        let quota = if imap.can_check_quota() {
            let folders = get_watched_folders(self).await;
            get_unique_quota_roots_and_usage(folders, imap).await
        } else {
            Err(anyhow!("Quota not supported by your provider."))
        };

        *self.quota.write().await = Some(QuotaInfo {
            recent: quota,
            modified: time(),
        });

        self.emit_event(EventType::ConnectivityChanged);
        Status::Finished(Ok(()))
    }
}
