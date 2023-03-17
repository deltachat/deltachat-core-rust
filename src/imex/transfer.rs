//! Transfer a backup to an other device.
//!
//! This module provides support for using n0's iroh tool to initiate transfer of a backup
//! to another device using a QR code.
//!
//! Using the iroh terminology there are two parties to this:
//!
//! - The *Provider*, which starts a server and listens for connections.
//! - The *Getter*, which connects to the server and retrieves the data.
//!
//! Iroh is designed around the idea of verifying hashes, the downloads are verified as
//! they are retrieved.  The entire transfer is initiated by requesting the data of a single
//! root hash.
//!
//! Both the provider and the getter are authenticated:
//!
//! - The provider is known by its *peer ID*.
//! - The provider needs an *authentication token* from the getter before it accepts a
//!   connection.
//!
//! Both these are transferred in the QR code offered to the getter.  This ensures that the
//! getter can not connect to an impersonated provider and the provider does not offer the
//! download to an impersonated getter.

use std::future::Future;
use std::net::Ipv4Addr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::Poll;

use anyhow::{anyhow, bail, ensure, format_err, Context as _, Result};
use async_channel::Receiver;
use futures_lite::StreamExt;
use iroh::get::{DataStream, Options};
use iroh::progress::ProgressEmitter;
use iroh::protocol::AuthToken;
use iroh::provider::{DataSource, Event, Provider, Ticket};
use iroh::Hash;
use tokio::fs::{self, File};
use tokio::io::{self, AsyncWriteExt, BufWriter};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, Mutex};
use tokio::task::{JoinHandle, JoinSet};
use tokio_stream::wrappers::ReadDirStream;

use crate::blob::BlobDirContents;
use crate::chat::delete_and_reset_all_device_msgs;
use crate::context::Context;
use crate::qr::Qr;
use crate::{e2ee, EventType};

use super::{export_database, DBFILE_BACKUP_NAME};

/// Provide or send a backup of this device.
///
/// This creates a backup of the current device and starts a service which offers another
/// device to download this backup.
///
/// This does not make a full backup on disk, only the SQLite database is created on disk,
/// the blobs in the blob directory are not copied.
///
/// This starts a task which acquires the global "ongoing" mutex.  If you need to stop the
/// task use the [`Context::stop_ongoing`] mechanism.
///
/// The task implements [`Future`] and awaiting it will complete once a transfer has been
/// either completed or aborted.
#[derive(Debug)]
pub struct BackupProvider {
    /// The supervisor task, run by [`BackupProvider::watch_provider`].
    handle: JoinHandle<Result<()>>,
    /// The ticket to retrieve the backup collection.
    ticket: Ticket,
}

impl BackupProvider {
    /// Prepares for sending a backup to a second device.
    ///
    /// Before calling this function all I/O must be stopped so that no changes to the blobs
    /// or database are happening, this is done by calling the [`Accounts::stop_io`] or
    /// [`Context::stop_io`] APIs first.
    ///
    /// This will acquire the global "ongoing process" mutex, which can be used to cancel
    /// the process.
    ///
    /// [`Accounts::stop_io`]: crate::accounts::Accounts::stop_io
    pub async fn prepare(context: &Context) -> Result<Self> {
        e2ee::ensure_secret_key_exists(context)
            .await
            .context("Private key not available, aborting backup export")?;

        // Acquire global "ongoing" mutex.
        let cancel_token = context.alloc_ongoing().await?;
        let context_dir = context
            .get_blobdir()
            .parent()
            .ok_or(anyhow!("Context dir not found"))?;
        let dbfile = context_dir.join(DBFILE_BACKUP_NAME);
        if fs::metadata(&dbfile).await.is_ok() {
            fs::remove_file(&dbfile).await?;
            warn!(context, "Previous database export deleted");
        }
        let dbfile = TempPathGuard::new(dbfile);
        let res = tokio::select! {
            biased;
            res = Self::prepare_inner(context, &dbfile) => {
                match res {
                    Ok(slf) => Ok(slf),
                    Err(err) => {
                        error!(context, "Failed to set up second device setup: {:#}", err);
                        Err(err)
                    },
                }
            },
            _ = cancel_token.recv() => Err(format_err!("cancelled")),
        };
        let (provider, ticket) = match res {
            Ok((provider, ticket)) => (provider, ticket),
            Err(err) => {
                context.free_ongoing().await;
                return Err(err);
            }
        };
        let handle = tokio::spawn(Self::watch_provider(
            context.clone(),
            provider,
            cancel_token,
            dbfile,
        ));
        let slf = Self { handle, ticket };
        let qr = slf.qr();
        *context.export_provider.lock().expect("poisoned lock") = Some(qr);
        Ok(slf)
    }

