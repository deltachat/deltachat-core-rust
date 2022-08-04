pub mod api;
pub use api::events;
pub use yerpc;

#[cfg(test)]
mod tests {
    use super::api::{Accounts, CommandApi};
    use async_channel::unbounded;
    use futures::StreamExt;
    use tempfile::TempDir;
    use yerpc::{RpcClient, RpcSession};

    #[tokio::test(flavor = "multi_thread")]
    async fn basic_json_rpc_functionality() -> anyhow::Result<()> {
        let tmp_dir = TempDir::new().unwrap().path().into();
        let accounts = Accounts::new(tmp_dir).await?;
        let api = CommandApi::new(accounts);

        let (sender, mut receiver) = unbounded::<String>();

        let (client, mut rx) = RpcClient::new();
        let session = RpcSession::new(client, api);
        tokio::spawn({
            async move {
                while let Some(message) = rx.next().await {
                    let message = serde_json::to_string(&message)?;
                    sender.send(message).await?;
                }
                let res: Result<(), anyhow::Error> = Ok(());
                res
            }
        });

        {
            let request = r#"{"jsonrpc":"2.0","method":"add_account","params":[],"id":1}"#;
            let response = r#"{"jsonrpc":"2.0","id":1,"result":1}"#;
            session.handle_incoming(request).await;
            let result = receiver.next().await;
            println!("{:?}", result);
            assert_eq!(result, Some(response.to_owned()));
        }
        {
            let request = r#"{"jsonrpc":"2.0","method":"get_all_account_ids","params":[],"id":2}"#;
            let response = r#"{"jsonrpc":"2.0","id":2,"result":[1]}"#;
            session.handle_incoming(request).await;
            let result = receiver.next().await;
            println!("{:?}", result);
            assert_eq!(result, Some(response.to_owned()));
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_batch_set_config() -> anyhow::Result<()> {
        let tmp_dir = TempDir::new().unwrap().path().into();
        let accounts = Accounts::new(tmp_dir).await?;
        let api = CommandApi::new(accounts);

        let (sender, mut receiver) = unbounded::<String>();

        let (client, mut rx) = RpcClient::new();
        let session = RpcSession::new(client, api);
        tokio::spawn({
            async move {
                while let Some(message) = rx.next().await {
                    let message = serde_json::to_string(&message)?;
                    sender.send(message).await?;
                }
                let res: Result<(), anyhow::Error> = Ok(());
                res
            }
        });

        {
            let request = r#"{"jsonrpc":"2.0","method":"add_account","params":[],"id":1}"#;
            let response = r#"{"jsonrpc":"2.0","id":1,"result":1}"#;
            session.handle_incoming(request).await;
            let result = receiver.next().await;
            assert_eq!(result, Some(response.to_owned()));
        }
        {
            let request = r#"{"jsonrpc":"2.0","method":"batch_set_config","id":2,"params":[1,{"addr":"","mail_user":"","mail_pw":"","mail_server":"","mail_port":"","mail_security":"","imap_certificate_checks":"","send_user":"","send_pw":"","send_server":"","send_port":"","send_security":"","smtp_certificate_checks":"","socks5_enabled":"0","socks5_host":"","socks5_port":"","socks5_user":"","socks5_password":""}]}"#;
            let response = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
            session.handle_incoming(request).await;
            let result = receiver.next().await;
            assert_eq!(result, Some(response.to_owned()));
        }

        Ok(())
    }
}
