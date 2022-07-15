use axum::{extract::ws::WebSocketUpgrade, response::Response, routing::get, Extension, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use yerpc::axum::handle_ws_rpc;
use yerpc::{RpcClient, RpcSession};

mod api;
use api::events::event_to_json_rpc_notification;
use api::{Accounts, DeltaChatApiV0};

const DEFAULT_PORT: u16 = 20808;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), std::io::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let path = std::env::var("DC_ACCOUNTS_PATH").unwrap_or_else(|_| "./accounts".to_string());
    let port = std::env::var("DC_PORT")
        .map(|port| port.parse::<u16>().expect("DC_PORT must be a number"))
        .unwrap_or(DEFAULT_PORT);
    log::info!("Starting with accounts directory `{path}`.");
    let accounts = Accounts::new(PathBuf::from(&path)).await.unwrap();
    let state = DeltaChatApiV0::new(accounts);

    let app = Router::new()
        .route("/rpc/v0", get(handler))
        .layer(Extension(state.clone()));

    tokio::spawn(async move {
        state.accounts.read().await.start_io().await;
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
    tokio::spawn(async move {
        let events = api.accounts.read().await.get_event_emitter().await;
        while let Some(event) = events.recv().await {
            let event = event_to_json_rpc_notification(event);
            client.send_notification("event", Some(event)).await.ok();
        }
    });
    handle_ws_rpc(ws, out_receiver, session).await
}
