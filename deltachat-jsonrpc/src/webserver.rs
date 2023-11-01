use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{extract::ws::WebSocketUpgrade, response::Response, routing::get, Extension, Router};
use yerpc::axum::handle_ws_rpc;
use yerpc::{RpcClient, RpcSession};

mod api;
use api::{Accounts, CommandApi};

const DEFAULT_PORT: u16 = 20808;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), std::io::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let path = std::env::var("DC_ACCOUNTS_PATH").unwrap_or_else(|_| "./accounts".to_string());
    let port = std::env::var("DC_PORT")
        .map(|port| port.parse::<u16>().expect("DC_PORT must be a number"))
        .unwrap_or(DEFAULT_PORT);
    log::info!("Starting with accounts directory `{path}`.");
    let writable = true;
    let accounts = Accounts::new(PathBuf::from(&path), writable).await.unwrap();
    let state = CommandApi::new(accounts);

    let app = Router::new()
        .route("/ws", get(handler))
        .layer(Extension(state.clone()));

    tokio::spawn(async move {
        state.accounts.write().await.start_io().await;
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    log::info!("JSON-RPC WebSocket server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn handler(ws: WebSocketUpgrade, Extension(api): Extension<CommandApi>) -> Response {
    let (client, out_receiver) = RpcClient::new();
    let session = RpcSession::new(client.clone(), api.clone());
    handle_ws_rpc(ws, out_receiver, session).await
}
