use core::fmt;
use std::{ops::Deref, sync::Arc};

use async_std::sync::{Mutex, RwLockReadGuard};

use crate::dc_tools::time;
use crate::events::EventType;
use crate::quota::{
    QUOTA_ERROR_THRESHOLD_PERCENTAGE, QUOTA_MAX_AGE_SECONDS, QUOTA_WARN_THRESHOLD_PERCENTAGE,
};
use crate::{config::Config, dc_tools, scheduler::Scheduler, stock_str};
use crate::{context::Context, log::LogExt};
use anyhow::{anyhow, Result};
use humansize::{file_size_opts, FileSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, PartialOrd, Ord)]
pub enum Connectivity {
    NotConnected = 1000,
    Connecting = 2000,
    /// Fetching or sending messages
    Working = 3000,
    Connected = 4000,
}

// The order of the connectivities is important: worse connectivities (i.e. those at
// the top) take priority. This means that e.g. if any folder has an error - usually
// because there is no internet connection - the connectivity for the whole
// account will be `Notconnected`.
#[derive(Debug, Clone, PartialEq, Eq, EnumProperty, PartialOrd)]
enum DetailedConnectivity {
    Error(String),
    Uninitialized,
    Connecting,
    Working,
    InterruptingIdle,
    Connected,

    /// The folder was configured not to be watched or configured_*_folder is not set
    NotConfigured,
}

impl Default for DetailedConnectivity {
    fn default() -> Self {
        DetailedConnectivity::Uninitialized
    }
}

impl DetailedConnectivity {
    fn to_basic(&self) -> Option<Connectivity> {
        match self {
            DetailedConnectivity::Error(_) => Some(Connectivity::NotConnected),
            DetailedConnectivity::Uninitialized => Some(Connectivity::NotConnected),
            DetailedConnectivity::Connecting => Some(Connectivity::Connecting),
            DetailedConnectivity::Working => Some(Connectivity::Working),
            DetailedConnectivity::InterruptingIdle => Some(Connectivity::Connected),
            DetailedConnectivity::Connected => Some(Connectivity::Connected),

            // Just don't return a connectivity, probably the folder is configured not to be
            // watched or there is e.g. no "Sent" folder, so we are not interested in it
            DetailedConnectivity::NotConfigured => None,
        }
    }

    fn to_icon(&self) -> String {
        match self {
            DetailedConnectivity::Error(_)
            | DetailedConnectivity::Uninitialized
            | DetailedConnectivity::NotConfigured => "<span class=\"red dot\"></span>".to_string(),
            DetailedConnectivity::Connecting => "<span class=\"yellow dot\"></span>".to_string(),
            DetailedConnectivity::Working
            | DetailedConnectivity::InterruptingIdle
            | DetailedConnectivity::Connected => "<span class=\"green dot\"></span>".to_string(),
        }
    }

    async fn to_string_imap(&self, context: &Context) -> String {
        match self {
            DetailedConnectivity::Error(e) => stock_str::error(context, e).await,
            DetailedConnectivity::Uninitialized => "Not started".to_string(),
            DetailedConnectivity::Connecting => stock_str::connecting(context).await,
            DetailedConnectivity::Working => stock_str::updating(context).await,
            DetailedConnectivity::InterruptingIdle | DetailedConnectivity::Connected => {
                stock_str::connected(context).await
            }
            DetailedConnectivity::NotConfigured => "Not configured".to_string(),
        }
    }

    async fn to_string_smtp(&self, context: &Context) -> String {
        match self {
            DetailedConnectivity::Error(e) => stock_str::error(context, e).await,
            DetailedConnectivity::Uninitialized => {
                "You did not try to send a message recently.".to_string()
            }
            DetailedConnectivity::Connecting => stock_str::connecting(context).await,
            DetailedConnectivity::Working => stock_str::sending(context).await,

            // We don't know any more than that the last message was sent successfully;
            // since sending the last message, connectivity could have changed, which we don't notice
            // until another message is sent
            DetailedConnectivity::InterruptingIdle | DetailedConnectivity::Connected => {
                stock_str::last_msg_sent_successfully(context).await
            }
            DetailedConnectivity::NotConfigured => "Not configured".to_string(),
        }
    }

