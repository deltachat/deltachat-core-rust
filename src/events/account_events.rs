//! contains tests for account (list) events

#[cfg(test)]
mod test {

    use std::time::Duration;

    use anyhow::Result;
    use tempfile::tempdir;

    use crate::accounts::Accounts;
    use crate::imex::{get_backup, has_backup, imex, BackupProvider, ImexMode};
    use crate::test_utils::{EventTracker, TestContext, TestContextManager};
    use crate::EventType;

    async fn wait_for_item_changed(context: &TestContext) {
        context
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::AccountsItemChanged))
            .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_account_event() -> Result<()> {
        let dir = tempdir().unwrap();
        let mut manager = Accounts::new(dir.path().join("accounts"), true).await?;
        let tracker = EventTracker::new(manager.get_event_emitter());

        // create account
        let account_id = manager.add_account().await?;
        tracker
            .get_matching(|evt| matches!(evt, EventType::AccountsChanged))
            .await;

        // remove account
        manager.remove_account(account_id).await?;
        tracker
            .get_matching(|evt| matches!(evt, EventType::AccountsChanged))
            .await;

        // create closed account
        manager.add_closed_account().await?;
        tracker
            .get_matching(|evt| matches!(evt, EventType::AccountsChanged))
            .await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_configuration() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let context = tcm.unconfigured().await;
        context.configure_addr("delta@example.com").await;
        wait_for_item_changed(&context).await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_displayname() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let context = tcm.alice().await;
        context
            .set_config(crate::config::Config::Displayname, Some("🐰 Alice"))
            .await?;
        wait_for_item_changed(&context).await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_selfavatar() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let context = tcm.alice().await;
        let file = context.dir.path().join("avatar.jpg");
        let bytes = include_bytes!("../../test-data/image/avatar1000x1000.jpg");
        tokio::fs::write(&file, bytes).await?;
        context
            .set_config(
                crate::config::Config::Selfavatar,
                Some(file.to_str().unwrap()),
            )
            .await?;
        wait_for_item_changed(&context).await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_private_tag() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let context = tcm.alice().await;
        context
            .set_config(crate::config::Config::PrivateTag, Some("Wonderland"))
            .await?;
        wait_for_item_changed(&context).await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_import_backup() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let context1 = tcm.alice().await;
        let backup_dir = tempfile::tempdir().unwrap();
        assert!(
            imex(&context1, ImexMode::ExportBackup, backup_dir.path(), None)
                .await
                .is_ok()
        );

        let context2 = TestContext::new().await;
        assert!(!context2.is_configured().await?);
        let backup = has_backup(&context2, backup_dir.path()).await?;
        imex(&context2, ImexMode::ImportBackup, backup.as_ref(), None).await?;
        assert!(context2.is_configured().await?);
        wait_for_item_changed(&context2).await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_receive_backup() {
        let mut tcm = TestContextManager::new();
        // Create first device.
        let ctx0 = tcm.alice().await;
        // Prepare to transfer backup.
        let provider = BackupProvider::prepare(&ctx0).await.unwrap();
        // Set up second device.
        let ctx1 = tcm.unconfigured().await;

        ctx1.evtracker.clear_events();
        get_backup(&ctx1, provider.qr()).await.unwrap();

        // Make sure the provider finishes without an error.
        tokio::time::timeout(Duration::from_secs(30), provider)
            .await
            .expect("timed out")
            .expect("error in provider");

        wait_for_item_changed(&ctx1).await;
    }

    // TODO: test receiving synced config from second device
}
