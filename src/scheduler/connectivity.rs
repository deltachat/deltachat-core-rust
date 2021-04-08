use core::fmt;
use std::{ops::Deref, sync::Arc};

use async_std::sync::Mutex;

use crate::events::EventType;
use crate::{config::Config, scheduler::Scheduler};
use crate::{context::Context, log::LogExt};

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty, PartialOrd, Ord, Display)]
pub enum Connectivity {
    Error(String),
    Connecting,
    Fetching,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, PartialOrd, Ord)]
// TODO maybe I come up with a better name than "basic"
pub enum BasicConnectivity {
    NotConnected = 0,
    Connecting = 1,
    Connected = 2,
}

impl Connectivity {
    pub fn to_basic(&self) -> BasicConnectivity {
        match self {
            Connectivity::Error(_) => BasicConnectivity::NotConnected,
            Connectivity::Connecting => BasicConnectivity::Connecting,
            Connectivity::Fetching => BasicConnectivity::Connected,
            Connectivity::Connected => BasicConnectivity::Connected,
        }
    }
}

#[derive(Clone)]
pub struct ConnectivityStore(Arc<Mutex<Connectivity>>);

impl ConnectivityStore {
    pub(crate) fn new() -> Self {
        ConnectivityStore(Arc::new(Mutex::new(Connectivity::Error(
            "Not started".to_string(),
        ))))
    }

    pub(crate) async fn set(&self, context: &Context, v: Connectivity) {
        {
            *self.0.lock().await = v;
        }
        context.emit_event(EventType::ConnectivityChanged);
    }

    pub(crate) async fn set_err(&self, context: &Context, e: impl ToString) {
        self.set(context, Connectivity::Error(e.to_string())).await;
    }
    pub(crate) async fn set_connecting(&self, context: &Context) {
        self.set(context, Connectivity::Connecting).await;
    }
    pub(crate) async fn set_fetching(&self, context: &Context) {
        self.set(context, Connectivity::Fetching).await;
    }
    pub(crate) async fn set_connected(&self, context: &Context) {
        self.set(context, Connectivity::Connected).await;
    }

    pub(crate) async fn get(&self) -> Connectivity {
        self.0.lock().await.deref().clone()
    }
    pub(crate) async fn get_basic(&self) -> BasicConnectivity {
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
    pub async fn get_connectivity(&self) -> BasicConnectivity {
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
                    .unwrap_or(BasicConnectivity::NotConnected)
            }
            Scheduler::Stopped => BasicConnectivity::NotConnected,
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
}
pub async fn get_connectivity_html(context: &Context) -> String {
    let mut ret =
            "<!DOCTYPE html>\n<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" /></head><body>\n".to_string();

    ret += &format!(
        "<h1>Connectivity: {:?}</h1>",
        context.get_connectivity().await
    );

    match &*context.scheduler.read().await {
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

            ret += "<ul>";
            for (folder, watch, state) in folders_states {
                let w = context.get_config(*watch).await.ok_or_log(context);

                if w.flatten() == Some("1".to_string()) {
                    let f = context.get_config(*folder).await.ok_or_log(context);

                    if let Some(foldername) = f.flatten() {
                        ret += "<li><b>Folder &quot;";
                        ret += &foldername;
                        ret += "&quot;:</b> ";
                        ret += &state.connectivity.get().await.to_string();
                        ret += "</li>";
                    }
                }
            }

            ret += "<li><b>Sending messages:</b> ";
            ret += &smtp.state.connectivity.get().await.to_string();
            ret += "</li>";

            ret += "</ul>";
        }
        Scheduler::Stopped => {}
    }

    ret += "</body></html>\n";
    ret
}
