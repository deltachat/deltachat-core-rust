pub mod api;
pub use api::events;

pub use yerpc;

#[cfg(test)]
mod tests {
    use super::api::{Accounts, CommandApi};
    use async_channel::unbounded;
    use async_std::task;
    use futures::StreamExt;
    use tempfile::TempDir;
    use yerpc::{MessageHandle, RpcHandle};

    #[async_std::test]
    async fn basic_json_rpc_functionality() -> anyhow::Result<()> {
        // println!("{}", "");
        let tmp_dir = TempDir::new().unwrap().path().into();
        println!("tmp_dir: {:?}", tmp_dir);

        let accounts = Accounts::new(tmp_dir).await?;
        let cmd_api = CommandApi::new(accounts);

        let (sender, mut receiver) = unbounded::<String>();

        let (request_handle, mut rx) = RpcHandle::new();
        let session = cmd_api;
        let handle = MessageHandle::new(request_handle, session);
        task::spawn({
            async move {
                while let Some(message) = rx.next().await {
                    let message = serde_json::to_string(&message)?;
                    // Abort serialization on error.
                    sender.send(message).await?;
                }
                let res: Result<(), anyhow::Error> = Ok(());
                res
            }
        });

        {
            let request = r#"{"jsonrpc":"2.0","method":"add_account","params":[],"id":1}"#;
            let response = r#"{"jsonrpc":"2.0","id":1,"result":1}"#;
            handle.handle_message(request).await;
            let result = receiver.next().await;
            println!("{:?}", result);
            assert_eq!(result, Some(response.to_owned()));
        }
        {
            let request = r#"{"jsonrpc":"2.0","method":"get_all_account_ids","params":[],"id":2}"#;
            let response = r#"{"jsonrpc":"2.0","id":2,"result":[1]}"#;
            handle.handle_message(request).await;
            let result = receiver.next().await;
            println!("{:?}", result);
            assert_eq!(result, Some(response.to_owned()));
        }

        Ok(())
    }
}
