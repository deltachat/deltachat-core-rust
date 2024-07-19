//! Transfer a backup to an other device.
//!
//! This module provides support for using [iroh](https://iroh.computer/)
//! to initiate transfer of a backup to another device using a QR code.
//!
//! There are two parties to this:
//! - The *Provider*, which starts a server and listens for connections.
//! - The *Getter*, which connects to the server and retrieves the data.
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
//!
//! Protocol starts by getter opening a bidirectional QUIC stream
//! to the provider and sending authentication token.
//! Provider verifies received authentication token,
//! sends the size of all files in a backup (database and all blobs)
//! as an unsigned 64-bit big endian integer and streams the backup in tar format.
//! Getter receives the backup and acknowledges successful reception
//! by sending a single byte.
//! Provider closes the endpoint after receiving an acknowledgment.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use anyhow::{anyhow, bail, ensure, format_err, Context as _, Result};
use futures_lite::StreamExt;
use iroh_net::relay::RelayMode;
use iroh_net::Endpoint;
use iroh_old;
use iroh_old::blobs::Collection;
use iroh_old::get::DataStream;
use iroh_old::progress::ProgressEmitter;
use iroh_old::provider::Ticket;
use tokio::fs::{self, File};
use tokio::io::{self, AsyncWriteExt, BufWriter};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, Mutex};
use tokio::task::{JoinHandle, JoinSet};
use tokio_stream::wrappers::ReadDirStream;
use tokio_util::sync::CancellationToken;

use crate::chat::{add_device_msg, delete_and_reset_all_device_msgs};
use crate::context::Context;
use crate::imex::BlobDirContents;
use crate::message::{Message, Viewtype};
use crate::qr::{self, Qr};
use crate::stock_str::backup_transfer_msg_body;
use crate::tools::{create_id, time, TempPathGuard};
use crate::EventType;

use super::{export_backup_stream, export_database, import_backup_stream, DBFILE_BACKUP_NAME};

const MAX_CONCURRENT_DIALS: u8 = 16;

/// ALPN protocol identifier for the backup transfer protocol.
const BACKUP_ALPN: &[u8] = b"/deltachat/backup";

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
#[derive(Debug)]
pub struct BackupProvider {
    /// iroh-net endpoint.
    _endpoint: Endpoint,

    /// iroh-net address.
    node_addr: iroh_net::NodeAddr,

    /// Authentication token that should be submitted
    /// to retrieve the backup.
    auth_token: String,

    /// Handle for the task accepting backup transfer requests.
    handle: JoinHandle<Result<()>>,

    /// Guard to cancel the provider on drop.
    _drop_guard: tokio_util::sync::DropGuard,
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
        let relay_mode = RelayMode::Disabled;
        let endpoint = Endpoint::builder()
            .alpns(vec![BACKUP_ALPN.to_vec()])
            .relay_mode(relay_mode)
            .bind(0)
            .await?;
        let node_addr = endpoint.node_addr().await?;

        // Acquire global "ongoing" mutex.
        let cancel_token = context.alloc_ongoing().await?;
        let paused_guard = context.scheduler.pause(context.clone()).await?;
        let context_dir = context
            .get_blobdir()
            .parent()
            .context("Context dir not found")?;
        let dbfile = context_dir.join(DBFILE_BACKUP_NAME);
        if fs::metadata(&dbfile).await.is_ok() {
            fs::remove_file(&dbfile).await?;
            warn!(context, "Previous database export deleted");
        }
        let dbfile = TempPathGuard::new(dbfile);

        // Authentication token that receiver should send us to receive a backup.
        let auth_token = create_id();

        let passphrase = String::new();

        export_database(context, &dbfile, passphrase, time())
            .await
            .context("Database export failed")?;
        context.emit_event(EventType::ImexProgress(300));