    fn all_work_done(&self) -> bool {
        match self {
            DetailedConnectivity::Error(_) => true,
            DetailedConnectivity::Uninitialized => false,
            DetailedConnectivity::Connecting => false,
            DetailedConnectivity::Working => false,
            DetailedConnectivity::InterruptingIdle => false,
            DetailedConnectivity::Connected => true,
            DetailedConnectivity::NotConfigured => true,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ConnectivityStore(Arc<Mutex<DetailedConnectivity>>);

impl ConnectivityStore {
    async fn set(&self, context: &Context, v: DetailedConnectivity) {
        {
            *self.0.lock().await = v;
        }
        context.emit_event(EventType::ConnectivityChanged);
    }

    pub(crate) async fn set_err(&self, context: &Context, e: impl ToString) {
        self.set(context, DetailedConnectivity::Error(e.to_string()))
            .await;
    }
    pub(crate) async fn set_connecting(&self, context: &Context) {
        self.set(context, DetailedConnectivity::Connecting).await;
    }
    pub(crate) async fn set_working(&self, context: &Context) {
        self.set(context, DetailedConnectivity::Working).await;
    }
    pub(crate) async fn set_connected(&self, context: &Context) {
        self.set(context, DetailedConnectivity::Connected).await;
    }
    pub(crate) async fn set_not_configured(&self, context: &Context) {
        self.set(context, DetailedConnectivity::NotConfigured).await;
    }

    async fn get_detailed(&self) -> DetailedConnectivity {
        self.0.lock().await.deref().clone()
    }
    async fn get_basic(&self) -> Option<Connectivity> {
        self.0.lock().await.to_basic()
    }
    async fn get_all_work_done(&self) -> bool {
        self.0.lock().await.all_work_done()
    }
}

/// Set all folder states to InterruptingIdle in case they were `Connected` before.
/// Called during `dc_maybe_network()` to make sure that `dc_accounts_all_work_done()`
/// returns false immediately after `dc_maybe_network()`.
pub(crate) async fn idle_interrupted(scheduler: RwLockReadGuard<'_, Scheduler>) {
    let [inbox, mvbox, sentbox] = match &*scheduler {
        Scheduler::Running {
            inbox,
            mvbox,
            sentbox,
            ..
        } => [
            inbox.state.connectivity.clone(),
            mvbox.state.connectivity.clone(),
            sentbox.state.connectivity.clone(),
        ],
        Scheduler::Stopped => return,
    };
    drop(scheduler);

    let mut connectivity_lock = inbox.0.lock().await;
    // For the inbox, we also have to set the connectivity to InterruptingIdle if it was
    // NotConfigured before: If all folders are NotConfigured, dc_get_connectivity()
    // returns Connected. But after dc_maybe_network(), dc_get_connectivity() must not
    // return Connected until DC is completely done with fetching folders; this also
    // includes scan_folders() which happens on the inbox thread.
    if *connectivity_lock == DetailedConnectivity::Connected
        || *connectivity_lock == DetailedConnectivity::NotConfigured
    {
        *connectivity_lock = DetailedConnectivity::InterruptingIdle;
    }
    drop(connectivity_lock);

    for state in &[&mvbox, &sentbox] {
        let mut connectivity_lock = state.0.lock().await;
        if *connectivity_lock == DetailedConnectivity::Connected {
            *connectivity_lock = DetailedConnectivity::InterruptingIdle;
        }
    }
    // No need to send ConnectivityChanged, the user-facing connectivity doesn't change because
    // of what we do here.
}

/// Set the connectivity to "Not connected" after a call to dc_maybe_network_lost().
/// If we did not do this, the connectivity would stay "Connected" for quite a long time
/// after `maybe_network_lost()` was called.
pub(crate) async fn maybe_network_lost(
    context: &Context,
    scheduler: RwLockReadGuard<'_, Scheduler>,
) {
    let stores = match &*scheduler {
        Scheduler::Running {
            inbox,
            mvbox,
            sentbox,
            ..
        } => [
            inbox.state.connectivity.clone(),
            mvbox.state.connectivity.clone(),
            sentbox.state.connectivity.clone(),
        ],
        Scheduler::Stopped => return,
    };
    drop(scheduler);

    for store in &stores {
        let mut connectivity_lock = store.0.lock().await;
        if !matches!(
            *connectivity_lock,
            DetailedConnectivity::Uninitialized
                | DetailedConnectivity::Error(_)
                | DetailedConnectivity::NotConfigured,
        ) {
            *connectivity_lock = DetailedConnectivity::Error("Connection lost".to_string());
        }
        drop(connectivity_lock);
    }
    context.emit_event(EventType::ConnectivityChanged);
}

impl fmt::Debug for ConnectivityStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(guard) = self.0.try_lock() {
            write!(f, "ConnectivityStore {:?}", &*guard)
        } else {
            write!(f, "ConnectivityStore [LOCKED]")
        }
    }
}

impl Context {
    /// Get the current connectivity, i.e. whether the device is connected to the IMAP server.
    /// One of:
    /// - DC_CONNECTIVITY_NOT_CONNECTED (1000-1999): Show e.g. the string "Not connected" or a red dot
    /// - DC_CONNECTIVITY_CONNECTING (2000-2999): Show e.g. the string "Connecting…" or a yellow dot
    /// - DC_CONNECTIVITY_WORKING (3000-3999): Show e.g. the string "Updating…" or a spinning wheel
    /// - DC_CONNECTIVITY_CONNECTED (>=4000): Show e.g. the string "Connected" or a green dot
    ///
    /// We don't use exact values but ranges here so that we can split up
    /// states into multiple states in the future.
    ///
    /// Meant as a rough overview that can be shown
    /// e.g. in the title of the main screen.
    ///
    /// If the connectivity changes, a DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
    pub async fn get_connectivity(&self) -> Connectivity {
        let lock = self.scheduler.read().await;
        let stores: Vec<_> = match &*lock {
            Scheduler::Running {
                inbox,
                mvbox,
                sentbox,
                ..
            } => [&inbox.state, &mvbox.state, &sentbox.state]
                .iter()
                .map(|state| state.connectivity.clone())
                .collect(),
            Scheduler::Stopped => return Connectivity::NotConnected,
        };
        drop(lock);

        let mut connectivities = Vec::new();
        for s in stores {
            if let Some(connectivity) = s.get_basic().await {
                connectivities.push(connectivity);
            }
        }
        connectivities
            .into_iter()
            .min()
            .unwrap_or(Connectivity::Connected)
    }

