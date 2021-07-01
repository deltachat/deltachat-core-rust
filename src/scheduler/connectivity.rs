use core::fmt;
use std::{ops::Deref, sync::Arc};

use async_std::sync::Mutex;

use crate::events::EventType;
use crate::{config::Config, scheduler::Scheduler};
use crate::{context::Context, log::LogExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, PartialOrd, Ord)]
pub enum Connectivity {
    NotConnected = 1000,
    Connecting = 2000,
    Working = 3000, // Fetching or sending messages
    InterruptingIdle = 4000,
    Connected = 5000,
}

// The order of the connectivities is important: worse connectivities (i.e. those at
// the top) take priority. This means that e.g. if any folder has an error - usually
// because there is no internet connection - the connectivity for the whole
// account will be `Notconnected`.
#[derive(Debug, Clone, PartialEq, Eq, EnumProperty)]
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

impl DetailedConnectivity {
    fn to_basic(&self) -> Option<Connectivity> {
        match self {
            DetailedConnectivity::Error(_) => Some(Connectivity::NotConnected),
            DetailedConnectivity::Uninitialized => Some(Connectivity::NotConnected),
            DetailedConnectivity::Connecting => Some(Connectivity::Connecting),
            DetailedConnectivity::Working => Some(Connectivity::Working),
            DetailedConnectivity::InterruptingIdle => Some(Connectivity::InterruptingIdle),
            DetailedConnectivity::Connected => Some(Connectivity::Connected),

            // Just don't return a connectivity, probably the folder is configured not to be
            // watched or there is e.g. no "Sent" folder, so we are not interested in it
            DetailedConnectivity::NotConfigured => None,
        }
    }

    fn to_string_imap(&self, _context: &Context) -> String {
        match self {
            DetailedConnectivity::Error(e) => format!("🔴 Error: {}", e),
            DetailedConnectivity::Uninitialized => "🔴 Not started".to_string(),
            DetailedConnectivity::Connecting => "🟡 Connecting…".to_string(),
            DetailedConnectivity::Working => "⬇️ Getting new messages…".to_string(),
            DetailedConnectivity::InterruptingIdle | DetailedConnectivity::Connected => {
                "🟢 Connected".to_string()
            }
            DetailedConnectivity::NotConfigured => "🔴 Not configured".to_string(),
        }
    }

    fn to_string_smtp(&self, _context: &Context) -> String {
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
            DetailedConnectivity::InterruptingIdle | DetailedConnectivity::Connected => {
                "🟢 Your last message was sent successfully".to_string()
            }
            DetailedConnectivity::NotConfigured => "🔴 Not configured".to_string(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ConnectivityStore(Arc<Mutex<DetailedConnectivity>>);

impl ConnectivityStore {
    pub(crate) fn new() -> Self {
        ConnectivityStore(Arc::new(Mutex::new(DetailedConnectivity::Uninitialized)))
    }

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
}

/// Set all folder states to InterruptingIdle in case they were `Connected` before.
/// Called during `dc_maybe_network()`.
pub(crate) async fn idle_interrupted(scheduler: &Scheduler) {
    if let Scheduler::Running {
        inbox,
        mvbox,
        sentbox,
        ..
    } = scheduler
    {
        let mut lock = inbox.state.connectivity.0.lock().await;
        // For the inbox, we also have to set the connectivity to InterruptingIdle if it was
        // NotConfigured before: If all folders are NotConfigured, dc_get_connectivity()
        // returns Connected. But after dc_maybe_network(), dc_get_connectivity() must not
        // return Connected until DC is completely done with fetching folders; this also
        // includes scan_folders() which happens on the inbox thread.
        if *lock == DetailedConnectivity::Connected || *lock == DetailedConnectivity::NotConfigured
        {
            *lock = DetailedConnectivity::InterruptingIdle;
        }
        drop(lock);

        for state in &[&mvbox.state, &sentbox.state] {
            let mut lock = state.connectivity.0.lock().await;
            if *lock == DetailedConnectivity::Connected {
                *lock = DetailedConnectivity::InterruptingIdle;
            }
        }
    }
    // We don't send a ConnectivityChanged event when setting the state to InterruptingIdle because
    // the connectivity didn't actually change. We only distinguish between `Connected` and
    // `InterruptingIdle` so that:
    // After calling dc_maybe_network(), when the connectivity is `Connected` again, the UI can be
    // sure that DC is done with fetching from all folders once.
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
    /// - DC_CONNECTIVITY_WORKING (3000-3999): Show e.g. the string "Getting new messages" or a spinning wheel
    /// - DC_CONNECTIVITY_INTERRUPTING_IDLE or DC_CONNECTIVITY_CONNECTED (>=4000): Show e.g. the string "Connected" or a green dot
    ///
    /// We don't use exact values but ranges here so that we can split up
    /// states into multiple states in the future.
    ///
    /// Meant as a rough overview that can be shown
    /// e.g. in the title of the main screen.
    ///
    /// Also, you can use this to find out when the core is completely done with fetching:
    /// - call dc_start_io() (in case IO was not running)
    /// - call dc_maybe_network()
    /// - wait until the connectivity is DC_CONNECTIVITY_CONNECTED (>=5000)
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
                let states = [&inbox.state, &mvbox.state, &sentbox.state];
                let mut connectivities = Vec::new();
                for s in &states {
                    // TODO/QUESTION get_basic() locks a mutex, and above we called `scheduler.read()`. This means
                    // that we will be holding two locks, which sounds like a great opportunity for
                    // a deadlock.
                    // Below (commented out, slightly outdated), I wrote another possible
                    // version of this code which first clones all the ConnectivityStore's
                    // (which are Arc's under the hood), then releases the scheduler-read-lock and only then
                    // calls `get_basic()`. Would this be better? Or don't I have to worry about deadlocks here at all?
                    // Same goes for get_connectivity_html().

                    if let Some(connectivity) = s.connectivity.get_basic().await {
                        connectivities.push(connectivity);
                    }
                }
                connectivities
                    .into_iter()
                    .min()
                    .unwrap_or(Connectivity::Connected)
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

                    let mut folder_added = false;
                    if w.flatten() == Some("1".to_string()) {
                        let f = self.get_config(*folder).await.ok_or_log(self).flatten();

                        if let Some(foldername) = f {
                            ret += "<li><b>&quot;";
                            ret += &foldername;
                            ret += "&quot;:</b> ";
                            ret += &state.connectivity.get_detailed().await.to_string_imap(self);
                            ret += "</li>";

                            folder_added = true;
                        }
                    }

                    if !folder_added && folder == &Config::ConfiguredInboxFolder {
                        let detailed = &state.connectivity.get_detailed().await;
                        if let DetailedConnectivity::Error(_) = detailed {
                            // On the inbox thread, we also do some other things like scan_folders and run jobs
                            // so, maybe, the inbox is not watched, but something else went wrong
                            ret += "<li>";
                            ret += &detailed.to_string_imap(self);
                            ret += "</li>";
                        }
                    }
                }
                ret += "</ul></div>";

                ret += "<h3>Outgoing messages:</h3><ul style=\"list-style-type: none;\"><li>";
                ret += &smtp
                    .state
                    .connectivity
                    .get_detailed()
                    .await
                    .to_string_smtp(self);
                ret += "</li></ul>";
            }
            Scheduler::Stopped => {}
        }

        ret += "</body></html>\n";
        ret
    }
}