    /// Creates the provider task.
    ///
    /// Having this as a function makes it easier to cancel it when needed.
    async fn prepare_inner(context: &Context, dbfile: &Path) -> Result<(Provider, Ticket)> {
        // Generate the token up front: we also use it to encrypt the database.
        let token = AuthToken::generate();
        context.emit_event(SendProgress::Started.into());
        export_database(context, dbfile, token.to_string())
            .await
            .context("Database export failed")?;
        context.emit_event(SendProgress::DatabaseExported.into());

        // Now we can be sure IO is not running.
        let mut files = vec![DataSource::with_name(
            dbfile.to_owned(),
            format!("db/{DBFILE_BACKUP_NAME}"),
        )];
        let blobdir = BlobDirContents::new(context).await?;
        for blob in blobdir.iter() {
            let path = blob.to_abs_path();
            let name = format!("blob/{}", blob.as_file_name());
            files.push(DataSource::with_name(path, name));
        }

        // Start listening.
        let (db, hash) = iroh::provider::create_collection(files).await?;
        context.emit_event(SendProgress::CollectionCreated.into());
        let provider = Provider::builder(db)
            .bind_addr((Ipv4Addr::UNSPECIFIED, 0).into())
            .auth_token(token)
            .spawn()?;
        context.emit_event(SendProgress::ProviderListening.into());
        info!(context, "Waiting for remote to connect");
        let ticket = provider.ticket(hash);
        Ok((provider, ticket))
    }

    /// Supervises the iroh [`Provider`], terminating it when needed.
    ///
    /// This will watch the provider and terminate it when:
    ///
    /// - A transfer is completed, successful or unsuccessful.
    /// - An event could not be observed to protect against not knowing of a completed event.
    /// - The ongoing process is cancelled.
    ///
    /// The *cancel_token* is the handle for the ongoing process mutex, when this completes
    /// we must cancel this operation.
    async fn watch_provider(
        context: Context,
        mut provider: Provider,
        cancel_token: Receiver<()>,
        _dbfile: TempPathGuard,
    ) -> Result<()> {
        // _dbfile exists so we can clean up the file once it is no longer needed
        let mut events = provider.subscribe();
        let mut total_size = 0;
        let mut current_size = 0;
        let res = loop {
            tokio::select! {
                biased;
                res = &mut provider => {
                    break res.context("BackupProvider failed");
                },
                maybe_event = events.recv() => {
                    match maybe_event {
                        Ok(event) => {
                            match event {
                                Event::ClientConnected { ..} => {
                                    context.emit_event(SendProgress::ClientConnected.into());
                                }
                                Event::RequestReceived { .. } => {
                                }
                                Event::TransferCollectionStarted { total_blobs_size, .. } => {
                                    total_size = total_blobs_size;
                                    context.emit_event(SendProgress::TransferInProgress {
                                        current_size,
                                        total_size,
                                    }.into());
                                }
                                Event::TransferBlobCompleted { size, .. } => {
                                    current_size += size;
                                    context.emit_event(SendProgress::TransferInProgress {
                                        current_size,
                                        total_size,
                                    }.into());
                                }
                                Event::TransferCollectionCompleted { .. } => {
                                    context.emit_event(SendProgress::TransferInProgress {
                                        current_size: total_size,
                                        total_size
                                    }.into());
                                    provider.shutdown();
                                }
                                Event::TransferAborted { .. } => {
                                    provider.shutdown();
                                    break Err(anyhow!("BackupProvider transfer aborted"));
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // We should never see this, provider.join() should complete
                            // first.
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // We really shouldn't be lagging, if we did we may have missed
                            // a completion event.
                            provider.shutdown();
                            break Err(anyhow!("Missed events from BackupProvider"));
                        }
                    }
                },
                _ = cancel_token.recv() => {
                    provider.shutdown();
                    break Err(anyhow!("BackupSender cancelled"));
                },
            }
        };
        context
            .export_provider
            .lock()
            .expect("poisoned lock")
            .take();
        match &res {
            Ok(_) => context.emit_event(SendProgress::Completed.into()),
            Err(err) => {
                error!(context, "Backup transfer failure: {err:#}");
                context.emit_event(SendProgress::Failed.into())
            }
        }
        context.free_ongoing().await;
        res
    }

    /// Returns a QR code that allows fetching this backup.
    ///
    /// This QR code can be passed to [`get_backup`] on a (different) device.
    pub fn qr(&self) -> Qr {
        Qr::Backup {
            ticket: self.ticket.clone(),
        }
    }
}

impl Future for BackupProvider {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)?
    }
}

