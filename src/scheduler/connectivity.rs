use core::fmt;
use std::{ops::Deref, sync::Arc};

use async_std::sync::Mutex;

use crate::events::EventType;
use crate::{config::Config, scheduler::Scheduler};
use crate::{context::Context, log::LogExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, PartialOrd, Ord)]
pub enum Connectivity {
    NotConnected = 0,
    Connecting = 1,
    Connected = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty, PartialOrd, Ord)]
pub enum DetailedConnectivity {
    Error(String),
    Uninitialized,
    Connecting,
    Working,
    Connected,
}

impl DetailedConnectivity {
    pub fn to_basic(&self) -> Connectivity {
        match self {
            DetailedConnectivity::Error(_) => Connectivity::NotConnected,
            DetailedConnectivity::Uninitialized => Connectivity::NotConnected,
            DetailedConnectivity::Connecting => Connectivity::Connecting,
            DetailedConnectivity::Working => Connectivity::Connected,
            DetailedConnectivity::Connected => Connectivity::Connected,
        }
    }

    pub fn to_string_imap(&self, _context: &Context) -> String {
        match self {
            DetailedConnectivity::Error(e) => format!("🔴 Error: {}", e),
            DetailedConnectivity::Uninitialized => "🔴 Not started".to_string(),
            DetailedConnectivity::Connecting => "🟡 Connecting…".to_string(),
            DetailedConnectivity::Working => "⬇️ Getting new messages…".to_string(),
            DetailedConnectivity::Connected => "🟢 Connected".to_string(),
        }
    }

    pub fn to_string_smtp(&self, _context: &Context) -> String {
        match self {
            DetailedConnectivity::Error(e) => format!("🔴 Error: {}", e),
            DetailedConnectivity::Uninitialized => {
                "(You did not try to send a message recently)".to_string()
            }
            DetailedConnectivity::Connecting => "🟡 Connecting…".to_string(),
            DetailedConnectivity::Working => "⬆️ Sending…".to_string(),

            // We don't know any more than that the last message was sent successfully;
            // since sending the last message, connectivity could have changed, which we don't notice
            // until another message is sent
            DetailedConnectivity::Connected => {
                "🟢 Your last message was sent successfully".to_string()
            }
        }
    }
}

#[derive(Clone)]
pub struct ConnectivityStore(Arc<Mutex<DetailedConnectivity>>);

impl ConnectivityStore {
    pub(crate) fn new() -> Self {
        ConnectivityStore(Arc::new(Mutex::new(DetailedConnectivity::Uninitialized)))
    }

    pub(crate) async fn set(&self, context: &Context, v: DetailedConnectivity) {
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

    pub(crate) async fn get(&self) -> DetailedConnectivity {
        self.0.lock().await.deref().clone()
    }
    pub(crate) async fn get_basic(&self) -> Connectivity {
        self.0.lock().await.to_basic()
    }
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
    /// - DC_CONNECTIVITY_NOT_CONNECTED (0): Show e.g. the string "Not connected" or a red dot
    /// - DC_CONNECTIVITY_CONNECTING (1): Show e.g. the string "Connecting…" or a yellow dot
    /// - DC_CONNECTIVITY_CONNECTED (2): Show e.g. the string "Connected" or a green dot
    ///
    /// Meant as a rough overview that can be shown
    /// e.g. in the title of the main screen.
    ///
    /// If the connectivity changes, a DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
    pub async fn get_connectivity(&self) -> Connectivity {
        match &*self.scheduler.read().await {
            Scheduler::Running {
                inbox,
                mvbox,
                sentbox,
                ..
            } => {
                let states = [&inbox.state, &mvbox.state, &sentbox.state]; // TODO add smtp.state again
                let mut connectivities = Vec::new();
                for s in &states {
                    // TODO get_basic() locks a mutex, and above we called `scheduler.read()`. This means
                    // that we will be holding two locks, which sounds like a great opportunity for
                    // a deadlock.
                    // Below, I wrote another possible version of this code which first clones all the ConnectivityStore's
                    // (which are Arc's under the hood), then releases the scheduler-read-lock and only then
                    // calls `get_basic()`. Would this be better?

                    connectivities.push(s.connectivity.get_basic().await);
                }
                connectivities
                    .into_iter()
                    .min()
                    .unwrap_or(Connectivity::NotConnected)
            }
            Scheduler::Stopped => Connectivity::NotConnected,
        }
        // let mut stores = Vec::new();
        // match &*self.scheduler.read().await {
        //     Scheduler::Running {
        //         inbox,
        //         mvbox,
        //         sentbox,
        //         ..
        //     } => {
        //         for state in [&inbox.state, &mvbox.state, &sentbox.state].iter() {
        //             stores.push(state.connectivity.clone())
        //         }
        //     }
        //     Scheduler::Stopped => return BasicConnectivity::NotConnected,
        // }
        // let mut connectivities = Vec::new();
        // for store in stores {
        //     connectivities.push(store.get_basic().await);
        // }
        // connectivities
        //     .into_iter()
        //     .min()
        //     .unwrap_or(BasicConnectivity::NotConnected)
    }

    /// Get an overview over the current connectivity, and possibly more statistics.
    /// Meant to give the user more insight about the current status than
    /// the basic connectivity info returned by dc_get_connectivity(); show this
    /// e.g., if the user taps on said basic connectivity info.
    ///
    /// If this page changes, a DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
    ///
    /// This comes as an HTML from the core so that we can easily improve it
    /// and the improvement instantly reaches all UIs.
    pub async fn get_connectivity_html(&self) -> String {
        let mut ret =
            "<!DOCTYPE html>\n<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" /></head><body>\n".to_string();

        match &*self.scheduler.read().await {
            Scheduler::Running {
                inbox,
                mvbox,
                sentbox,
                smtp,
                ..
            } => {
                // TODO when merging https://github.com/deltachat/deltachat-core-rust/pull/2289/, there will be a duplicate of this
                // in resync_folders()

                let folders_states = &[
                    (
                        Config::ConfiguredInboxFolder,
                        Config::InboxWatch,
                        &inbox.state,
                    ),
                    (
                        Config::ConfiguredMvboxFolder,
                        Config::MvboxWatch,
                        &mvbox.state,
                    ),
                    (
                        Config::ConfiguredSentboxFolder,
                        Config::SentboxWatch,
                        &sentbox.state,
                    ),
                ];

                ret += "<div><h3>Incoming messages:</h3><ul>";
                for (folder, watch, state) in folders_states {
                    let w = self.get_config(*watch).await.ok_or_log(self);

                    if w.flatten() == Some("1".to_string()) {
                        let f = self.get_config(*folder).await.ok_or_log(self);

                        if let Some(foldername) = f.flatten() {
                            ret += "<li><b>&quot;";
                            ret += &foldername;
                            ret += "&quot;:</b> ";
                            ret += &state.connectivity.get().await.to_string_imap(self);
                            ret += "</li>";
                        }
                    }
                }
                ret += "</ul></div>";

                ret += "<h3>Outgoing messages:</h3><ul style=\"list-style-type: none;\"><li>";
                ret += &smtp.state.connectivity.get().await.to_string_smtp(self);
                ret += "</li></ul>";
            }
            Scheduler::Stopped => {}
        }

        ret += "</body></html>\n";
        ret
    }
}