    /// Get an overview of the current connectivity, and possibly more statistics.
    /// Meant to give the user more insight about the current status than
    /// the basic connectivity info returned by dc_get_connectivity(); show this
    /// e.g., if the user taps on said basic connectivity info.
    ///
    /// If this page changes, a DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
    ///
    /// This comes as an HTML from the core so that we can easily improve it
    /// and the improvement instantly reaches all UIs.
    pub async fn get_connectivity_html(&self) -> Result<String> {
        let mut ret = r#"<!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="initial-scale=1.0" />
                <style>
                    ul {
                        list-style-type: none;
                        padding-left: 1em;
                    }
                    .dot {
                        height: 0.9em; width: 0.9em;
                        border: 1px solid #888;
                        border-radius: 50%;
                        display: inline-block;
                        position: relative; left: -0.1em; top: 0.1em;
                    }
                    .bar {
                        width: 90%;
                        border: 1px solid #888;
                        border-radius: .5em;
                        margin-top: .2em;
                        margin-bottom: 1em;
                        position: relative; left: -0.2em;
                    }
                    .progress {
                        min-width:1.8em;
                        height: 1em;
                        border-radius: .45em;
                        color: white;
                        text-align: center;
                        padding-bottom: 2px;
                    }
                    .red {
                        background-color: #f33b2d;
                    }
                    .green {
                        background-color: #34c759;
                    }
                    .yellow {
                        background-color: #fdc625;
                    }
                </style>
            </head>
            <body>"#
            .to_string();

        let lock = self.scheduler.read().await;
        let (folders_states, smtp) = match &*lock {
            Scheduler::Running {
                inbox,
                mvbox,
                sentbox,
                smtp,
                ..
            } => (
                [
                    (
                        Config::ConfiguredInboxFolder,
                        Config::InboxWatch,
                        inbox.state.connectivity.clone(),
                    ),
                    (
                        Config::ConfiguredMvboxFolder,
                        Config::MvboxWatch,
                        mvbox.state.connectivity.clone(),
                    ),
                    (
                        Config::ConfiguredSentboxFolder,
                        Config::SentboxWatch,
                        sentbox.state.connectivity.clone(),
                    ),
                ],
                smtp.state.connectivity.clone(),
            ),
            Scheduler::Stopped => {
                return Err(anyhow!("Not started"));
            }
        };
        drop(lock);

        ret += &format!("<h3>{}</h3><ul>", stock_str::incoming_messages(self).await);
        for (folder, watch, state) in &folders_states {
            let w = self.get_config(*watch).await.ok_or_log(self);

            let mut folder_added = false;
            if w.flatten() == Some("1".to_string()) {
                let f = self.get_config(*folder).await.ok_or_log(self).flatten();

                if let Some(foldername) = f {
                    let detailed = &state.get_detailed().await;
                    ret += "<li>";
                    ret += &*detailed.to_icon();
                    ret += " <b>";
                    ret += &*escaper::encode_minimal(&foldername);
                    ret += ":</b> ";
                    ret += &*escaper::encode_minimal(&*detailed.to_string_imap(self).await);
                    ret += "</li>";

                    folder_added = true;
                }
            }

            if !folder_added && folder == &Config::ConfiguredInboxFolder {
                let detailed = &state.get_detailed().await;
                if let DetailedConnectivity::Error(_) = detailed {
                    // On the inbox thread, we also do some other things like scan_folders and run jobs
                    // so, maybe, the inbox is not watched, but something else went wrong
                    ret += "<li>";
                    ret += &*detailed.to_icon();
                    ret += " ";
                    ret += &*escaper::encode_minimal(&detailed.to_string_imap(self).await);
                    ret += "</li>";
                }
            }
        }
        ret += "</ul>";

        ret += &format!(
            "<h3>{}</h3><ul><li>",
            stock_str::outgoing_messages(self).await
        );
        let detailed = smtp.get_detailed().await;
        ret += &*detailed.to_icon();
        ret += " ";
        ret += &*escaper::encode_minimal(&detailed.to_string_smtp(self).await);
        ret += "</li></ul>";

        let domain = dc_tools::EmailAddress::new(
            &self
                .get_config(Config::ConfiguredAddr)
                .await?
                .unwrap_or_default(),
        )?
        .domain;
        ret += &format!(
            "<h3>{}</h3><ul>",
            stock_str::storage_on_domain(self, domain).await
        );
        let quota = self.quota.read().await;
        if let Some(quota) = &*quota {
            match &quota.recent {
                Ok(quota) => {
                    let roots_cnt = quota.len();
                    for (root_name, resources) in quota {
                        use async_imap::types::QuotaResourceName::*;
                        for resource in resources {
                            ret += "<li>";

                            // root name is empty eg. for gmail and redundant eg. for riseup.
                            // therefore, use it only if there are really several roots.
                            if roots_cnt > 1 && !root_name.is_empty() {
                                ret +=
                                    &format!("<b>{}:</b> ", &*escaper::encode_minimal(root_name));
                            } else {
                                info!(self, "connectivity: root name hidden: \"{}\"", root_name);
                            }

                            ret += &match &resource.name {
                                Atom(resource_name) => {
                                    format!(
                                        "<b>{}:</b> {} of {} used",
                                        &*escaper::encode_minimal(resource_name),
                                        resource.usage.to_string(),
                                        resource.limit.to_string(),
                                    )
                                }
                                Message => {
                                    format!(
                                        "<b>{}:</b> {} of {} used",
                                        stock_str::messages(self).await,
                                        resource.usage.to_string(),
                                        resource.limit.to_string(),
                                    )
                                }
                                Storage => {
                                    // do not use a special title needed for "Storage":
                                    // - it is usually shown directly under the "Storage" headline
                                    // - by the units "1 MB of 10 MB used" there is some difference to eg. "Messages: 1 of 10 used"
                                    // - the string is not longer than the other strings that way (minus title, plus units) -
                                    //   additional linebreaks on small displays are unlikely therefore
                                    // - most times, this is the only item anyway
                                    let usage = (resource.usage * 1024)
                                        .file_size(file_size_opts::BINARY)
                                        .unwrap_or_default();
                                    let limit = (resource.limit * 1024)
                                        .file_size(file_size_opts::BINARY)
                                        .unwrap_or_default();
                                    format!("{} of {} used", usage, limit)
                                }
                            };

                            let percent = resource.get_usage_percentage();
                            let color = if percent >= QUOTA_ERROR_THRESHOLD_PERCENTAGE {
                                "red"
                            } else if percent >= QUOTA_WARN_THRESHOLD_PERCENTAGE {
                                "yellow"
                            } else {
                                "green"
                            };
                            ret += &format!("<div class=\"bar\"><div class=\"progress {}\" style=\"width: {}%\">{}%</div></div>", color, percent, percent);

                            ret += "</li>";
                        }
                    }
                }
                Err(e) => {
                    ret += format!("<li>{}</li>", e).as_str();
                }
            }

            if quota.modified + QUOTA_MAX_AGE_SECONDS < time() {
                self.schedule_quota_update().await?;
            }
        } else {
            ret += &format!("<li>{}</li>", stock_str::one_moment(self).await);
            self.schedule_quota_update().await?;
        }
        ret += "</ul>";

        ret += "</body></html>\n";
        Ok(ret)
    }

    pub async fn all_work_done(&self) -> bool {
        let lock = self.scheduler.read().await;
        let stores: Vec<_> = match &*lock {
            Scheduler::Running {
                inbox,
                mvbox,
                sentbox,
                smtp,
                ..
            } => [&inbox.state, &mvbox.state, &sentbox.state, &smtp.state]
                .iter()
                .map(|state| state.connectivity.clone())
                .collect(),
            Scheduler::Stopped => return false,
        };
        drop(lock);

        for s in &stores {
            if !s.get_all_work_done().await {
                return false;
            }
        }
        true
    }
}
