use anyhow::{anyhow, bail, Context, Result};
use deltachat::{
    chat::{
        self, add_contact_to_chat, forward_msgs, get_chat_media, get_chat_msgs, marknoticed_chat,
        remove_contact_from_chat, Chat, ChatId, ChatItem, ProtectionStatus,
    },
    chatlist::Chatlist,
    config::Config,
    constants::DC_MSG_ID_DAYMARKER,
    contact::{may_be_valid_addr, Contact, ContactId, Origin},
    context::get_info,
    ephemeral::Timer,
    imex, location,
    message::{
        self, delete_msgs, get_msg_info, markseen_msgs, Message, MessageState, MsgId, Viewtype,
    },
    provider::get_provider_info,
    qr,
    qr_code_generator::get_securejoin_qr_svg,
    reaction::send_reaction,
    securejoin,
    stock_str::StockMessage,
    webxdc::StatusUpdateSerial,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};
use tokio::{fs, sync::RwLock};
use walkdir::WalkDir;
use yerpc::rpc;

pub use deltachat::accounts::Accounts;

pub mod events;
pub mod types;

use crate::api::types::chat_list::{get_chat_list_item_by_id, ChatListItemFetchResult};
use crate::api::types::qr::QrObject;

use types::account::Account;
use types::chat::FullChat;
use types::chat_list::ChatListEntry;
use types::contact::ContactObject;
use types::message::MessageObject;
use types::provider_info::ProviderInfo;
use types::webxdc::WebxdcMessageInfo;

use self::types::{
    chat::{BasicChat, JSONRPCChatVisibility, MuteDuration},
    location::JsonrpcLocation,
    message::{
        JSONRPCMessageListItem, MessageNotificationInfo, MessageSearchResult, MessageViewtype,
    },
};

use num_traits::FromPrimitive;

#[derive(Clone, Debug)]
pub struct CommandApi {
    pub(crate) accounts: Arc<RwLock<Accounts>>,
}

impl CommandApi {
    pub fn new(accounts: Accounts) -> Self {
        CommandApi {
            accounts: Arc::new(RwLock::new(accounts)),
        }
    }

    #[allow(dead_code)]
    pub fn from_arc(accounts: Arc<RwLock<Accounts>>) -> Self {
        CommandApi { accounts }
    }

    async fn get_context(&self, id: u32) -> Result<deltachat::context::Context> {
        let sc = self
            .accounts
            .read()
            .await
            .get_account(id)
            .ok_or_else(|| anyhow!("account with id {} not found", id))?;
        Ok(sc)
    }
}

#[rpc(all_positional, ts_outdir = "typescript/generated")]
impl CommandApi {
    // ---------------------------------------------
    //  Misc top level functions
    // ---------------------------------------------

    /// Check if an email address is valid.
    async fn check_email_validity(&self, email: String) -> bool {
        may_be_valid_addr(&email)
    }

