use anyhow::{anyhow, Result};
use async_imap::types::{Quota, QuotaResource};
use indexmap::IndexMap;

use crate::imap::Imap;

pub(crate) async fn get_unique_quota_roots_and_usage(
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