        let drop_token = CancellationToken::new();
        let handle = {
            let context = context.clone();
            let drop_token = drop_token.clone();
            let endpoint = endpoint.clone();
            let auth_token = auth_token.clone();
            tokio::spawn(async move {
                Self::accept_loop(
                    context.clone(),
                    endpoint,
                    auth_token,
                    cancel_token,
                    drop_token,
                    dbfile,
                )
                .await;
                info!(context, "Finished accept loop.");

                context.free_ongoing().await;

                // Explicit drop to move the guards into this future
                drop(paused_guard);
                Ok(())
            })
        };
        Ok(Self {
            _endpoint: endpoint,
            node_addr,
            auth_token,
            handle,
            _drop_guard: drop_token.drop_guard(),
        })
    }

    async fn handle_connection(
        context: Context,
        conn: iroh_net::endpoint::Connecting,
        auth_token: String,
        dbfile: Arc<TempPathGuard>,
    ) -> Result<()> {
        let conn = conn.await?;
        let (mut send_stream, mut recv_stream) = conn.accept_bi().await?;

        // Read authentication token from the stream.
        let mut received_auth_token = vec![0u8; auth_token.len()];
        recv_stream.read_exact(&mut received_auth_token).await?;
        if received_auth_token.as_slice() != auth_token.as_bytes() {
            warn!(context, "Received wrong backup authentication token.");
            return Ok(());
        }

        info!(context, "Received valid backup authentication token.");

        let blobdir = BlobDirContents::new(&context).await?;

        let mut file_size = 0;
        file_size += dbfile.metadata()?.len();
        for blob in blobdir.iter() {
            file_size += blob.to_abs_path().metadata()?.len()
        }

        send_stream.write_all(&file_size.to_be_bytes()).await?;

        export_backup_stream(&context, &dbfile, blobdir, send_stream)
            .await
            .context("Failed to write backup into QUIC stream")?;
        info!(context, "Finished writing backup into QUIC stream.");
        let mut buf = [0u8; 1];
        info!(context, "Waiting for acknowledgment.");
        recv_stream.read_exact(&mut buf).await?;
        info!(context, "Received backup reception acknowledgement.");
        context.emit_event(EventType::ImexProgress(1000));

        let mut msg = Message::new(Viewtype::Text);
        msg.text = backup_transfer_msg_body(&context).await;
        add_device_msg(&context, None, Some(&mut msg)).await?;

        Ok(())
    }

    async fn accept_loop(
        context: Context,
        endpoint: Endpoint,
        auth_token: String,
        cancel_token: async_channel::Receiver<()>,
        drop_token: CancellationToken,
        dbfile: TempPathGuard,
    ) {
        let dbfile = Arc::new(dbfile);
        loop {
            tokio::select! {
                biased;

                conn = endpoint.accept() => {
                    if let Some(conn) = conn {
                        // Got a new in-progress connection.
                        let context = context.clone();
                        let auth_token = auth_token.clone();
                        let dbfile = dbfile.clone();
                        if let Err(err) = Self::handle_connection(context.clone(), conn, auth_token, dbfile).await {
                            warn!(context, "Error while handling backup connection: {err:#}.");
                        } else {
                            info!(context, "Backup transfer finished successfully.");
                            break;
                        }
                    } else {
                        break;
                    }
                },
                _ = cancel_token.recv() => {
                    context.emit_event(EventType::ImexProgress(0));
                    break;
                }
                _ = drop_token.cancelled() => {
                    context.emit_event(EventType::ImexProgress(0));
                    break;
                }
            }
        }
    }

    /// Returns a QR code that allows fetching this backup.
    ///
    /// This QR code can be passed to [`get_backup`] on a (different) device.
    pub fn qr(&self) -> Qr {
        Qr::Backup2 {
            node_addr: self.node_addr.clone(),

            auth_token: self.auth_token.clone(),
        }
    }
}

impl Future for BackupProvider {
    type Output = Result<()>;