    /// Get general system info.
    async fn get_system_info(&self) -> BTreeMap<&'static str, String> {
        get_info()
    }

    // ---------------------------------------------
    // Account Management
    // ---------------------------------------------

    async fn add_account(&self) -> Result<u32> {
        self.accounts.write().await.add_account().await
    }

    async fn remove_account(&self, account_id: u32) -> Result<()> {
        self.accounts.write().await.remove_account(account_id).await
    }

    async fn get_all_account_ids(&self) -> Vec<u32> {
        self.accounts.read().await.get_all()
    }

    /// Select account id for internally selected state.
    /// TODO: Likely this is deprecated as all methods take an account id now.
    async fn select_account(&self, id: u32) -> Result<()> {
        self.accounts.write().await.select_account(id).await
    }

    /// Get the selected account id of the internal state..
    /// TODO: Likely this is deprecated as all methods take an account id now.
    async fn get_selected_account_id(&self) -> Option<u32> {
        self.accounts.read().await.get_selected_account_id()
    }

    /// Get a list of all configured accounts.
    async fn get_all_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        for id in self.accounts.read().await.get_all() {
            let context_option = self.accounts.read().await.get_account(id);
            if let Some(ctx) = context_option {
                accounts.push(Account::from_context(&ctx, id).await?)
            } else {
                println!("account with id {} doesn't exist anymore", id);
            }
        }
        Ok(accounts)
    }

    async fn start_io_for_all_accounts(&self) -> Result<()> {
        self.accounts.read().await.start_io().await;
        Ok(())
    }

    async fn stop_io_for_all_accounts(&self) -> Result<()> {
        self.accounts.read().await.stop_io().await;
        Ok(())
    }

    // ---------------------------------------------
    // Methods that work on individual accounts
    // ---------------------------------------------

    async fn start_io(&self, id: u32) -> Result<()> {
        let ctx = self.get_context(id).await?;
        ctx.start_io().await;
        Ok(())
    }

    async fn stop_io(&self, id: u32) -> Result<()> {
        let ctx = self.get_context(id).await?;
        ctx.stop_io().await;
        Ok(())
    }

    /// Get top-level info for an account.
    async fn get_account_info(&self, account_id: u32) -> Result<Account> {
        let context_option = self.accounts.read().await.get_account(account_id);
        if let Some(ctx) = context_option {
            Ok(Account::from_context(&ctx, account_id).await?)
        } else {
            Err(anyhow!(
                "account with id {} doesn't exist anymore",
                account_id
            ))
        }
    }

    /// Get the combined filesize of an account in bytes
    async fn get_account_file_size(&self, account_id: u32) -> Result<u64> {
        let ctx = self.get_context(account_id).await?;
        let dbfile = ctx.get_dbfile().metadata()?.len();
        let total_size = WalkDir::new(ctx.get_blobdir())
            .max_depth(2)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .fold(0, |acc, m| acc + m.len());

        Ok(dbfile + total_size)
    }

    /// Returns provider for the given domain.
    ///
    /// This function looks up domain in offline database.
    ///
    /// For compatibility, email address can be passed to this function
    /// instead of the domain.
    async fn get_provider_info(
        &self,
        account_id: u32,
        email: String,
    ) -> Result<Option<ProviderInfo>> {
        let ctx = self.get_context(account_id).await?;

        let socks5_enabled = ctx
            .get_config_bool(deltachat::config::Config::Socks5Enabled)
            .await?;

        let provider_info =
            get_provider_info(&ctx, email.split('@').last().unwrap_or(""), socks5_enabled).await;
        Ok(ProviderInfo::from_dc_type(provider_info))
    }

    /// Checks if the context is already configured.
    async fn is_configured(&self, account_id: u32) -> Result<bool> {
        let ctx = self.get_context(account_id).await?;
        ctx.is_configured().await
    }

    /// Get system info for an account.
    async fn get_info(&self, account_id: u32) -> Result<BTreeMap<&'static str, String>> {
        let ctx = self.get_context(account_id).await?;
        ctx.get_info().await
    }

    async fn set_config(&self, account_id: u32, key: String, value: Option<String>) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        set_config(&ctx, &key, value.as_deref()).await
    }

    async fn batch_set_config(
        &self,
        account_id: u32,
        config: HashMap<String, Option<String>>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        for (key, value) in config.into_iter() {
            set_config(&ctx, &key, value.as_deref())
                .await
                .with_context(|| format!("Can't set {} to {:?}", key, value))?;
        }
        Ok(())
    }

    /// Set configuration values from a QR code. (technically from the URI that is stored in the qrcode)
    /// Before this function is called, `checkQr()` should confirm the type of the
    /// QR code is `account` or `webrtcInstance`.
    ///
    /// Internally, the function will call dc_set_config() with the appropriate keys,
    async fn set_config_from_qr(&self, account_id: u32, qr_content: String) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        qr::set_config_from_qr(&ctx, &qr_content).await
    }

    async fn check_qr(&self, account_id: u32, qr_content: String) -> Result<QrObject> {
        let ctx = self.get_context(account_id).await?;
        let qr = qr::check_qr(&ctx, &qr_content).await?;
        let qr_object = QrObject::from(qr);
        Ok(qr_object)
    }

    async fn get_config(&self, account_id: u32, key: String) -> Result<Option<String>> {
        let ctx = self.get_context(account_id).await?;
        get_config(&ctx, &key).await
    }

    async fn batch_get_config(
        &self,
        account_id: u32,
        keys: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let ctx = self.get_context(account_id).await?;
        let mut result: HashMap<String, Option<String>> = HashMap::new();
        for key in keys {
            result.insert(key.clone(), get_config(&ctx, &key).await?);
        }
        Ok(result)
    }

    async fn set_stock_strings(&self, strings: HashMap<u32, String>) -> Result<()> {
        let accounts = self.accounts.read().await;
        for (stock_id, stock_message) in strings {
            if let Some(stock_id) = StockMessage::from_u32(stock_id) {
                accounts
                    .set_stock_translation(stock_id, stock_message)
                    .await?;
            }
        }
        Ok(())
    }

    /// Configures this account with the currently set parameters.
    /// Setup the credential config before calling this.
    async fn configure(&self, account_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ctx.stop_io().await;
        let result = ctx.configure().await;
        if result.is_err() {
            if let Ok(true) = ctx.is_configured().await {
                ctx.start_io().await;
            }
            return result;
        }
        ctx.start_io().await;
        Ok(())
    }

    /// Signal an ongoing process to stop.
    async fn stop_ongoing_process(&self, account_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ctx.stop_ongoing().await;
        Ok(())
    }

    async fn export_self_keys(
        &self,
        account_id: u32,
        path: String,
        passphrase: Option<String>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        imex::imex(
            &ctx,
            imex::ImexMode::ExportSelfKeys,
            path.as_ref(),
            passphrase,
        )
        .await
    }

    async fn import_self_keys(
        &self,
        account_id: u32,
        path: String,
        passphrase: Option<String>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        imex::imex(
            &ctx,
            imex::ImexMode::ImportSelfKeys,
            path.as_ref(),
            passphrase,
        )
        .await
    }

    /// Returns the message IDs of all _fresh_ messages of any chat.
    /// Typically used for implementing notification summaries
    /// or badge counters e.g. on the app icon.
    /// The list is already sorted and starts with the most recent fresh message.
    ///
    /// Messages belonging to muted chats or to the contact requests are not returned;
    /// these messages should not be notified
    /// and also badge counters should not include these messages.
    ///
    /// To get the number of fresh messages for a single chat, muted or not,
    /// use `get_fresh_msg_cnt()`.
    async fn get_fresh_msgs(&self, account_id: u32) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        Ok(ctx
            .get_fresh_msgs()
            .await?
            .iter()
            .map(|msg_id| msg_id.to_u32())
            .collect())
    }

    /// Get the number of _fresh_ messages in a chat.
    /// Typically used to implement a badge with a number in the chatlist.
    ///
    /// If the specified chat is muted,
    /// the UI should show the badge counter "less obtrusive",
    /// e.g. using "gray" instead of "red" color.
    async fn get_fresh_msg_cnt(&self, account_id: u32, chat_id: u32) -> Result<usize> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).get_fresh_msg_cnt(&ctx).await
    }

    /// Estimate the number of messages that will be deleted
    /// by the set_config()-options `delete_device_after` or `delete_server_after`.
    /// This is typically used to show the estimated impact to the user
    /// before actually enabling deletion of old messages.
    async fn estimate_auto_deletion_count(
        &self,
        account_id: u32,
        from_server: bool,
        seconds: i64,
    ) -> Result<usize> {
        let ctx = self.get_context(account_id).await?;
        message::estimate_deletion_cnt(&ctx, from_server, seconds).await
    }

    // ---------------------------------------------
    //  autocrypt
    // ---------------------------------------------

    async fn initiate_autocrypt_key_transfer(&self, account_id: u32) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        deltachat::imex::initiate_key_transfer(&ctx).await
    }

    async fn continue_autocrypt_key_transfer(
        &self,
        account_id: u32,
        message_id: u32,
        setup_code: String,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        deltachat::imex::continue_key_transfer(&ctx, MsgId::new(message_id), &setup_code).await
    }

    // ---------------------------------------------
    //   chat list
    // ---------------------------------------------

    async fn get_chatlist_entries(
        &self,
        account_id: u32,
        list_flags: Option<u32>,
        query_string: Option<String>,
        query_contact_id: Option<u32>,
    ) -> Result<Vec<ChatListEntry>> {
        let ctx = self.get_context(account_id).await?;
        let list = Chatlist::try_load(
            &ctx,
            list_flags.unwrap_or(0) as usize,
            query_string.as_deref(),
            query_contact_id.map(ContactId::new),
        )
        .await?;
        let mut l: Vec<ChatListEntry> = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            l.push(ChatListEntry(
                list.get_chat_id(i)?.to_u32(),
                list.get_msg_id(i)?.unwrap_or_default().to_u32(),
            ));
        }
        Ok(l)
    }

    async fn get_chatlist_items_by_entries(
        &self,
        account_id: u32,
        entries: Vec<ChatListEntry>,
    ) -> Result<HashMap<u32, ChatListItemFetchResult>> {
        // todo custom json deserializer for ChatListEntry?
        let ctx = self.get_context(account_id).await?;
        let mut result: HashMap<u32, ChatListItemFetchResult> =
            HashMap::with_capacity(entries.len());
        for entry in entries.iter() {
            result.insert(
                entry.0,
                match get_chat_list_item_by_id(&ctx, entry).await {
                    Ok(res) => res,
                    Err(err) => ChatListItemFetchResult::Error {
                        id: entry.0,
                        error: format!("{:?}", err),
                    },
                },
            );
        }
        Ok(result)
    }

    // ---------------------------------------------
    //  chat
    // ---------------------------------------------

    async fn get_full_chat_by_id(&self, account_id: u32, chat_id: u32) -> Result<FullChat> {
        let ctx = self.get_context(account_id).await?;
        FullChat::try_from_dc_chat_id(&ctx, chat_id).await
    }

    /// get basic info about a chat,
    /// use chatlist_get_full_chat_by_id() instead if you need more information
    async fn get_basic_chat_info(&self, account_id: u32, chat_id: u32) -> Result<BasicChat> {
        let ctx = self.get_context(account_id).await?;
        BasicChat::try_from_dc_chat_id(&ctx, chat_id).await
    }

    async fn accept_chat(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).accept(&ctx).await
    }

    async fn block_chat(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).block(&ctx).await
    }

    /// Delete a chat.
    ///
    /// Messages are deleted from the device and the chat database entry is deleted.
    /// After that, the event #DC_EVENT_MSGS_CHANGED is posted.
    ///
    /// Things that are _not done_ implicitly:
    ///
    /// - Messages are **not deleted from the server**.
    /// - The chat or the contact is **not blocked**, so new messages from the user/the group may appear as a contact request
    ///   and the user may create the chat again.
    /// - **Groups are not left** - this would
    ///   be unexpected as (1) deleting a normal chat also does not prevent new mails
    ///   from arriving, (2) leaving a group requires sending a message to
    ///   all group members - especially for groups not used for a longer time, this is
    ///   really unexpected when deletion results in contacting all members again,
    ///   (3) only leaving groups is also a valid usecase.
    ///
    /// To leave a chat explicitly, use leave_group()
    async fn delete_chat(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).delete(&ctx).await
    }

    /// Get encryption info for a chat.
    /// Get a multi-line encryption info, containing encryption preferences of all members.
    /// Can be used to find out why messages sent to group are not encrypted.
    ///
    /// returns Multi-line text
    async fn get_chat_encryption_info(&self, account_id: u32, chat_id: u32) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).get_encryption_info(&ctx).await
    }

    /// Get QR code (text and SVG) that will offer an Setup-Contact or Verified-Group invitation.
    /// The QR code is compatible to the OPENPGP4FPR format
    /// so that a basic fingerprint comparison also works e.g. with OpenKeychain.
    ///
    /// The scanning device will pass the scanned content to `checkQr()` then;
    /// if `checkQr()` returns `askVerifyContact` or `askVerifyGroup`
    /// an out-of-band-verification can be joined using `secure_join()`
    ///
    /// chat_id: If set to a group-chat-id,
    ///     the Verified-Group-Invite protocol is offered in the QR code;
    ///     works for protected groups as well as for normal groups.
    ///     If not set, the Setup-Contact protocol is offered in the QR code.
    ///     See https://countermitm.readthedocs.io/en/latest/new.html
    ///     for details about both protocols.
    ///
    /// return format: `[code, svg]`
    async fn get_chat_securejoin_qr_code_svg(
        &self,
        account_id: u32,
        chat_id: Option<u32>,
    ) -> Result<(String, String)> {
        let ctx = self.get_context(account_id).await?;
        let chat = chat_id.map(ChatId::new);
        Ok((
            securejoin::get_securejoin_qr(&ctx, chat).await?,
            get_securejoin_qr_svg(&ctx, chat).await?,
        ))
    }

    /// Continue a Setup-Contact or Verified-Group-Invite protocol
    /// started on another device with `get_chat_securejoin_qr_code_svg()`.
    /// This function is typically called when `check_qr()` returns
    /// type=AskVerifyContact or type=AskVerifyGroup.
    ///
    /// The function returns immediately and the handshake runs in background,
    /// sending and receiving several messages.
    /// During the handshake, info messages are added to the chat,
    /// showing progress, success or errors.
    ///
    /// Subsequent calls of `secure_join()` will abort previous, unfinished handshakes.
    ///
    /// See https://countermitm.readthedocs.io/en/latest/new.html
    /// for details about both protocols.
    ///
    /// **qr**: The text of the scanned QR code. Typically, the same string as given
    ///     to `check_qr()`.
    ///
    /// **returns**: The chat ID of the joined chat, the UI may redirect to the this chat.
    ///         A returned chat ID does not guarantee that the chat is protected or the belonging contact is verified.
    ///
    async fn secure_join(&self, account_id: u32, qr: String) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        let chat_id = securejoin::join_securejoin(&ctx, &qr).await?;
        Ok(chat_id.to_u32())
    }

    async fn leave_group(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        remove_contact_from_chat(&ctx, ChatId::new(chat_id), ContactId::SELF).await
    }

    /// Remove a member from a group.
    ///
    /// If the group is already _promoted_ (any message was sent to the group),
    /// all group members are informed by a special status message that is sent automatically by this function.
    ///
    /// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
    async fn remove_contact_from_chat(
        &self,
        account_id: u32,
        chat_id: u32,
        contact_id: u32,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        remove_contact_from_chat(&ctx, ChatId::new(chat_id), ContactId::new(contact_id)).await
    }

    /// Add a member to a group.
    ///
    /// If the group is already _promoted_ (any message was sent to the group),
    /// all group members are informed by a special status message that is sent automatically by this function.
    ///
    /// If the group has group protection enabled, only verified contacts can be added to the group.
    ///
    /// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
    async fn add_contact_to_chat(
        &self,
        account_id: u32,
        chat_id: u32,
        contact_id: u32,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        add_contact_to_chat(&ctx, ChatId::new(chat_id), ContactId::new(contact_id)).await
    }

    /// Get the contact IDs belonging to a chat.
    ///
    /// - for normal chats, the function always returns exactly one contact,
    ///   DC_CONTACT_ID_SELF is returned only for SELF-chats.
    ///
    /// - for group chats all members are returned, DC_CONTACT_ID_SELF is returned
    ///   explicitly as it may happen that oneself gets removed from a still existing
    ///   group
    ///
    /// - for broadcasts, all recipients are returned, DC_CONTACT_ID_SELF is not included
    ///
    /// - for mailing lists, the behavior is not documented currently, we will decide on that later.
    ///   for now, the UI should not show the list for mailing lists.
    ///   (we do not know all members and there is not always a global mailing list address,
    ///   so we could return only SELF or the known members; this is not decided yet)
    async fn get_chat_contacts(&self, account_id: u32, chat_id: u32) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        let contacts = chat::get_chat_contacts(&ctx, ChatId::new(chat_id)).await?;
        Ok(contacts.iter().map(|id| id.to_u32()).collect::<Vec<u32>>())
    }

    /// Create a new group chat.
    ///
    /// After creation,
    /// the group has one member with the ID DC_CONTACT_ID_SELF
    /// and is in _unpromoted_ state.
    /// This means, you can add or remove members, change the name,
    /// the group image and so on without messages being sent to all group members.
    ///
    /// This changes as soon as the first message is sent to the group members
    /// and the group becomes _promoted_.
    /// After that, all changes are synced with all group members
    /// by sending status message.
    ///
    /// To check, if a chat is still unpromoted, you can look at the `is_unpromoted` property of `BasicChat` or `FullChat`.
    /// This may be useful if you want to show some help for just created groups.
    ///
    /// @param protect If set to 1 the function creates group with protection initially enabled.
    ///     Only verified members are allowed in these groups
    ///     and end-to-end-encryption is always enabled.
    async fn create_group_chat(&self, account_id: u32, name: String, protect: bool) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        let protect = match protect {
            true => ProtectionStatus::Protected,
            false => ProtectionStatus::Unprotected,
        };
        chat::create_group_chat(&ctx, protect, &name)
            .await
            .map(|id| id.to_u32())
    }

    /// Create a new broadcast list.
    ///
    /// Broadcast lists are similar to groups on the sending device,
    /// however, recipients get the messages in normal one-to-one chats
    /// and will not be aware of other members.
    ///
    /// Replies to broadcasts go only to the sender
    /// and not to all broadcast recipients.
    /// Moreover, replies will not appear in the broadcast list
    /// but in the one-to-one chat with the person answering.
    ///
    /// The name and the image of the broadcast list is set automatically
    /// and is visible to the sender only.
    /// Not asking for these data allows more focused creation
    /// and we bypass the question who will get which data.
    /// Also, many users will have at most one broadcast list
    /// so, a generic name and image is sufficient at the first place.
    ///
    /// Later on, however, the name can be changed using dc_set_chat_name().
    /// The image cannot be changed to have a unique, recognizable icon in the chat lists.
    /// All in all, this is also what other messengers are doing here.
    async fn create_broadcast_list(&self, account_id: u32) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        chat::create_broadcast_list(&ctx)
            .await
            .map(|id| id.to_u32())
    }

    /// Set group name.
    ///
    /// If the group is already _promoted_ (any message was sent to the group),
    /// all group members are informed by a special status message that is sent automatically by this function.
    ///
    /// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
    async fn set_chat_name(&self, account_id: u32, chat_id: u32, new_name: String) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        chat::set_chat_name(&ctx, ChatId::new(chat_id), &new_name).await
    }

    /// Set group profile image.
    ///
    /// If the group is already _promoted_ (any message was sent to the group),
    /// all group members are informed by a special status message that is sent automatically by this function.
    ///
    /// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
    ///
    /// To find out the profile image of a chat, use dc_chat_get_profile_image()
    ///
    /// @param image_path Full path of the image to use as the group image. The image will immediately be copied to the
    ///     `blobdir`; the original image will not be needed anymore.
    ///      If you pass null here, the group image is deleted (for promoted groups, all members are informed about
    ///      this change anyway).
    async fn set_chat_profile_image(
        &self,
        account_id: u32,
        chat_id: u32,
        image_path: Option<String>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        chat::set_chat_profile_image(&ctx, ChatId::new(chat_id), image_path.unwrap_or_default())
            .await
    }

    async fn set_chat_visibility(
        &self,
        account_id: u32,
        chat_id: u32,
        visibility: JSONRPCChatVisibility,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;

        ChatId::new(chat_id)
            .set_visibility(&ctx, visibility.into_core_type())
            .await
    }

    async fn set_chat_ephemeral_timer(
        &self,
        account_id: u32,
        chat_id: u32,
        timer: u32,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id)
            .set_ephemeral_timer(&ctx, Timer::from_u32(timer))
            .await
    }

    async fn get_chat_ephemeral_timer(&self, account_id: u32, chat_id: u32) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        Ok(ChatId::new(chat_id)
            .get_ephemeral_timer(&ctx)
            .await?
            .to_u32())
    }

    // for now only text messages, because we only used text messages in desktop thusfar
    async fn add_device_message(
        &self,
        account_id: u32,
        label: String,
        text: String,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(text));
        let message_id =
            deltachat::chat::add_device_msg(&ctx, Some(&label), Some(&mut msg)).await?;
        Ok(message_id.to_u32())
    }

    ///  Mark all messages in a chat as _noticed_.
    ///  _Noticed_ messages are no longer _fresh_ and do not count as being unseen
    ///  but are still waiting for being marked as "seen" using markseen_msgs()
    ///  (IMAP/MDNs is not done for noticed messages).
    ///
    ///  Calling this function usually results in the event #DC_EVENT_MSGS_NOTICED.
    ///  See also markseen_msgs().
    async fn marknoticed_chat(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        marknoticed_chat(&ctx, ChatId::new(chat_id)).await
    }

    async fn get_first_unread_message_of_chat(
        &self,
        account_id: u32,
        chat_id: u32,
    ) -> Result<Option<u32>> {
        let ctx = self.get_context(account_id).await?;

        // TODO: implement this in core with an SQL query, that will be way faster
        let messages = get_chat_msgs(&ctx, ChatId::new(chat_id), 0).await?;
        let mut first_unread_message_id = None;
        for item in messages.into_iter().rev() {
            if let ChatItem::Message { msg_id } = item {
                match msg_id.get_state(&ctx).await? {
                    MessageState::InSeen => break,
                    MessageState::InFresh | MessageState::InNoticed => {
                        first_unread_message_id = Some(msg_id)
                    }
                    _ => continue,
                }
            }
        }
        Ok(first_unread_message_id.map(|id| id.to_u32()))
    }

    /// Set mute duration of a chat.
    ///
    /// The UI can then call is_chat_muted() when receiving a new message
    /// to decide whether it should trigger an notification.
    ///
    /// Muted chats should not sound or vibrate
    /// and should not show a visual notification in the system area.
    /// Moreover, muted chats should be excluded from global badge counter
    /// (get_fresh_msgs() skips muted chats therefore)
    /// and the in-app, per-chat badge counter should use a less obtrusive color.
    ///
    /// Sends out #DC_EVENT_CHAT_MODIFIED.
    async fn set_chat_mute_duration(
        &self,
        account_id: u32,
        chat_id: u32,
        duration: MuteDuration,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        chat::set_muted(&ctx, ChatId::new(chat_id), duration.try_into_core_type()?).await
    }

    /// Check whether the chat is currently muted (can be changed by set_chat_mute_duration()).
    ///
    /// This is available as a standalone function outside of fullchat, because it might be only needed for notification
    async fn is_chat_muted(&self, account_id: u32, chat_id: u32) -> Result<bool> {
        let ctx = self.get_context(account_id).await?;
        Ok(Chat::load_from_db(&ctx, ChatId::new(chat_id))
            .await?
            .is_muted())
    }

    // ---------------------------------------------
    // message list
    // ---------------------------------------------

    /// Mark messages as presented to the user.
    /// Typically, UIs call this function on scrolling through the message list,
    /// when the messages are presented at least for a little moment.
    /// The concrete action depends on the type of the chat and on the users settings
    /// (dc_msgs_presented() may be a better name therefore, but well. :)
    ///
    /// - For normal chats, the IMAP state is updated, MDN is sent
    ///   (if set_config()-options `mdns_enabled` is set)
    ///   and the internal state is changed to @ref DC_STATE_IN_SEEN to reflect these actions.
    ///
    /// - For contact requests, no IMAP or MDNs is done
    ///   and the internal state is not changed therefore.
    ///   See also marknoticed_chat().
    ///
    /// Moreover, timer is started for incoming ephemeral messages.
    /// This also happens for contact requests chats.
    ///
    /// One #DC_EVENT_MSGS_NOTICED event is emitted per modified chat.
    async fn markseen_msgs(&self, account_id: u32, msg_ids: Vec<u32>) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        markseen_msgs(&ctx, msg_ids.into_iter().map(MsgId::new).collect()).await
    }

    async fn get_message_ids(&self, account_id: u32, chat_id: u32, flags: u32) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        let msg = get_chat_msgs(&ctx, ChatId::new(chat_id), flags).await?;
        Ok(msg
            .iter()
            .map(|chat_item| -> u32 {
                match chat_item {
                    deltachat::chat::ChatItem::Message { msg_id } => msg_id.to_u32(),
                    deltachat::chat::ChatItem::DayMarker { .. } => DC_MSG_ID_DAYMARKER,
                }
            })
            .collect())
    }

    async fn get_message_list_items(
        &self,
        account_id: u32,
        chat_id: u32,
        flags: u32,
    ) -> Result<Vec<JSONRPCMessageListItem>> {
        let ctx = self.get_context(account_id).await?;
        let msg = get_chat_msgs(&ctx, ChatId::new(chat_id), flags).await?;
        Ok(msg
            .iter()
            .map(|chat_item| (*chat_item).into())
            .collect::<Vec<JSONRPCMessageListItem>>())
    }

    async fn get_message(&self, account_id: u32, message_id: u32) -> Result<MessageObject> {
        let ctx = self.get_context(account_id).await?;
        MessageObject::from_message_id(&ctx, message_id).await
    }

    async fn get_message_html(&self, account_id: u32, message_id: u32) -> Result<Option<String>> {
        let ctx = self.get_context(account_id).await?;
        MsgId::new(message_id).get_html(&ctx).await
    }

    async fn get_messages(
        &self,
        account_id: u32,
        message_ids: Vec<u32>,
    ) -> Result<HashMap<u32, MessageObject>> {
        let ctx = self.get_context(account_id).await?;
        let mut messages: HashMap<u32, MessageObject> = HashMap::new();
        for message_id in message_ids {
            messages.insert(
                message_id,
                MessageObject::from_message_id(&ctx, message_id).await?,
            );
        }
        Ok(messages)
    }

    /// Fetch info desktop needs for creating a notification for a message
    async fn get_message_notification_info(
        &self,
        account_id: u32,
        message_id: u32,
    ) -> Result<MessageNotificationInfo> {
        let ctx = self.get_context(account_id).await?;
        MessageNotificationInfo::from_msg_id(&ctx, MsgId::new(message_id)).await
    }

    /// Delete messages. The messages are deleted on the current device and
    /// on the IMAP server.
    async fn delete_messages(&self, account_id: u32, message_ids: Vec<u32>) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        let msgs: Vec<MsgId> = message_ids.into_iter().map(MsgId::new).collect();
        delete_msgs(&ctx, &msgs).await
    }

    /// Get an informational text for a single message. The text is multiline and may
    /// contain e.g. the raw text of the message.
    ///
    /// The max. text returned is typically longer (about 100000 characters) than the
    /// max. text returned by dc_msg_get_text() (about 30000 characters).
    async fn get_message_info(&self, account_id: u32, message_id: u32) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        get_msg_info(&ctx, MsgId::new(message_id)).await
    }

    /// Asks the core to start downloading a message fully.
    /// This function is typically called when the user hits the "Download" button
    /// that is shown by the UI in case `download_state` is `'Available'` or `'Failure'`
    ///
    /// On success, the @ref DC_MSG "view type of the message" may change
    /// or the message may be replaced completely by one or more messages with other message IDs.
    /// That may happen e.g. in cases where the message was encrypted
    /// and the type could not be determined without fully downloading.
    /// Downloaded content can be accessed as usual after download.
    ///
    /// To reflect these changes a @ref DC_EVENT_MSGS_CHANGED event will be emitted.
    async fn download_full_message(&self, account_id: u32, message_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        MsgId::new(message_id).download_full(&ctx).await
    }

    /// Search messages containing the given query string.
    /// Searching can be done globally (chat_id=0) or in a specified chat only (chat_id set).
    ///
    /// Global chat results are typically displayed using dc_msg_get_summary(), chat
    /// search results may just hilite the corresponding messages and present a
    /// prev/next button.
    ///
    /// For global search, result is limited to 1000 messages,
    /// this allows incremental search done fast.
    /// So, when getting exactly 1000 results, the result may be truncated;
    /// the UIs may display sth. as "1000+ messages found" in this case.
    /// Chat search (if a chat_id is set) is not limited.
    async fn search_messages(
        &self,
        account_id: u32,
        query: String,
        chat_id: Option<u32>,
    ) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        let messages = ctx.search_msgs(chat_id.map(ChatId::new), &query).await?;
        Ok(messages
            .iter()
            .map(|msg_id| msg_id.to_u32())
            .collect::<Vec<u32>>())
    }

    async fn message_ids_to_search_results(
        &self,
        account_id: u32,
        message_ids: Vec<u32>,
    ) -> Result<HashMap<u32, MessageSearchResult>> {
        let ctx = self.get_context(account_id).await?;
        let mut results: HashMap<u32, MessageSearchResult> =
            HashMap::with_capacity(message_ids.len());
        for id in message_ids {
            results.insert(
                id,
                MessageSearchResult::from_msg_id(&ctx, MsgId::new(id)).await?,
            );
        }
        Ok(results)
    }

    // ---------------------------------------------
    //  contact
    // ---------------------------------------------

    /// Get a single contact options by ID.
    async fn get_contact(&self, account_id: u32, contact_id: u32) -> Result<ContactObject> {
        let ctx = self.get_context(account_id).await?;
        let contact_id = ContactId::new(contact_id);

        ContactObject::try_from_dc_contact(
            &ctx,
            deltachat::contact::Contact::get_by_id(&ctx, contact_id).await?,
        )
        .await
    }

    /// Add a single contact as a result of an explicit user action.
    ///
    /// Returns contact id of the created or existing contact
    async fn create_contact(
        &self,
        account_id: u32,
        email: String,
        name: Option<String>,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        if !may_be_valid_addr(&email) {
            bail!(anyhow!(
                "provided email address is not a valid email address"
            ))
        }
        let contact_id = Contact::create(&ctx, &name.unwrap_or_default(), &email).await?;
        Ok(contact_id.to_u32())
    }

    /// Returns contact id of the created or existing DM chat with that contact
    async fn create_chat_by_contact_id(&self, account_id: u32, contact_id: u32) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        let contact = Contact::get_by_id(&ctx, ContactId::new(contact_id)).await?;
        ChatId::create_for_contact(&ctx, contact.id)
            .await
            .map(|id| id.to_u32())
    }

    async fn block_contact(&self, account_id: u32, contact_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        Contact::block(&ctx, ContactId::new(contact_id)).await
    }

    async fn unblock_contact(&self, account_id: u32, contact_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        Contact::unblock(&ctx, ContactId::new(contact_id)).await
    }

    async fn get_blocked_contacts(&self, account_id: u32) -> Result<Vec<ContactObject>> {
        let ctx = self.get_context(account_id).await?;
        let blocked_ids = Contact::get_all_blocked(&ctx).await?;
        let mut contacts: Vec<ContactObject> = Vec::with_capacity(blocked_ids.len());
        for id in blocked_ids {
            contacts.push(
                ContactObject::try_from_dc_contact(
                    &ctx,
                    deltachat::contact::Contact::get_by_id(&ctx, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn get_contact_ids(
        &self,
        account_id: u32,
        list_flags: u32,
        query: Option<String>,
    ) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        let contacts = Contact::get_all(&ctx, list_flags, query.as_deref()).await?;
        Ok(contacts.into_iter().map(|c| c.to_u32()).collect())
    }

    /// Get a list of contacts.
    /// (formerly called getContacts2 in desktop)
    async fn get_contacts(
        &self,
        account_id: u32,
        list_flags: u32,
        query: Option<String>,
    ) -> Result<Vec<ContactObject>> {
        let ctx = self.get_context(account_id).await?;
        let contact_ids = Contact::get_all(&ctx, list_flags, query.as_deref()).await?;
        let mut contacts: Vec<ContactObject> = Vec::with_capacity(contact_ids.len());
        for id in contact_ids {
            contacts.push(
                ContactObject::try_from_dc_contact(
                    &ctx,
                    deltachat::contact::Contact::get_by_id(&ctx, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn get_contacts_by_ids(
        &self,
        account_id: u32,
        ids: Vec<u32>,
    ) -> Result<HashMap<u32, ContactObject>> {
        let ctx = self.get_context(account_id).await?;

        let mut contacts = HashMap::with_capacity(ids.len());
        for id in ids {
            contacts.insert(
                id,
                ContactObject::try_from_dc_contact(
                    &ctx,
                    deltachat::contact::Contact::get_by_id(&ctx, ContactId::new(id)).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn delete_contact(&self, account_id: u32, contact_id: u32) -> Result<bool> {
        let ctx = self.get_context(account_id).await?;
        let contact_id = ContactId::new(contact_id);

        Contact::delete(&ctx, contact_id).await?;
        Ok(true)
    }

    async fn change_contact_name(
        &self,
        account_id: u32,
        contact_id: u32,
        name: String,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        let contact_id = ContactId::new(contact_id);
        let contact = Contact::load_from_db(&ctx, contact_id).await?;
        let addr = contact.get_addr();
        Contact::create(&ctx, &name, addr).await?;
        Ok(())
    }

    /// Get encryption info for a contact.
    /// Get a multi-line encryption info, containing your fingerprint and the
    /// fingerprint of the contact, used e.g. to compare the fingerprints for a simple out-of-band verification.
    async fn get_contact_encryption_info(
        &self,
        account_id: u32,
        contact_id: u32,
    ) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        Contact::get_encrinfo(&ctx, ContactId::new(contact_id)).await
    }

    /// Check if an e-mail address belongs to a known and unblocked contact.
    /// To get a list of all known and unblocked contacts, use contacts_get_contacts().
    ///
    /// To validate an e-mail address independently of the contact database
    /// use check_email_validity().
    async fn lookup_contact_id_by_addr(
        &self,
        account_id: u32,
        addr: String,
    ) -> Result<Option<u32>> {
        let ctx = self.get_context(account_id).await?;
        let contact_id = Contact::lookup_id_by_addr(&ctx, &addr, Origin::IncomingReplyTo).await?;
        Ok(contact_id.map(|id| id.to_u32()))
    }

    // ---------------------------------------------
    //                   chat
    // ---------------------------------------------

    /// Returns all message IDs of the given types in a chat.
    /// Typically used to show a gallery.
    ///
    /// The list is already sorted and starts with the oldest message.
    /// Clients should not try to re-sort the list as this would be an expensive action
    /// and would result in inconsistencies between clients.
    ///
    /// Setting `chat_id` to `None` (`null` in typescript) means get messages with media
    /// from any chat of the currently used account.
    async fn get_chat_media(
        &self,
        account_id: u32,
        chat_id: Option<u32>,
        message_type: MessageViewtype,
        or_message_type2: Option<MessageViewtype>,
        or_message_type3: Option<MessageViewtype>,
    ) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;

        let chat_id = match chat_id {
            None | Some(0) => None,
            Some(id) => Some(ChatId::new(id)),
        };
        let msg_type = message_type.into();
        let or_msg_type2 = or_message_type2.map_or(Viewtype::Unknown, |v| v.into());
        let or_msg_type3 = or_message_type3.map_or(Viewtype::Unknown, |v| v.into());

        let media = get_chat_media(&ctx, chat_id, msg_type, or_msg_type2, or_msg_type3).await?;
        Ok(media.iter().map(|msg_id| msg_id.to_u32()).collect())
    }

    /// Search next/previous message based on a given message and a list of types.
    /// Typically used to implement the "next" and "previous" buttons
    /// in a gallery or in a media player.
    ///
    /// one combined call for getting chat::get_next_media for both directions
    /// the manual chat::get_next_media in only one direction is not exposed by the jsonrpc yet
    async fn get_neighboring_chat_media(
        &self,
        account_id: u32,
        msg_id: u32,
        message_type: MessageViewtype,
        or_message_type2: Option<MessageViewtype>,
        or_message_type3: Option<MessageViewtype>,
    ) -> Result<(Option<u32>, Option<u32>)> {
        let ctx = self.get_context(account_id).await?;

        let msg_type: Viewtype = message_type.into();
        let msg_type2: Viewtype = or_message_type2.map(|v| v.into()).unwrap_or_default();
        let msg_type3: Viewtype = or_message_type3.map(|v| v.into()).unwrap_or_default();

        let prev = chat::get_next_media(
            &ctx,
            MsgId::new(msg_id),
            chat::Direction::Backward,
            msg_type,
            msg_type2,
            msg_type3,
        )
        .await?
        .map(|id| id.to_u32());

        let next = chat::get_next_media(
            &ctx,
            MsgId::new(msg_id),
            chat::Direction::Forward,
            msg_type,
            msg_type2,
            msg_type3,
        )
        .await?
        .map(|id| id.to_u32());

        Ok((prev, next))
    }

    // ---------------------------------------------
    //                   backup
    // ---------------------------------------------

    async fn export_backup(
        &self,
        account_id: u32,
        destination: String,
        passphrase: Option<String>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ctx.stop_io().await;
        let result = imex::imex(
            &ctx,
            imex::ImexMode::ExportBackup,
            destination.as_ref(),
            passphrase,
        )
        .await;
        ctx.start_io().await;
        result
    }

    async fn import_backup(
        &self,
        account_id: u32,
        path: String,
        passphrase: Option<String>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        imex::imex(
            &ctx,
            imex::ImexMode::ImportBackup,
            path.as_ref(),
            passphrase,
        )
        .await
    }

    // ---------------------------------------------
    //                connectivity
    // ---------------------------------------------

    /// Indicate that the network likely has come back.
    /// or just that the network conditions might have changed
    async fn maybe_network(&self) -> Result<()> {
        self.accounts.read().await.maybe_network().await;
        Ok(())
    }

    /// Get the current connectivity, i.e. whether the device is connected to the IMAP server.
    /// One of:
    /// - DC_CONNECTIVITY_NOT_CONNECTED (1000-1999): Show e.g. the string "Not connected" or a red dot
    /// - DC_CONNECTIVITY_CONNECTING (2000-2999): Show e.g. the string "Connecting…" or a yellow dot
    /// - DC_CONNECTIVITY_WORKING (3000-3999): Show e.g. the string "Getting new messages" or a spinning wheel
    /// - DC_CONNECTIVITY_CONNECTED (>=4000): Show e.g. the string "Connected" or a green dot
    ///
    /// We don't use exact values but ranges here so that we can split up
    /// states into multiple states in the future.
    ///
    /// Meant as a rough overview that can be shown
    /// e.g. in the title of the main screen.
    ///
    /// If the connectivity changes, a #DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
    async fn get_connectivity(&self, account_id: u32) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        Ok(ctx.get_connectivity().await as u32)
    }

    /// Get an overview of the current connectivity, and possibly more statistics.
    /// Meant to give the user more insight about the current status than
    /// the basic connectivity info returned by get_connectivity(); show this
    /// e.g., if the user taps on said basic connectivity info.
    ///
    /// If this page changes, a #DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
    ///
    /// This comes as an HTML from the core so that we can easily improve it
    /// and the improvement instantly reaches all UIs.
    async fn get_connectivity_html(&self, account_id: u32) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        ctx.get_connectivity_html().await
    }

    // ---------------------------------------------
    //                  locations
    // ---------------------------------------------

    async fn get_locations(
        &self,
        account_id: u32,
        chat_id: Option<u32>,
        contact_id: Option<u32>,
        timestamp_begin: i64,
        timestamp_end: i64,
    ) -> Result<Vec<JsonrpcLocation>> {
        let ctx = self.get_context(account_id).await?;

        let locations = location::get_range(
            &ctx,
            chat_id.map(ChatId::new),
            contact_id,
            timestamp_begin,
            timestamp_end,
        )
        .await?;

        Ok(locations.into_iter().map(|l| l.into()).collect())
    }

    // ---------------------------------------------
    //                   webxdc
    // ---------------------------------------------

    async fn send_webxdc_status_update(
        &self,
        account_id: u32,
        instance_msg_id: u32,
        update_str: String,
        description: String,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ctx.send_webxdc_status_update(MsgId::new(instance_msg_id), &update_str, &description)
            .await
    }

    async fn get_webxdc_status_updates(
        &self,
        account_id: u32,
        instance_msg_id: u32,
        last_known_serial: u32,
    ) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        ctx.get_webxdc_status_updates(
            MsgId::new(instance_msg_id),
            StatusUpdateSerial::new(last_known_serial),
        )
        .await
    }

    /// Get info from a webxdc message
    async fn get_webxdc_info(
        &self,
        account_id: u32,
        instance_msg_id: u32,
    ) -> Result<WebxdcMessageInfo> {
        let ctx = self.get_context(account_id).await?;
        WebxdcMessageInfo::get_for_message(&ctx, MsgId::new(instance_msg_id)).await
    }

    /// Forward messages to another chat.
    ///
    /// All types of messages can be forwarded,
    /// however, they will be flagged as such (dc_msg_is_forwarded() is set).
    ///
    /// Original sender, info-state and webxdc updates are not forwarded on purpose.
    async fn forward_messages(
        &self,
        account_id: u32,
        message_ids: Vec<u32>,
        chat_id: u32,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        let message_ids: Vec<MsgId> = message_ids.into_iter().map(MsgId::new).collect();
        forward_msgs(&ctx, &message_ids, ChatId::new(chat_id)).await
    }

    async fn send_sticker(
        &self,
        account_id: u32,
        chat_id: u32,
        sticker_path: String,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;

        let mut msg = Message::new(Viewtype::Sticker);
        msg.set_file(&sticker_path, None);

        let message_id = deltachat::chat::send_msg(&ctx, ChatId::new(chat_id), &mut msg).await?;
        Ok(message_id.to_u32())
    }

    /// Send a reaction to message.
    ///
    /// Reaction is a string of emojis separated by spaces. Reaction to a
    /// single message can be sent multiple times. The last reaction
    /// received overrides all previously received reactions. It is
    /// possible to remove all reactions by sending an empty string.
    async fn send_reaction(
        &self,
        account_id: u32,
        message_id: u32,
        reaction: Vec<String>,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        let message_id = send_reaction(&ctx, MsgId::new(message_id), &reaction.join(" ")).await?;
        Ok(message_id.to_u32())
    }

    // ---------------------------------------------
    //           functions for the composer
    //    the composer is the message input field
    // ---------------------------------------------

    async fn remove_draft(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).set_draft(&ctx, None).await
    }

    ///  Get draft for a chat, if any.
    async fn get_draft(&self, account_id: u32, chat_id: u32) -> Result<Option<MessageObject>> {
        let ctx = self.get_context(account_id).await?;
        if let Some(draft) = ChatId::new(chat_id).get_draft(&ctx).await? {
            Ok(Some(
                MessageObject::from_msg_id(&ctx, draft.get_id()).await?,
            ))
        } else {
            Ok(None)
        }
    }

    async fn send_videochat_invitation(&self, account_id: u32, chat_id: u32) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        chat::send_videochat_invitation(&ctx, ChatId::new(chat_id))
            .await
            .map(|msg_id| msg_id.to_u32())
    }

    // ---------------------------------------------
    //           misc prototyping functions
    //       that might get removed later again
    // ---------------------------------------------

    async fn misc_get_sticker_folder(&self, account_id: u32) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        let account_folder = ctx
            .get_dbfile()
            .parent()
            .context("account folder not found")?;
        let sticker_folder_path = account_folder.join("./stickers");
        fs::create_dir_all(&sticker_folder_path).await?;
        sticker_folder_path
            .to_str()
            .map(|s| s.to_owned())
            .context("path conversion to string failed")
    }

    async fn misc_save_sticker(
        &self,
        account_id: u32,
        message_id: u32,
        collection: String,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        let message = Message::load_from_db(&ctx, MsgId::new(message_id)).await?;
        if message.get_viewtype() != Viewtype::Sticker {
            bail!("message is not a sticker");
        }
        let account_folder = ctx
            .get_dbfile()
            .parent()
            .context("account folder not found")?;
        if collection.contains('/') || collection.contains('\\') || collection.contains('.') {
            bail!("illegal characters in collection name");
        }
        let destination_path = account_folder.join("./stickers").join(collection);
        fs::create_dir_all(&destination_path).await?;
        let file = message.get_file(&ctx).context("no file")?;
        fs::copy(
            &file,
            destination_path.join(format!(
                "{}.{}",
                message_id,
                file.extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
            )),
        )
        .await?;
        Ok(())
    }

    /// for desktop, get stickers from stickers folder,
    /// grouped by the folder they are in.
    async fn misc_get_stickers(&self, account_id: u32) -> Result<HashMap<String, Vec<String>>> {
        let ctx = self.get_context(account_id).await?;
        let account_folder = ctx
            .get_dbfile()
            .parent()
            .context("account folder not found")?;
        let sticker_folder_path = account_folder.join("./stickers");
        fs::create_dir_all(&sticker_folder_path).await?;
        let mut result = HashMap::new();

        let mut packs = tokio::fs::read_dir(sticker_folder_path).await?;
        while let Some(entry) = packs.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let pack_name = entry.file_name().into_string().unwrap_or_default();
            let mut stickers = tokio::fs::read_dir(entry.path()).await?;
            let mut sticker_paths = Vec::new();
            while let Some(sticker_entry) = stickers.next_entry().await? {
                if !sticker_entry.file_type().await?.is_file() {
                    continue;
                }
                let sticker_name = sticker_entry.file_name().into_string().unwrap_or_default();
                if sticker_name.ends_with(".png") || sticker_name.ends_with(".webp") {
                    sticker_paths.push(
                        sticker_entry
                            .path()
                            .to_str()
                            .map(|s| s.to_owned())
                            .context("path conversion to string failed")?,
                    );
                }
            }
            if !sticker_paths.is_empty() {
                result.insert(pack_name, sticker_paths);
            }
        }

        Ok(result)
    }

    /// Returns the messageid of the sent message
    async fn misc_send_text_message(
        &self,
        account_id: u32,
        chat_id: u32,
        text: String,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(text));

        let message_id = deltachat::chat::send_msg(&ctx, ChatId::new(chat_id), &mut msg).await?;
        Ok(message_id.to_u32())
    }

    // mimics the old desktop call, will get replaced with something better in the composer rewrite,
    // the better version will just be sending the current draft, though there will be probably something similar with more options to this for the corner cases like setting a marker on the map
    async fn misc_send_msg(
        &self,
        account_id: u32,
        chat_id: u32,
        text: Option<String>,
        file: Option<String>,
        location: Option<(f64, f64)>,
        quoted_message_id: Option<u32>,
    ) -> Result<(u32, MessageObject)> {
        let ctx = self.get_context(account_id).await?;
        let mut message = Message::new(if file.is_some() {
            Viewtype::File
        } else {
            Viewtype::Text
        });
        if text.is_some() {
            message.set_text(text);
        }
        if let Some(file) = file {
            message.set_file(file, None);
        }
        if let Some((latitude, longitude)) = location {
            message.set_location(latitude, longitude);
        }
        if let Some(id) = quoted_message_id {
            message
                .set_quote(
                    &ctx,
                    Some(
                        &Message::load_from_db(&ctx, MsgId::new(id))
                            .await
                            .context("message to quote could not be loaded")?,
                    ),
                )
                .await?;
        }
        let msg_id = chat::send_msg(&ctx, ChatId::new(chat_id), &mut message)
            .await?
            .to_u32();
        let message = MessageObject::from_message_id(&ctx, msg_id).await?;
        Ok((msg_id, message))
    }

    // mimics the old desktop call, will get replaced with something better in the composer rewrite,
    // the better version should support:
    // - changing viewtype to enable/disable compression
    // - keeping same message id as long as attachment does not change for webxdc messages
    async fn misc_set_draft(
        &self,
        account_id: u32,
        chat_id: u32,
        text: Option<String>,
        file: Option<String>,
        quoted_message_id: Option<u32>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        let mut draft = Message::new(if file.is_some() {
            Viewtype::File
        } else {
            Viewtype::Text
        });
        if text.is_some() {
            draft.set_text(text);
        }
        if let Some(file) = file {
            draft.set_file(file, None);
        }
        if let Some(id) = quoted_message_id {
            draft
                .set_quote(
                    &ctx,
                    Some(
                        &Message::load_from_db(&ctx, MsgId::new(id))
                            .await
                            .context("message to quote could not be loaded")?,
                    ),
                )
                .await?;
        }

        ChatId::new(chat_id).set_draft(&ctx, Some(&mut draft)).await
    }
}

// Helper functions (to prevent code duplication)
async fn set_config(
    ctx: &deltachat::context::Context,
    key: &str,
    value: Option<&str>,
) -> Result<(), anyhow::Error> {
    if key.starts_with("ui.") {
        ctx.set_ui_config(key, value).await?;
    } else {
        ctx.set_config(Config::from_str(key).context("unknown key")?, value)
            .await?;

        match key {
            "sentbox_watch" | "mvbox_move" | "only_fetch_mvbox" => {
                ctx.restart_io_if_running().await;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn get_config(
    ctx: &deltachat::context::Context,
    key: &str,
) -> Result<Option<String>, anyhow::Error> {
    if key.starts_with("ui.") {
        ctx.get_ui_config(key).await
    } else {
        ctx.get_config(Config::from_str(key).context("unknown key")?)
            .await
    }
}
