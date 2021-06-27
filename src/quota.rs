use anyhow::{anyhow, bail, Result};
use async_imap::types::{Quota, QuotaResource};
use humansize::{file_size_opts, FileSize};
use itertools::Itertools;

use crate::context::Context;
use crate::imap::Imap;
use crate::stock_str::{
    quota_mailbox_nearly_full, quota_not_supported, quota_resource_messages, quota_resource_storage,
};
use crate::{
    chat::{add_device_msg, add_device_msg_with_importance},
    constants::Viewtype,
    imap::scan_folders::get_watched_folders,
    message::Message,
};

/// warn about a nearly full mailbox after this usage percentage is reached.
pub const QUOTA_WARN_THRESHOLD_PERCENTAGE: u64 = 90;

/// Seconds until the quota will be checked again
pub const CHECK_QUOTA_FREQUENCY: i64 = 60 * 60 * 24;

/// Generates a detailed report about the current Quota usage on the for deltachat relevant folders
/// and sends it to the user via [add_device_msg]
///
/// It's a bit like the prepaid mobile carrier service menu/messages,
/// where you type a special number and then get a message back with your current balance.
pub(crate) async fn quota_usage_report_job(context: &Context, imap: &mut Imap) -> Result<()> {
    if let Err(err) = imap.prepare(context).await {
        warn!(context, "could not connect: {:?}", err);
        bail!("imap is not ready");
    }

    if imap.can_check_quota() {
        let folders = get_watched_folders(context).await;
        let unique_quota_roots = get_unique_quota_roots_and_usage(folders, imap).await?;

        // build and send report message
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(generate_report_message(&unique_quota_roots, context).await?);
        add_device_msg_with_importance(context, None, Some(&mut msg), true).await?;
    } else {
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(quota_not_supported(context).await);
        add_device_msg_with_importance(context, None, Some(&mut msg), true).await?;
    }

    Ok(())
}

async fn generate_report_message(
    unique_quota_roots: &[(String, Vec<QuotaResource>)],
    context: &Context,
) -> Result<String> {
    let mut message = String::new();

    let storage_stock_string = quota_resource_storage(context).await;
    let messages_stock_string = quota_resource_messages(context).await;
    for (name, quota_resources) in unique_quota_roots {
        message.push_str(&format!("{}:\n", &name));
        use async_imap::types::QuotaResourceName::*;
        for resource in quota_resources {
            message.push_str(&match &resource.name {
                Atom(name) => {
                    format!("{}: {}/{} \n", name, resource.usage, resource.limit)
                }
                Message => {
                    format!(
                        "{}: {}/{}\n",
                        messages_stock_string, resource.usage, resource.limit
                    )
                }
                Storage => {
                    let used = (resource.usage * 1024)
                        .file_size(file_size_opts::BINARY)
                        .map_err(|err| anyhow!("{}", err))?;
                    let limit = (resource.limit * 1024)
                        .file_size(file_size_opts::BINARY)
                        .map_err(|err| anyhow!("{}", err))?;
                    format!("{}: {}/{}\n", storage_stock_string, used, limit)
                }
            });
        }
    }
    Ok(message)
}

async fn get_unique_quota_roots_and_usage(
    folders: Vec<String>,
    imap: &mut Imap,
) -> Result<Vec<(String, Vec<QuotaResource>)>> {
    // IDEA: OPTIMIZATION:
    // the unique quota roots of get_unique_quota_roots_and_usage could be cached and then the server could be asked for the quotas of those.
    // this would reduce incoming traffic and for most cases also outgoing traffic, because most email server share quota roots across folders/mailboxes.
    // Again at the cost of increasing code complexity and the question for how long it should be cached
    // IDEA2: maybe the provider db could also contain quota root names?
    let mut unique_quota_roots: Vec<(String, Vec<QuotaResource>)> = Vec::new();
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
                match unique_quota_roots
                    .iter()
                    .find_position(|(root_name, _)| root_name == quota_root_name)
                {
                    None => {
                        unique_quota_roots.push((quota_root_name.clone(), quota.resources));
                    }
                    Some((position, ..)) => {
                        // replace old quotas, because between fetching quotaroots for folders,
                        // messages could be recieved and so the usage could have been changed
                        unique_quota_roots.get_mut(position).unwrap().1 = quota.resources;
                    }
                }
            }
        }
    }
    Ok(unique_quota_roots)
}

fn get_highest_usage<'t>(
    unique_quota_roots: &'t [(String, Vec<QuotaResource>)],
) -> Result<(u64, &'t String, &QuotaResource)> {
    let mut highest: Option<(u64, &'t String, &QuotaResource)> = None;
    for (name, resources) in unique_quota_roots {
        for r in resources {
            let usage_percent = r.get_usage_percentage();
            match highest {
                None => {
                    highest = Some((usage_percent, name, r));
                }
                Some((up, ..)) => {
                    if up <= usage_percent {
                        highest = Some((usage_percent, name, r));
                    }
                }
            };
        }
    }

    Ok(highest.ok_or_else(|| anyhow!("no quota_resource found, this is unexpected"))?)
}

pub(crate) async fn check_quota_job(context: &Context, imap: &mut Imap) -> Result<()> {
    if let Err(err) = imap.prepare(context).await {
        warn!(context, "could not connect: {:?}", err);
        bail!("imap is not ready");
    }

    if !imap.can_check_quota() {
        warn!(
            context,
            "QuotaCheck: the email server does not support the quota extention"
        );
    } else {
        let folders = get_watched_folders(context).await;
        let unique_quota_roots = get_unique_quota_roots_and_usage(folders, imap).await?;
        if unique_quota_roots.is_empty() {
            bail!("no quota root");
        }
        // whats the highest quota
        let (usage_percentage, root_name, quota_resource) = get_highest_usage(&unique_quota_roots)?;
        // post highest quota to info! for debugging purposes
        info!(
            context,
            "QuotaCheck: highest QuotaResource is {}% full: {:?} (root_name: {})",
            usage_percentage,
            quota_resource,
            root_name
        );
        // check if highest usage percent reaches warning threshold
        if usage_percentage >= QUOTA_WARN_THRESHOLD_PERCENTAGE {
            // why log it? because then we can see it also in logs users might send us.
            warn!(
                context,
                "QuotaCheck: resource usage percentage({}%) higher than threshold({}%)",
                usage_percentage,
                QUOTA_WARN_THRESHOLD_PERCENTAGE
            );

            let mut details_msg = Message::new(Viewtype::Text);
            details_msg.text = Some(generate_report_message(&unique_quota_roots, context).await?);
            add_device_msg(context, None, Some(&mut details_msg)).await?;

            // if yes post a device message informing the user that the mailbox is nearly full.
            let mut msg = Message::new(Viewtype::Text);
            msg.text = Some(quota_mailbox_nearly_full(context).await);
            add_device_msg_with_importance(context, None, Some(&mut msg), true).await?;
        }
    }

    Ok(())
}