/// A guard which will remove the path when dropped.
///
/// It implements [`Deref`] it it can be used as a `&Path`.
#[derive(Debug)]
struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let path = self.path.clone();
        tokio::spawn(async move {
            fs::remove_file(&path).await.ok();
        });
    }
}

impl Deref for TempPathGuard {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

/// Create [`EventType::ImexProgress`] events using readable names.
///
/// Plus you get warnings if you don't use all variants.
#[derive(Debug)]
enum SendProgress {
    Failed,
    Started,
    DatabaseExported,
    CollectionCreated,
    ProviderListening,
    ClientConnected,
    TransferInProgress { current_size: u64, total_size: u64 },
    Completed,
}

impl From<SendProgress> for EventType {
    fn from(source: SendProgress) -> Self {
        use SendProgress::*;
        let num: u16 = match source {
            Failed => 0,
            Started => 100,
            DatabaseExported => 300,
            CollectionCreated => 350,
            ProviderListening => 400,
            ClientConnected => 450,
            TransferInProgress {
                current_size,
                total_size,
            } => {
                // the range is 450..=950
                450 + ((current_size as f64 / total_size as f64) * 500.).floor() as u16
            }
            Completed => 1000,
        };
        Self::ImexProgress(num.into())
    }
}

/// Contacts a backup provider and receives the backup from it.
///
/// This uses a QR code to contact another instance of deltachat which is providing a backup
/// using the [`BackupProvider`].  Once connected it will authenticate using the secrets in
/// the QR code and retrieve the backup.
///
/// This is a long running operation which will only when completed.
///
/// Using [`Qr`] as argument is a bit odd as it only accepts one specific variant of it.  It
/// does avoid having [`iroh::provider::Ticket`] in the primary API however, without
/// having to revert to untyped bytes.
pub async fn get_backup(context: &Context, qr: Qr) -> Result<()> {
    ensure!(
        matches!(qr, Qr::Backup { .. }),
        "QR code for backup must be of type DCBACKUP"
    );
    ensure!(
        !context.is_configured().await?,
        "Cannot import backups to accounts in use."
    );
    ensure!(
        context.scheduler.read().await.is_none(),
        "cannot import backup, IO is running"
    );

    // Acquire global "ongoing" mutex.
    let cancel_token = context.alloc_ongoing().await?;
    tokio::select! {
        biased;
        res = get_backup_inner(context, qr) => {
            context.free_ongoing().await;
            res
        }
        _ = cancel_token.recv() => Err(format_err!("cancelled")),
    }
}

async fn get_backup_inner(context: &Context, qr: Qr) -> Result<()> {
    let ticket = match qr {
        Qr::Backup { ticket } => ticket,
        _ => bail!("QR code for backup must be of type DCBACKUP"),
    };
    if ticket.addrs.is_empty() {
        bail!("ticket is missing addresses to dial");
    }
    for addr in &ticket.addrs {
        let opts = Options {
            addr: *addr,
            peer_id: Some(ticket.peer),
            keylog: false,
        };
        info!(context, "attempting to contact {}", addr);
        match transfer_from_provider(context, &ticket, opts).await {
            Ok(_) => {
                delete_and_reset_all_device_msgs(context).await?;
                context.emit_event(ReceiveProgress::Completed.into());
                return Ok(());
            }
            Err(TransferError::ConnectionError(err)) => {
                warn!(context, "Connection error: {err:#}.");
                continue;
            }
            Err(TransferError::Other(err)) => {
                // Clean up any blobs we already wrote.
                let readdir = fs::read_dir(context.get_blobdir()).await?;
                let mut readdir = ReadDirStream::new(readdir);
                while let Some(dirent) = readdir.next().await {
                    if let Ok(dirent) = dirent {
                        fs::remove_file(dirent.path()).await.ok();
                    }
                }
                context.emit_event(ReceiveProgress::Failed.into());
                return Err(err);
            }
        }
    }
    Err(anyhow!("failed to contact provider"))
}

/// Error during a single transfer attempt.
///
/// Mostly exists to distinguish between `ConnectionError` and any other errors.
#[derive(Debug, thiserror::Error)]
enum TransferError {
    #[error("connection error")]
    ConnectionError(#[source] anyhow::Error),
    #[error("other")]
    Other(#[source] anyhow::Error),
}

async fn transfer_from_provider(
    context: &Context,
    ticket: &Ticket,
    opts: Options,
) -> Result<(), TransferError> {
    let progress = ProgressEmitter::new(0, ReceiveProgress::max_blob_progress());
    spawn_progress_proxy(context.clone(), progress.subscribe());
    let mut connected = false;
    let on_connected = || {
        context.emit_event(ReceiveProgress::Connected.into());
        connected = true;
        async { Ok(()) }
    };
    let jobs = Mutex::new(JoinSet::default());
    let on_blob =
        |hash, reader, name| on_blob(context, &progress, &jobs, ticket, hash, reader, name);
    let res = iroh::get::run(
        ticket.hash,
        ticket.token,
        opts,
        on_connected,
        |collection| {
            context.emit_event(ReceiveProgress::CollectionRecieved.into());
            progress.set_total(collection.total_blobs_size());
            async { Ok(()) }
        },
        on_blob,
    )
    .await;

    let mut jobs = jobs.lock().await;
    while let Some(job) = jobs.join_next().await {
        job.context("job failed").map_err(TransferError::Other)?;
    }

    drop(progress);
    match res {
        Ok(stats) => {
            info!(
                context,
                "Backup transfer finished, transfer rate is {} Mbps.",
                stats.mbits()
            );
            Ok(())
        }
        Err(err) => match connected {
            true => Err(TransferError::Other(err)),
            false => Err(TransferError::ConnectionError(err)),
        },
    }
}

/// Get callback when a blob is received from the provider.
///
/// This writes the blobs to the blobdir.  If the blob is the database it will import it to
/// the database of the current [`Context`].
async fn on_blob(
    context: &Context,
    progress: &ProgressEmitter,
    jobs: &Mutex<JoinSet<()>>,
    ticket: &Ticket,
    _hash: Hash,
    mut reader: DataStream,
    name: String,
) -> Result<DataStream> {
    ensure!(!name.is_empty(), "Received a nameless blob");
    let path = if name.starts_with("db/") {
        let context_dir = context
            .get_blobdir()
            .parent()
            .ok_or(anyhow!("Context dir not found"))?;
        let dbfile = context_dir.join(DBFILE_BACKUP_NAME);
        if fs::metadata(&dbfile).await.is_ok() {
            fs::remove_file(&dbfile).await?;
            warn!(context, "Previous database export deleted");
        }
        dbfile
    } else {
        ensure!(name.starts_with("blob/"), "malformatted blob name");
        let blobname = name.rsplit('/').next().context("malformatted blob name")?;
        context.get_blobdir().join(blobname)
    };

    let mut wrapped_reader = progress.wrap_async_read(&mut reader);
    let file = File::create(&path).await?;
    let mut file = BufWriter::with_capacity(128 * 1024, file);
    io::copy(&mut wrapped_reader, &mut file).await?;
    file.flush().await?;

    if name.starts_with("db/") {
        let context = context.clone();
        let token = ticket.token.to_string();
        jobs.lock().await.spawn(async move {
            if let Err(err) = context.sql.import(&path, token).await {
                error!(context, "cannot import database: {:#?}", err);
            }
            if let Err(err) = fs::remove_file(&path).await {
                error!(
                    context,
                    "failed to delete database import file '{}': {:#?}",
                    path.display(),
                    err,
                );
            }
        });
    }
    Ok(reader)
}

/// Spawns a task proxying progress events.
///
/// This spawns a tokio task which receives events from the [`ProgressEmitter`] and sends
/// them to the context.  The task finishes when the emitter is dropped.
///
/// This could be done directly in the emitter by making it less generic.
fn spawn_progress_proxy(context: Context, mut rx: broadcast::Receiver<u16>) {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(step) => context.emit_event(ReceiveProgress::BlobProgress(step).into()),
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(_)) => continue,
            }
        }
    });
}

