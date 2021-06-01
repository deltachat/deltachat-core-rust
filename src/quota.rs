use anyhow::{anyhow, Result};
use async_imap::imap_proto::{Quota, QuotaResource};
use humansize::{file_size_opts, FileSize};
use itertools::Itertools;

use crate::context::Context;
use crate::imap::Imap;
use crate::{
    chat::add_device_msg, constants::Viewtype, imap::scan_folders::get_watched_folders,
    message::Message,
};

pub(crate) async fn quota_usage_report_job(context: &Context, imap: &mut Imap) -> Result<()> {
    if imap.check_for_quota_support().await? {
        let folders = get_watched_folders(&context).await;
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

        // build report message
        let mut message: String = "".to_owned();

        for (name, quota_resources) in unique_quota_roots {
            message.push_str(&format!("{}:\n", &name));
            use async_imap::imap_proto::QuotaResourceName::*;
            for resource in quota_resources {
                message.push_str(&match resource.name {
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

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(message);
        add_device_msg(&context, None, Some(&mut msg)).await?;
    } else {
        warn!(
            context,
            "the email server does not support the quota extention"
        );
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("the email server does not support the quota extention".to_owned()); // todo stock string?
        add_device_msg(&context, None, Some(&mut msg)).await?;
    }

    Ok(())
}
