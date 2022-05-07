use async_std::path::PathBuf;
use async_std::task;
use tide::Request;
use yerpc::RpcHandle;
use yerpc_tide::yerpc_handler;

mod api;
use api::events::event_to_json_rpc_notification;
use api::{Accounts, CommandApi};

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    log::info!("Starting");

    let accounts = Accounts::new(PathBuf::from("./accounts")).await.unwrap();
    let state = CommandApi::new(accounts);

    let mut app = tide::with_state(state.clone());
    app.at("/ws").get(yerpc_handler(request_handler));

    state.accounts.read().await.start_io().await;
    app.listen("127.0.0.1:20808").await?;

    Ok(())
}
async fn request_handler(
    request: Request<CommandApi>,
    rpc: RpcHandle,
) -> anyhow::Result<CommandApi> {
    let state = request.state().clone();
    task::spawn(event_loop(state.clone(), rpc));
    Ok(state)
}

async fn event_loop(state: CommandApi, rpc: RpcHandle) -> anyhow::Result<()> {
    let mut events = state.accounts.read().await.get_event_emitter().await;
    while let Ok(Some(event)) = events.recv().await {
        // log::debug!("event {:?}", event);
        let event = event_to_json_rpc_notification(event);
        rpc.notify("event", Some(event)).await?;
    }
    Ok(())
}