    /// Waits for the backup transfer to complete.
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)?
    }
}

/// Retrieves backup from a legacy backup provider using iroh 0.4.
pub async fn get_legacy_backup(context: &Context, qr: Qr) -> Result<()> {
    ensure!(
        matches!(qr, Qr::Backup { .. }),
        "QR code for backup must be of type DCBACKUP"
    );
    ensure!(
        !context.is_configured().await?,
        "Cannot import backups to accounts in use."
    );
    // Acquire global "ongoing" mutex.
    let cancel_token = context.alloc_ongoing().await?;
    let _guard = context.scheduler.pause(context.clone()).await;
    info!(
        context,
        "Running get_backup for {}",
        qr::format_backup(&qr)?
    );
    let res = tokio::select! {
        biased;
        res = get_backup_inner(context, qr) => res,
        _ = cancel_token.recv() => Err(format_err!("cancelled")),
    };
    context.free_ongoing().await;
    res
}

pub async fn get_backup2(
    context: &Context,
    node_addr: iroh_net::NodeAddr,
    auth_token: String,
) -> Result<()> {
    let relay_mode = RelayMode::Disabled;

    let endpoint = Endpoint::builder().relay_mode(relay_mode).bind(0).await?;

    let conn = endpoint.connect(node_addr, BACKUP_ALPN).await?;
    let (mut send_stream, mut recv_stream) = conn.open_bi().await?;
    info!(context, "Sending backup authentication token.");
    send_stream.write_all(auth_token.as_bytes()).await?;

    let passphrase = String::new();
    info!(context, "Starting to read backup from the stream.");

    let mut file_size_buf = [0u8; 8];
    recv_stream.read_exact(&mut file_size_buf).await?;
    let file_size = u64::from_be_bytes(file_size_buf);
    import_backup_stream(context, recv_stream, file_size, passphrase)
        .await
        .context("Failed to import backup from QUIC stream")?;
    info!(context, "Finished importing backup from the stream.");
    context.emit_event(EventType::ImexProgress(1000));

    // Send an acknowledgement, but ignore the errors.
    // We have imported backup successfully already.
    send_stream.write_all(b".").await.ok();
    send_stream.finish().await.ok();
    info!(context, "Sent backup reception acknowledgment.");

    Ok(())
}

/// Contacts a backup provider and receives the backup from it.
///
/// This uses a QR code to contact another instance of deltachat which is providing a backup
/// using the [`BackupProvider`].  Once connected it will authenticate using the secrets in
/// the QR code and retrieve the backup.
///
/// This is a long running operation which will return only when completed.
///
/// Using [`Qr`] as argument is a bit odd as it only accepts specific variants of it.  It
/// does avoid having [`iroh_old::provider::Ticket`] in the primary API however, without
/// having to revert to untyped bytes.
pub async fn get_backup(context: &Context, qr: Qr) -> Result<()> {
    match qr {
        Qr::Backup { .. } => get_legacy_backup(context, qr).await?,
        Qr::Backup2 {
            node_addr,
            auth_token,
        } => get_backup2(context, node_addr, auth_token).await?,
        _ => bail!("QR code for backup must be of type DCBACKUP or DCBACKUP2"),
    }
    Ok(())
}

async fn get_backup_inner(context: &Context, qr: Qr) -> Result<()> {
    let ticket = match qr {
        Qr::Backup { ticket } => ticket,
        _ => bail!("QR code for backup must be of type DCBACKUP"),
    };

    match transfer_from_provider(context, &ticket).await {
        Ok(()) => {
            context.sql.run_migrations(context).await?;
            delete_and_reset_all_device_msgs(context).await?;
            context.emit_event(ReceiveProgress::Completed.into());
            Ok(())
        }
        Err(err) => {
            // Clean up any blobs we already wrote.
            let readdir = fs::read_dir(context.get_blobdir()).await?;
            let mut readdir = ReadDirStream::new(readdir);
            while let Some(dirent) = readdir.next().await {
                if let Ok(dirent) = dirent {
                    fs::remove_file(dirent.path()).await.ok();
                }
            }
            context.emit_event(ReceiveProgress::Failed.into());
            Err(err)
        }
    }
}

