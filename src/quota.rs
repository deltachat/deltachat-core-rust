use anyhow::{anyhow, bail, Result};
use async_imap::imap_proto::{Quota, QuotaResource};
use humansize::{file_size_opts, FileSize};
use itertools::Itertools;

use crate::constants::DC_QUOTA_WARN_THRESHOLD_PERCENTAGE;
use crate::context::Context;
use crate::imap::Imap;
use crate::{
    chat::{add_device_msg, add_device_msg_with_importance},
    constants::Viewtype,
    imap::scan_folders::get_watched_folders,
    message::Message,
};

pub(crate) async fn quota_usage_report_job(context: &Context, imap: &mut Imap) -> Result<()> {
    // IDEA: OPTIMIZATION: check_for_quota_support could be cached in the config, would increase code complexity but decrease traffic a bit
    if imap.check_for_quota_support().await? {
        let folders = get_watched_folders(&context).await;
        let unique_quota_roots = get_unique_quota_roots_and_usage(folders, imap).await?;

        // build and send report message
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(generate_report_message(&unique_quota_roots)?);
        add_device_msg(&context, None, Some(&mut msg)).await?;
    } else {
        warn!(
            context,
            "the email server does not support the quota extention"
        );
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("the email server does not support the quota extention".to_owned()); // todo stock string?
        add_device_msg_with_importance(&context, None, Some(&mut msg), true).await?;
    }

    Ok(())
}

fn generate_report_message(
    unique_quota_roots: &Vec<(String, Vec<QuotaResource>)>,
) -> Result<String> {
    let mut message: String = "".to_owned();
    for (name, quota_resources) in unique_quota_roots {
        message.push_str(&format!("{}:\n", &name));
        use async_imap::imap_proto::QuotaResourceName::*;
        for resource in quota_resources {
            message.push_str(&match &resource.name {
                Atom(name) => {
                    format!("[{}/{}] {}\n", resource.usage, resource.limit, name)
                }
                Message => {
                    format!("{}/{} Messages\n", resource.usage, resource.limit)
                } // TODO stockstring
                Storage => {
                    let used = (resource.usage * 1024)
                        .file_size(file_size_opts::BINARY)
                        .map_err(|err| anyhow!("{}", err))?;
                    let limit = (resource.limit * 1024)
                        .file_size(file_size_opts::BINARY)
                        .map_err(|err| anyhow!("{}", err))?;
                    format!("{}/{} Storage\n", used, limit) // TODO stockstring
                }
            });
        }
    }
    Ok(message)
}

async fn get_unique_quota_roots_and_usage(
    folders: Vec<String>,
    imap: &mut Imap,
) -> Result<Vec<(String, Vec<QuotaResource<'static>>)>> {
    // IDEA: OPTIMIZATION:
    // the unique quota roots of get_unique_quota_roots_and_usage could be cached and then the server could be asked for the quotas of those.
    // this would reduce incoming traffic and for most cases also outgoing traffic, because most email server share quota roots across folders/mailboxes.
    // Again at the cost of increasing code complexity and the question for how long it should be cached
    // IDEA2: maybe the provider db could also contain quota root names?
    let mut unique_quota_roots: Vec<(String, Vec<QuotaResource<'static>>)> = Vec::new();
    for folder in folders {
        let (quota_roots, quotas) = &imap.get_quota_roots(&folder).await?;
        // if there are new quota roots found in this imap folder, add them to the list
        for qr_entries in quota_roots {
            for quota_root_name in &qr_entries.quota_root_names {
                // the quota for that quota root
                let quota: Quota<'static> = quotas
                    .iter()
                    .find(|q| &q.root_name == quota_root_name)
                    .map(|q| q.clone().into_owned())
                    .ok_or(anyhow!("quota_root should have a quota"))?;
                match unique_quota_roots
                    .iter()
                    .find_position(|(root_name, _)| root_name == quota_root_name)
                {
                    None => {
                        unique_quota_roots
                            .push((quota_root_name.clone().into_owned(), quota.resources));
                    }
                    Some((position, ..)) => {
                        // replace old quotas, because between fetching quotaroots for folders,
                        // messages could be recieved and so the usage could have been changed
                        unique_quota_roots[position].1 = quota.resources;
                    }
                }
            }
        }
    }
    Ok(unique_quota_roots)
}

fn get_highest_usage<'t>(
    unique_quota_roots: &'t Vec<(String, Vec<QuotaResource<'t>>)>,
) -> Result<(u64, &'t String, &QuotaResource<'t>)> {
    let mut highest: Option<(u64, &'t String, &QuotaResource<'t>)> = None;
    for (name, resources) in unique_quota_roots {
        for r in resources {
            let usage_percent = r.usage.saturating_mul(100) / r.limit;
            match highest {
                None => {
                    highest = Some((usage_percent, name, r));
                }
                Some((up, ..)) if up <= usage_percent => {
                    highest = Some((usage_percent, name, r));
                }
                Some(_) => {
                    unreachable!()
                }
            };
        }
    }

    return Ok(highest.ok_or(anyhow!("no quota_resource found, this is unexpected"))?);
}

pub(crate) async fn check_quota_job(context: &Context, imap: &mut Imap) -> Result<()> {
    // does server support quota
    if !imap.check_for_quota_support().await? {
        warn!(
            context,
            "QuotaCheck: the email server does not support the quota extention"
        );
    } else {
        let folders = get_watched_folders(&context).await;
        let unique_quota_roots = get_unique_quota_roots_and_usage(folders, imap).await?;
        if unique_quota_roots.len() == 0 {
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
        if usage_percentage >= DC_QUOTA_WARN_THRESHOLD_PERCENTAGE {
            // why log it? because then we can see it also in logs users might send us.
            warn!(
                context,
                "QuotaCheck: resource usage percentage({}%) higher than threshold({}%)",
                usage_percentage,
                DC_QUOTA_WARN_THRESHOLD_PERCENTAGE
            );

            let mut details_msg = Message::new(Viewtype::Text);
            details_msg.text = Some(generate_report_message(&unique_quota_roots)?);
            add_device_msg(&context, None, Some(&mut details_msg)).await?;

            // if yes post a device message informing the user that the mailbox is nearly full.
            let mut msg = Message::new(Viewtype::Text);
            msg.text = Some("Your mailbox on your email account is running full!\n Possible Solutions:\n - Delete old messages on the server\n- or enable \"Delete old messages from server\" in the deltachat settings\n- or upgrade your plan with your email provider\nIf you don't take action you will soon be unable to recieve messages.".to_owned()); // todo stock string?
            add_device_msg_with_importance(&context, None, Some(&mut msg), true).await?;
        }
    }

    Ok(())
}
