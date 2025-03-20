use core::fmt;
use std::cmp::min;
use std::{iter::once, ops::Deref, sync::Arc};

use anyhow::Result;
use humansize::{format_size, BINARY};
use tokio::sync::Mutex;

use crate::events::EventType;
use crate::imap::{scan_folders::get_watched_folder_configs, FolderMeaning};
use crate::login_param::ConfiguredLoginParam;
use crate::quota::{QUOTA_ERROR_THRESHOLD_PERCENTAGE, QUOTA_WARN_THRESHOLD_PERCENTAGE};
use crate::stock_str;
use crate::{context::Context, log::LogExt};

use super::InnerSchedulerState;

/// Rough connectivity status for display in the status bar in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, PartialOrd, Ord)]
pub enum Connectivity {
    /// Not connected.
    ///
    /// This may be because we just started,
    /// because we lost connection and
    /// were not able to connect and log in yet
    /// or because I/O is not started.
    NotConnected = 1000,

    /// Attempting to connect and log in.
    Connecting = 2000,

    /// Fetching or sending messages.
    Working = 3000,

    /// We are connected but not doing anything.
    ///
    /// This is the most common state,
    /// so mobile UIs display the profile name
    /// instead of connectivity status in this state.
    /// Desktop UI displays "Connected" in the tooltip,
    /// which signals that no more messages
    /// are coming in.
    Connected = 4000,
}

// The order of the connectivities is important: worse connectivities (i.e. those at
// the top) take priority. This means that e.g. if any folder has an error - usually
// because there is no internet connection - the connectivity for the whole
// account will be `Notconnected`.
#[derive(Debug, Default, Clone, PartialEq, Eq, EnumProperty, PartialOrd)]
enum DetailedConnectivity {
    Error(String),
    #[default]
    Uninitialized,

    /// Attempting to connect,
    /// until we successfully log in.
    Connecting,

    /// Connection is just established,
    /// there may be work to do.
    Preparing,

    /// There is actual work to do, e.g. there are messages in SMTP queue
    /// or we detected a message on IMAP server that should be downloaded.
    Working,

    InterruptingIdle,

    /// Connection is established and is idle.
    Idle,

    /// The folder was configured not to be watched or configured_*_folder is not set
    NotConfigured,
}

impl DetailedConnectivity {
    fn to_basic(&self) -> Option<Connectivity> {
        match self {
            DetailedConnectivity::Error(_) => Some(Connectivity::NotConnected),
            DetailedConnectivity::Uninitialized => Some(Connectivity::NotConnected),
            DetailedConnectivity::Connecting => Some(Connectivity::Connecting),
            DetailedConnectivity::Working => Some(Connectivity::Working),
            DetailedConnectivity::InterruptingIdle => Some(Connectivity::Working),

            // At this point IMAP has just connected,
            // but does not know yet if there are messages to download.
            // We still convert this to Working state
            // so user can see "Updating..." and not "Connected"
            // which is reserved for idle state.
            DetailedConnectivity::Preparing => Some(Connectivity::Working),

            // Just don't return a connectivity, probably the folder is configured not to be
            // watched or there is e.g. no "Sent" folder, so we are not interested in it
            DetailedConnectivity::NotConfigured => None,

            DetailedConnectivity::Idle => Some(Connectivity::Connected),
        }
    }

    fn to_icon(&self) -> String {
        match self {
            DetailedConnectivity::Error(_)
            | DetailedConnectivity::Uninitialized
            | DetailedConnectivity::NotConfigured => "<span class=\"red dot\"></span>".to_string(),
            DetailedConnectivity::Connecting => "<span class=\"yellow dot\"></span>".to_string(),
            DetailedConnectivity::Preparing
            | DetailedConnectivity::Working
            | DetailedConnectivity::InterruptingIdle
            | DetailedConnectivity::Idle => "<span class=\"green dot\"></span>".to_string(),
        }
    }