async fn transfer_from_provider(context: &Context, ticket: &Ticket) -> Result<()> {
    let progress = ProgressEmitter::new(0, ReceiveProgress::max_blob_progress());
    spawn_progress_proxy(context.clone(), progress.subscribe());
    let on_connected = || {
        context.emit_event(ReceiveProgress::Connected.into());
        async { Ok(()) }
    };
    let on_collection = |collection: &Collection| {
        context.emit_event(ReceiveProgress::CollectionReceived.into());
        progress.set_total(collection.total_blobs_size());
        async { Ok(()) }
    };
    let jobs = Mutex::new(JoinSet::default());
    let on_blob =
        |hash, reader, name| on_blob(context, &progress, &jobs, ticket, hash, reader, name);

    // Perform the transfer.
    let keylog = false; // Do not enable rustls SSLKEYLOGFILE env var functionality
    let stats = iroh_old::get::run_ticket(
        ticket,
        keylog,
        MAX_CONCURRENT_DIALS,
        on_connected,
        on_collection,
        on_blob,
    )
    .await?;

    let mut jobs = jobs.lock().await;
    while let Some(job) = jobs.join_next().await {
        job.context("job failed")?;
    }
    drop(progress);
    info!(
        context,
        "Backup transfer finished, transfer rate was {} Mbps.",
        stats.mbits()
    );
    Ok(())
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
    _hash: iroh_old::Hash,
    mut reader: DataStream,
    name: String,
) -> Result<DataStream> {
    ensure!(!name.is_empty(), "Received a nameless blob");
    let path = if name.starts_with("db/") {
        let context_dir = context
            .get_blobdir()
            .parent()
            .ok_or_else(|| anyhow!("Context dir not found"))?;
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
        let token = ticket.token().to_string();
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
    CollectionReceived,
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
            ReceiveProgress::CollectionReceived => 100,
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
        msg.set_text("hi there".to_string());
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
        let msgid = match msgs.first().unwrap() {
            ChatItem::Message { msg_id } => msg_id,
            _ => panic!("wrong chat item"),
        };
        let msg = Message::load_from_db(&ctx1, *msgid).await.unwrap();
        let text = msg.get_text();
        assert_eq!(text, "hi there");
        let msgid = match msgs.get(1).unwrap() {
            ChatItem::Message { msg_id } => msg_id,
            _ => panic!("wrong chat item"),
        };
        let msg = Message::load_from_db(&ctx1, *msgid).await.unwrap();

        let path = msg.get_file(&ctx1).unwrap();
        assert_eq!(path.with_file_name("hello.txt"), path);
        let text = fs::read_to_string(&path).await.unwrap();
        assert_eq!(text, "i am attachment");

        let path = path.with_file_name("saved.txt");
        msg.save_file(&ctx1, &path).await.unwrap();
        let text = fs::read_to_string(&path).await.unwrap();
        assert_eq!(text, "i am attachment");
        assert!(msg.save_file(&ctx1, &path).await.is_err());

        // Check that both received the ImexProgress events.
        ctx0.evtracker
            .get_matching(|ev| matches!(ev, EventType::ImexProgress(1000)))
            .await;
        ctx1.evtracker
            .get_matching(|ev| matches!(ev, EventType::ImexProgress(1000)))
            .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_drop_provider() {
        let mut tcm = TestContextManager::new();
        let ctx = tcm.alice().await;

        let provider = BackupProvider::prepare(&ctx).await.unwrap();
        drop(provider);
        ctx.evtracker
            .get_matching(|ev| matches!(ev, EventType::ImexProgress(0)))
            .await;
    }
}