/// Create [`EventType::ImexProgress`] events using readable names.
///
/// Plus you get warnings if you don't use all variants.
#[derive(Debug)]
enum ReceiveProgress {
    Connected,
    CollectionRecieved,
    /// A value between 0 and 85 interpreted as a percentage.
    ///
    /// Other values are already used by the other variants of this enum.
    BlobProgress(u16),
    Completed,
    Failed,
}

impl ReceiveProgress {
    /// The maximum value for [`ReceiveProgress::BlobProgress`].
    ///
    /// This only exists to keep this magic value local in this type.
    fn max_blob_progress() -> u16 {
        85
    }
}

impl From<ReceiveProgress> for EventType {
    fn from(source: ReceiveProgress) -> Self {
        let val = match source {
            ReceiveProgress::Connected => 50,
            ReceiveProgress::CollectionRecieved => 100,
            ReceiveProgress::BlobProgress(val) => 100 + 10 * val,
            ReceiveProgress::Completed => 1000,
            ReceiveProgress::Failed => 0,
        };
        EventType::ImexProgress(val.into())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::chat::{get_chat_msgs, send_msg, ChatItem};
    use crate::message::{Message, Viewtype};
    use crate::test_utils::TestContextManager;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_receive() {
        let mut tcm = TestContextManager::new();

        // Create first device.
        let ctx0 = tcm.alice().await;

        // Write a message in the self chat
        let self_chat = ctx0.get_self_chat().await;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hi there".to_string()));
        send_msg(&ctx0, self_chat.id, &mut msg).await.unwrap();

        // Send an attachment in the self chat
        let file = ctx0.get_blobdir().join("hello.txt");
        fs::write(&file, "i am attachment").await.unwrap();
        let mut msg = Message::new(Viewtype::File);
        msg.set_file(file.to_str().unwrap(), Some("text/plain"));
        send_msg(&ctx0, self_chat.id, &mut msg).await.unwrap();

        // Prepare to transfer backup.
        let provider = BackupProvider::prepare(&ctx0).await.unwrap();

        // Set up second device.
        let ctx1 = tcm.unconfigured().await;
        get_backup(&ctx1, provider.qr()).await.unwrap();

        // Make sure the provider finishes without an error.
        tokio::time::timeout(Duration::from_secs(30), provider)
            .await
            .expect("timed out")
            .expect("error in provider");

        // Check that we have the self message.
        let self_chat = ctx1.get_self_chat().await;
        let msgs = get_chat_msgs(&ctx1, self_chat.id).await.unwrap();
        assert_eq!(msgs.len(), 2);
        let msgid = match msgs.get(0).unwrap() {
            ChatItem::Message { msg_id } => msg_id,
            _ => panic!("wrong chat item"),
        };
        let msg = Message::load_from_db(&ctx1, *msgid).await.unwrap();
        let text = msg.get_text().unwrap();
        assert_eq!(text, "hi there");
        let msgid = match msgs.get(1).unwrap() {
            ChatItem::Message { msg_id } => msg_id,
            _ => panic!("wrong chat item"),
        };
        let msg = Message::load_from_db(&ctx1, *msgid).await.unwrap();
        let path = msg.get_file(&ctx1).unwrap();
        let text = fs::read_to_string(&path).await.unwrap();
        assert_eq!(text, "i am attachment");

        // Check that both received the ImexProgress events.
        ctx0.evtracker
            .get_matching(|ev| matches!(ev, EventType::ImexProgress(1000)))
            .await;
        ctx1.evtracker
            .get_matching(|ev| matches!(ev, EventType::ImexProgress(1000)))
            .await;
    }

    #[test]
    fn test_send_progress() {
        let cases = [
            ((0, 100), 450),
            ((10, 100), 500),
            ((50, 100), 700),
            ((100, 100), 950),
        ];

        for ((current_size, total_size), progress) in cases {
            let out = EventType::from(SendProgress::TransferInProgress {
                current_size,
                total_size,
            });
            assert_eq!(out, EventType::ImexProgress(progress));
        }
    }
}