    async fn to_string_imap(&self, context: &Context) -> String {
        match self {
            DetailedConnectivity::Error(e) => stock_str::error(context, e).await,
            DetailedConnectivity::Uninitialized => "Not started".to_string(),
            DetailedConnectivity::Connecting => stock_str::connecting(context).await,
            DetailedConnectivity::Preparing | DetailedConnectivity::Working => {
                stock_str::updating(context).await
            }
            DetailedConnectivity::InterruptingIdle | DetailedConnectivity::Idle => {
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
            DetailedConnectivity::InterruptingIdle
            | DetailedConnectivity::Preparing
            | DetailedConnectivity::Idle => stock_str::last_msg_sent_successfully(context).await,
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
            DetailedConnectivity::Preparing => false, // Just connected, there may still be work to do.
            DetailedConnectivity::NotConfigured => true,
            DetailedConnectivity::Idle => true,
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
    pub(crate) async fn set_preparing(&self, context: &Context) {
        self.set(context, DetailedConnectivity::Preparing).await;
    }
    pub(crate) async fn set_not_configured(&self, context: &Context) {
        self.set(context, DetailedConnectivity::NotConfigured).await;
    }
    pub(crate) async fn set_idle(&self, context: &Context) {
        self.set(context, DetailedConnectivity::Idle).await;
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

/// Set all folder states to InterruptingIdle in case they were `Idle` before.
/// Called during `dc_maybe_network()` to make sure that `all_work_done()`
/// returns false immediately after `dc_maybe_network()`.
pub(crate) async fn idle_interrupted(inbox: ConnectivityStore, oboxes: Vec<ConnectivityStore>) {
    let mut connectivity_lock = inbox.0.lock().await;
    // For the inbox, we also have to set the connectivity to InterruptingIdle if it was
    // NotConfigured before: If all folders are NotConfigured, dc_get_connectivity()
    // returns Connected. But after dc_maybe_network(), dc_get_connectivity() must not
    // return Connected until DC is completely done with fetching folders; this also
    // includes scan_folders() which happens on the inbox thread.
    if *connectivity_lock == DetailedConnectivity::Idle
        || *connectivity_lock == DetailedConnectivity::NotConfigured
    {
        *connectivity_lock = DetailedConnectivity::InterruptingIdle;
    }
    drop(connectivity_lock);

    for state in oboxes {
        let mut connectivity_lock = state.0.lock().await;
        if *connectivity_lock == DetailedConnectivity::Idle {
            *connectivity_lock = DetailedConnectivity::InterruptingIdle;
        }
    }
    // No need to send ConnectivityChanged, the user-facing connectivity doesn't change because
    // of what we do here.
}

/// Set the connectivity to "Not connected" after a call to dc_maybe_network_lost().
/// If we did not do this, the connectivity would stay "Connected" for quite a long time
/// after `maybe_network_lost()` was called.
pub(crate) async fn maybe_network_lost(context: &Context, stores: Vec<ConnectivityStore>) {
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
        if let Ok(guard) = self.0.try_lock() {
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
        let lock = self.scheduler.inner.read().await;
        let stores: Vec<_> = match *lock {
            InnerSchedulerState::Started(ref sched) => sched
                .boxes()
                .map(|b| b.conn_state.state.connectivity.clone())
                .collect(),
            _ => return Connectivity::NotConnected,
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
                <meta name="viewport" content="initial-scale=1.0; user-scalable=no" />
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

        // =============================================================================================
        //                              Get the states from the RwLock
        // =============================================================================================

        let lock = self.scheduler.inner.read().await;
        let (folders_states, smtp) = match *lock {
            InnerSchedulerState::Started(ref sched) => (
                sched
                    .boxes()
                    .map(|b| (b.meaning, b.conn_state.state.connectivity.clone()))
                    .collect::<Vec<_>>(),
                sched.smtp.state.connectivity.clone(),
            ),
            _ => {
                ret += &format!(
                    "<h3>{}</h3>\n</body></html>\n",
                    stock_str::not_connected(self).await
                );
                return Ok(ret);
            }
        };
        drop(lock);

        // =============================================================================================
        // Add e.g.
        //                              Incoming messages
        //                               - "Inbox": Connected
        //                               - "Sent": Connected
        // =============================================================================================

        let watched_folders = get_watched_folder_configs(self).await?;
        let incoming_messages = stock_str::incoming_messages(self).await;
        ret += &format!("<h3>{incoming_messages}</h3><ul>");
        for (folder, state) in &folders_states {
            let mut folder_added = false;

            if let Some(config) = folder.to_config().filter(|c| watched_folders.contains(c)) {
                let f = self.get_config(config).await.log_err(self).ok().flatten();

                if let Some(foldername) = f {
                    let detailed = &state.get_detailed().await;
                    ret += "<li>";
                    ret += &*detailed.to_icon();
                    ret += " <b>";
                    ret += &*escaper::encode_minimal(&foldername);
                    ret += ":</b> ";
                    ret += &*escaper::encode_minimal(&detailed.to_string_imap(self).await);
                    ret += "</li>";

                    folder_added = true;
                }
            }

            if !folder_added && folder == &FolderMeaning::Inbox {
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

        // =============================================================================================
        // Add e.g.
        //                              Outgoing messages
        //                                Your last message was sent successfully
        // =============================================================================================

        let outgoing_messages = stock_str::outgoing_messages(self).await;
        ret += &format!("<h3>{outgoing_messages}</h3><ul><li>");
        let detailed = smtp.get_detailed().await;
        ret += &*detailed.to_icon();
        ret += " ";
        ret += &*escaper::encode_minimal(&detailed.to_string_smtp(self).await);
        ret += "</li></ul>";

        // =============================================================================================
        // Add e.g.
        //                              Storage on testrun.org
        //                                1.34 GiB of 2 GiB used
        //                                [======67%=====       ]
        // =============================================================================================

        let domain =
            &deltachat_contact_tools::EmailAddress::new(&self.get_primary_self_addr().await?)?
                .domain;
        let storage_on_domain = stock_str::storage_on_domain(self, domain).await;
        ret += &format!("<h3>{storage_on_domain}</h3><ul>");
        let quota = self.quota.read().await;
        if let Some(quota) = &*quota {
            match &quota.recent {
                Ok(quota) => {
                    if !quota.is_empty() {
                        for (root_name, resources) in quota {
                            use async_imap::types::QuotaResourceName::*;
                            for resource in resources {
                                ret += "<li>";

                                // root name is empty eg. for gmail and redundant eg. for riseup.
                                // therefore, use it only if there are really several roots.
                                if quota.len() > 1 && !root_name.is_empty() {
                                    ret += &format!(
                                        "<b>{}:</b> ",
                                        &*escaper::encode_minimal(root_name)
                                    );
                                } else {
                                    info!(
                                        self,
                                        "connectivity: root name hidden: \"{}\"", root_name
                                    );
                                }

                                let messages = stock_str::messages(self).await;
                                let part_of_total_used = stock_str::part_of_total_used(
                                    self,
                                    &resource.usage.to_string(),
                                    &resource.limit.to_string(),
                                )
                                .await;
                                ret += &match &resource.name {
                                    Atom(resource_name) => {
                                        format!(
                                            "<b>{}:</b> {}",
                                            &*escaper::encode_minimal(resource_name),
                                            part_of_total_used
                                        )
                                    }
                                    Message => {
                                        format!("<b>{part_of_total_used}:</b> {messages}")
                                    }
                                    Storage => {
                                        // do not use a special title needed for "Storage":
                                        // - it is usually shown directly under the "Storage" headline
                                        // - by the units "1 MB of 10 MB used" there is some difference to eg. "Messages: 1 of 10 used"
                                        // - the string is not longer than the other strings that way (minus title, plus units) -
                                        //   additional linebreaks on small displays are unlikely therefore
                                        // - most times, this is the only item anyway
                                        let usage = &format_size(resource.usage * 1024, BINARY);
                                        let limit = &format_size(resource.limit * 1024, BINARY);
                                        stock_str::part_of_total_used(self, usage, limit).await
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
                                let div_width_percent = min(100, percent);
                                ret += &format!("<div class=\"bar\"><div class=\"progress {color}\" style=\"width: {div_width_percent}%\">{percent}%</div></div>");

                                ret += "</li>";
                            }
                        }
                    } else {
                        ret += format!("<li>Warning: {domain} claims to support quota but gives no information</li>").as_str();
                    }
                }
                Err(e) => {
                    ret += format!("<li>{e}</li>").as_str();
                }
            }
        } else {
            let not_connected = stock_str::not_connected(self).await;
            ret += &format!("<li>{not_connected}</li>");
        }
        ret += "</ul>";

        // =============================================================================================
        // Add e.g.
        //                              Security
        //                                TLS Certificate Checks: enabled
        // =============================================================================================

        if let Some(configured_login_param) = ConfiguredLoginParam::load(self).await? {
            let security = stock_str::security(self).await;
            ret += &format!("<h3>{security}</h3><ul>");

            ret += "<li>";
            if configured_login_param.strict_tls() {
                // GREEN: strict TLS checks are enabled.
                ret += &format!(
                    "<span class=\"green dot\"></span> <b>TLS Certificate Checks:</b> enabled"
                );
            } else if configured_login_param.strict_tls_manually_disabled() {
                // RED: TLS checks are manually disabled.
                ret += &format!(
                    "<span class=\"red dot\"></span> <b>TLS Certificate Checks:</b> disabled"
                );
            } else {
                // YELLOW: TLS checks are automatically disabled.
                ret += &format!(
                    "<span class=\"yellow dot\"></span> <b>TLS Certificate Checks:</b> disabled"
                );
            }
            ret += "</li></ul>";
        }

        // =============================================================================================

        ret += "</body></html>\n";
        Ok(ret)
    }

    /// Returns true if all background work is done.
    async fn all_work_done(&self) -> bool {
        let lock = self.scheduler.inner.read().await;
        let stores: Vec<_> = match *lock {
            InnerSchedulerState::Started(ref sched) => sched
                .boxes()
                .map(|b| &b.conn_state.state)
                .chain(once(&sched.smtp.state))
                .map(|state| state.connectivity.clone())
                .collect(),
            _ => return false,
        };
        drop(lock);

        for s in &stores {
            if !s.get_all_work_done().await {
                return false;
            }
        }
        true
    }

    /// Waits until background work is finished.
    pub async fn wait_for_all_work_done(&self) {
        // Ideally we could wait for connectivity change events,
        // but sleep loop is good enough.

        // First 100 ms sleep in chunks of 10 ms.
        for _ in 0..10 {
            if self.all_work_done().await {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // If we are not finished in 100 ms, keep waking up every 100 ms.
        while !self.all_work_done().await {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}
