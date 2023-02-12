use std::env;
///! Delta Chat core RPC server.
///!
///! It speaks JSON Lines over stdio.
use std::path::PathBuf;

use anyhow::{anyhow, Context as _, Result};
use deltachat::constants::DC_VERSION_STR;
use deltachat_jsonrpc::api::events::event_to_json_rpc_notification;
use deltachat_jsonrpc::api::{Accounts, CommandApi};
use futures_lite::stream::StreamExt;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;
use tracing::{info, trace};
use tracing_subscriber::{fmt, EnvFilter};
use yerpc::{RpcClient, RpcSession};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let mut args = env::args_os();
    let _program_name = args.next().context("no command line arguments found")?;
    if let Some(first_arg) = args.next() {
        if first_arg.to_str() == Some("--version") {
            if let Some(arg) = args.next() {
                return Err(anyhow!("Unrecognized argument {:?}", arg));
            }
            eprintln!("{}", &*DC_VERSION_STR);
            return Ok(());
        } else {
            return Err(anyhow!("Unrecognized option {:?}", first_arg));
        }
    }
    if let Some(arg) = args.next() {
        return Err(anyhow!("Unrecognized argument {:?}", arg));
    }

    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let path = std::env::var("DC_ACCOUNTS_PATH").unwrap_or_else(|_| "accounts".to_string());
    info!("Starting with accounts directory `{}`.", path);
    let accounts = Accounts::new(PathBuf::from(&path)).await?;
    let events = accounts.get_event_emitter();

    info!("Creating JSON-RPC API.");
    let state = CommandApi::new(accounts);

    let (client, mut out_receiver) = RpcClient::new();
    let session = RpcSession::new(client.clone(), state);

    // Events task converts core events to JSON-RPC notifications.
    let events_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let event = event_to_json_rpc_notification(event);
            client.send_notification("event", Some(event)).await?;
        }
        Ok(())
    });

    // Send task prints JSON responses to stdout.
    let send_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        while let Some(message) = out_receiver.next().await {
            let message = serde_json::to_string(&message)?;
            trace!("RPC send {message}");
            println!("{message}");
        }
        Ok(())
    });

    // Receiver task reads JSON requests from stdin.
    let recv_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let stdin = io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        while let Some(message) = lines.next_line().await? {
            trace!("RPC recv {}", message);
            let session = session.clone();
            tokio::spawn(async move {
                session.handle_incoming(&message).await;
            });
        }
        info!("EOF reached on stdin");
        Ok(())
    });

    // Wait for the end of stdin.
    recv_task.await??;

    // Shutdown the server.
    send_task.abort();
    events_task.abort();

    Ok(())
}
