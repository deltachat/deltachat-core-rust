// AUTO-GENERATED by yerpc-derive

import * as T from "./types.js"
import * as RPC from "./jsonrpc.js"

type RequestMethod = (method: string, params?: RPC.Params) => Promise<unknown>;
type NotificationMethod = (method: string, params?: RPC.Params) => void;

interface Transport {
  request: RequestMethod,
  notification: NotificationMethod
}

export class RawClient {
  constructor(private _transport: Transport) {}

  /**
   * Check if an email address is valid.
   */
  public checkEmailValidity(email: string): Promise<boolean> {
    return (this._transport.request('check_email_validity', [email] as RPC.Params)) as Promise<boolean>;
  }

  /**
   * Get general system info.
   */
  public getSystemInfo(): Promise<Record<string,string>> {
    return (this._transport.request('get_system_info', [] as RPC.Params)) as Promise<Record<string,string>>;
  }


  public addAccount(): Promise<T.U32> {
    return (this._transport.request('add_account', [] as RPC.Params)) as Promise<T.U32>;
  }


  public removeAccount(accountId: T.U32): Promise<null> {
    return (this._transport.request('remove_account', [accountId] as RPC.Params)) as Promise<null>;
  }


  public getAllAccountIds(): Promise<(T.U32)[]> {
    return (this._transport.request('get_all_account_ids', [] as RPC.Params)) as Promise<(T.U32)[]>;
  }

  /**
   * Select account id for internally selected state.
   * TODO: Likely this is deprecated as all methods take an account id now.
   */
  public selectAccount(id: T.U32): Promise<null> {
    return (this._transport.request('select_account', [id] as RPC.Params)) as Promise<null>;
  }

  /**
   * Get the selected account id of the internal state..
   * TODO: Likely this is deprecated as all methods take an account id now.
   */
  public getSelectedAccountId(): Promise<(T.U32|null)> {
    return (this._transport.request('get_selected_account_id', [] as RPC.Params)) as Promise<(T.U32|null)>;
  }

  /**
   * Get a list of all configured accounts.
   */
  public getAllAccounts(): Promise<(T.Account)[]> {
    return (this._transport.request('get_all_accounts', [] as RPC.Params)) as Promise<(T.Account)[]>;
  }

  /**
   * Get top-level info for an account.
   */
  public getAccountInfo(accountId: T.U32): Promise<T.Account> {
    return (this._transport.request('get_account_info', [accountId] as RPC.Params)) as Promise<T.Account>;
  }

  /**
   * Returns provider for the given domain.
   *
   * This function looks up domain in offline database.
   *
   * For compatibility, email address can be passed to this function
   * instead of the domain.
   */
  public getProviderInfo(accountId: T.U32, email: string): Promise<(T.ProviderInfo|null)> {
    return (this._transport.request('get_provider_info', [accountId, email] as RPC.Params)) as Promise<(T.ProviderInfo|null)>;
  }

  /**
   * Checks if the context is already configured.
   */
  public isConfigured(accountId: T.U32): Promise<boolean> {
    return (this._transport.request('is_configured', [accountId] as RPC.Params)) as Promise<boolean>;
  }

  /**
   * Get system info for an account.
   */
  public getInfo(accountId: T.U32): Promise<Record<string,string>> {
    return (this._transport.request('get_info', [accountId] as RPC.Params)) as Promise<Record<string,string>>;
  }


  public setConfig(accountId: T.U32, key: string, value: (string|null)): Promise<null> {
    return (this._transport.request('set_config', [accountId, key, value] as RPC.Params)) as Promise<null>;
  }


  public batchSetConfig(accountId: T.U32, config: Record<string,(string|null)>): Promise<null> {
    return (this._transport.request('batch_set_config', [accountId, config] as RPC.Params)) as Promise<null>;
  }

  /**
   * Set configuration values from a QR code. (technically from the URI that is stored in the qrcode)
   * Before this function is called, `checkQr()` should confirm the type of the
   * QR code is `account` or `webrtcInstance`.
   *
   * Internally, the function will call dc_set_config() with the appropriate keys,
   */
  public setConfigFromQr(accountId: T.U32, qrContent: string): Promise<null> {
    return (this._transport.request('set_config_from_qr', [accountId, qrContent] as RPC.Params)) as Promise<null>;
  }


  public checkQr(accountId: T.U32, qrContent: string): Promise<T.Qr> {
    return (this._transport.request('check_qr', [accountId, qrContent] as RPC.Params)) as Promise<T.Qr>;
  }


  public getConfig(accountId: T.U32, key: string): Promise<(string|null)> {
    return (this._transport.request('get_config', [accountId, key] as RPC.Params)) as Promise<(string|null)>;
  }


  public batchGetConfig(accountId: T.U32, keys: (string)[]): Promise<Record<string,(string|null)>> {
    return (this._transport.request('batch_get_config', [accountId, keys] as RPC.Params)) as Promise<Record<string,(string|null)>>;
  }

  /**
   * Configures this account with the currently set parameters.
   * Setup the credential config before calling this.
   */
  public configure(accountId: T.U32): Promise<null> {
    return (this._transport.request('configure', [accountId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Signal an ongoing process to stop.
   */
  public stopOngoingProcess(accountId: T.U32): Promise<null> {
    return (this._transport.request('stop_ongoing_process', [accountId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Returns the message IDs of all _fresh_ messages of any chat.
   * Typically used for implementing notification summaries
   * or badge counters e.g. on the app icon.
   * The list is already sorted and starts with the most recent fresh message.
   *
   * Messages belonging to muted chats or to the contact requests are not returned;
   * these messages should not be notified
   * and also badge counters should not include these messages.
   *
   * To get the number of fresh messages for a single chat, muted or not,
   * use `get_fresh_msg_cnt()`.
   */
  public getFreshMsgs(accountId: T.U32): Promise<(T.U32)[]> {
    return (this._transport.request('get_fresh_msgs', [accountId] as RPC.Params)) as Promise<(T.U32)[]>;
  }

  /**
   * Get the number of _fresh_ messages in a chat.
   * Typically used to implement a badge with a number in the chatlist.
   *
   * If the specified chat is muted,
   * the UI should show the badge counter "less obtrusive",
   * e.g. using "gray" instead of "red" color.
   */
  public getFreshMsgCnt(accountId: T.U32, chatId: T.U32): Promise<T.Usize> {
    return (this._transport.request('get_fresh_msg_cnt', [accountId, chatId] as RPC.Params)) as Promise<T.Usize>;
  }


  public autocryptInitiateKeyTransfer(accountId: T.U32): Promise<string> {
    return (this._transport.request('autocrypt_initiate_key_transfer', [accountId] as RPC.Params)) as Promise<string>;
  }


  public autocryptContinueKeyTransfer(accountId: T.U32, messageId: T.U32, setupCode: string): Promise<null> {
    return (this._transport.request('autocrypt_continue_key_transfer', [accountId, messageId, setupCode] as RPC.Params)) as Promise<null>;
  }


  public getChatlistEntries(accountId: T.U32, listFlags: (T.U32|null), queryString: (string|null), queryContactId: (T.U32|null)): Promise<(T.ChatListEntry)[]> {
    return (this._transport.request('get_chatlist_entries', [accountId, listFlags, queryString, queryContactId] as RPC.Params)) as Promise<(T.ChatListEntry)[]>;
  }


  public getChatlistItemsByEntries(accountId: T.U32, entries: (T.ChatListEntry)[]): Promise<Record<T.U32,T.ChatListItemFetchResult>> {
    return (this._transport.request('get_chatlist_items_by_entries', [accountId, entries] as RPC.Params)) as Promise<Record<T.U32,T.ChatListItemFetchResult>>;
  }


  public chatlistGetFullChatById(accountId: T.U32, chatId: T.U32): Promise<T.FullChat> {
    return (this._transport.request('chatlist_get_full_chat_by_id', [accountId, chatId] as RPC.Params)) as Promise<T.FullChat>;
  }

  /**
   * get basic info about a chat,
   * use chatlist_get_full_chat_by_id() instead if you need more information
   */
  public getBasicChatInfo(accountId: T.U32, chatId: T.U32): Promise<T.BasicChat> {
    return (this._transport.request('get_basic_chat_info', [accountId, chatId] as RPC.Params)) as Promise<T.BasicChat>;
  }


  public acceptChat(accountId: T.U32, chatId: T.U32): Promise<null> {
    return (this._transport.request('accept_chat', [accountId, chatId] as RPC.Params)) as Promise<null>;
  }


  public blockChat(accountId: T.U32, chatId: T.U32): Promise<null> {
    return (this._transport.request('block_chat', [accountId, chatId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Delete a chat.
   *
   * Messages are deleted from the device and the chat database entry is deleted.
   * After that, the event #DC_EVENT_MSGS_CHANGED is posted.
   *
   * Things that are _not done_ implicitly:
   *
   * - Messages are **not deleted from the server**.
   * - The chat or the contact is **not blocked**, so new messages from the user/the group may appear as a contact request
   *   and the user may create the chat again.
   * - **Groups are not left** - this would
   *   be unexpected as (1) deleting a normal chat also does not prevent new mails
   *   from arriving, (2) leaving a group requires sending a message to
   *   all group members - especially for groups not used for a longer time, this is
   *   really unexpected when deletion results in contacting all members again,
   *   (3) only leaving groups is also a valid usecase.
   *
   * To leave a chat explicitly, use leave_group()
   */
  public deleteChat(accountId: T.U32, chatId: T.U32): Promise<null> {
    return (this._transport.request('delete_chat', [accountId, chatId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Get encryption info for a chat.
   * Get a multi-line encryption info, containing encryption preferences of all members.
   * Can be used to find out why messages sent to group are not encrypted.
   *
   * returns Multi-line text
   */
  public getChatEncryptionInfo(accountId: T.U32, chatId: T.U32): Promise<string> {
    return (this._transport.request('get_chat_encryption_info', [accountId, chatId] as RPC.Params)) as Promise<string>;
  }

  /**
   * Get QR code (text and SVG) that will offer an Setup-Contact or Verified-Group invitation.
   * The QR code is compatible to the OPENPGP4FPR format
   * so that a basic fingerprint comparison also works e.g. with OpenKeychain.
   *
   * The scanning device will pass the scanned content to `checkQr()` then;
   * if `checkQr()` returns `askVerifyContact` or `askVerifyGroup`
   * an out-of-band-verification can be joined using dc_join_securejoin()
   *
   * chat_id: If set to a group-chat-id,
   *     the Verified-Group-Invite protocol is offered in the QR code;
   *     works for protected groups as well as for normal groups.
   *     If not set, the Setup-Contact protocol is offered in the QR code.
   *     See https://countermitm.readthedocs.io/en/latest/new.html
   *     for details about both protocols.
   *
   * return format: `[code, svg]`
   */
  public getChatSecurejoinQrCodeSvg(accountId: T.U32, chatId: (T.U32|null)): Promise<[string,string]> {
    return (this._transport.request('get_chat_securejoin_qr_code_svg', [accountId, chatId] as RPC.Params)) as Promise<[string,string]>;
  }


  public leaveGroup(accountId: T.U32, chatId: T.U32): Promise<null> {
    return (this._transport.request('leave_group', [accountId, chatId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Remove a member from a group.
   *
   * If the group is already _promoted_ (any message was sent to the group),
   * all group members are informed by a special status message that is sent automatically by this function.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
   */
  public removeContactFromChat(accountId: T.U32, chatId: T.U32, contactId: T.U32): Promise<null> {
    return (this._transport.request('remove_contact_from_chat', [accountId, chatId, contactId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Add a member to a group.
   *
   * If the group is already _promoted_ (any message was sent to the group),
   * all group members are informed by a special status message that is sent automatically by this function.
   *
   * If the group has group protection enabled, only verified contacts can be added to the group.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
   */
  public addContactToChat(accountId: T.U32, chatId: T.U32, contactId: T.U32): Promise<null> {
    return (this._transport.request('add_contact_to_chat', [accountId, chatId, contactId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Get the contact IDs belonging to a chat.
   *
   * - for normal chats, the function always returns exactly one contact,
   *   DC_CONTACT_ID_SELF is returned only for SELF-chats.
   *
   * - for group chats all members are returned, DC_CONTACT_ID_SELF is returned
   *   explicitly as it may happen that oneself gets removed from a still existing
   *   group
   *
   * - for broadcasts, all recipients are returned, DC_CONTACT_ID_SELF is not included
   *
   * - for mailing lists, the behavior is not documented currently, we will decide on that later.
   *   for now, the UI should not show the list for mailing lists.
   *   (we do not know all members and there is not always a global mailing list address,
   *   so we could return only SELF or the known members; this is not decided yet)
   */
  public getChatContacts(accountId: T.U32, chatId: T.U32): Promise<(T.U32)[]> {
    return (this._transport.request('get_chat_contacts', [accountId, chatId] as RPC.Params)) as Promise<(T.U32)[]>;
  }

  /**
   * Create a new group chat.
   *
   * After creation,
   * the group has one member with the ID DC_CONTACT_ID_SELF
   * and is in _unpromoted_ state.
   * This means, you can add or remove members, change the name,
   * the group image and so on without messages being sent to all group members.
   *
   * This changes as soon as the first message is sent to the group members
   * and the group becomes _promoted_.
   * After that, all changes are synced with all group members
   * by sending status message.
   *
   * To check, if a chat is still unpromoted, you can look at the `is_unpromoted` property of `BasicChat` or `FullChat`.
   * This may be useful if you want to show some help for just created groups.
   *
   * @param protect If set to 1 the function creates group with protection initially enabled.
   *     Only verified members are allowed in these groups
   *     and end-to-end-encryption is always enabled.
   */
  public createGroupChat(accountId: T.U32, name: string, protect: boolean): Promise<T.U32> {
    return (this._transport.request('create_group_chat', [accountId, name, protect] as RPC.Params)) as Promise<T.U32>;
  }

  /**
   * Create a new broadcast list.
   *
   * Broadcast lists are similar to groups on the sending device,
   * however, recipients get the messages in normal one-to-one chats
   * and will not be aware of other members.
   *
   * Replies to broadcasts go only to the sender
   * and not to all broadcast recipients.
   * Moreover, replies will not appear in the broadcast list
   * but in the one-to-one chat with the person answering.
   *
   * The name and the image of the broadcast list is set automatically
   * and is visible to the sender only.
   * Not asking for these data allows more focused creation
   * and we bypass the question who will get which data.
   * Also, many users will have at most one broadcast list
   * so, a generic name and image is sufficient at the first place.
   *
   * Later on, however, the name can be changed using dc_set_chat_name().
   * The image cannot be changed to have a unique, recognizable icon in the chat lists.
   * All in all, this is also what other messengers are doing here.
   */
  public createBroadcastList(accountId: T.U32): Promise<T.U32> {
    return (this._transport.request('create_broadcast_list', [accountId] as RPC.Params)) as Promise<T.U32>;
  }

  /**
   * Set group name.
   *
   * If the group is already _promoted_ (any message was sent to the group),
   * all group members are informed by a special status message that is sent automatically by this function.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
   */
  public setChatName(accountId: T.U32, chatId: T.U32, newName: string): Promise<null> {
    return (this._transport.request('set_chat_name', [accountId, chatId, newName] as RPC.Params)) as Promise<null>;
  }

  /**
   * Set group profile image.
   *
   * If the group is already _promoted_ (any message was sent to the group),
   * all group members are informed by a special status message that is sent automatically by this function.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
   *
   * To find out the profile image of a chat, use dc_chat_get_profile_image()
   *
   * @param image_path Full path of the image to use as the group image. The image will immediately be copied to the
   *     `blobdir`; the original image will not be needed anymore.
   *      If you pass null here, the group image is deleted (for promoted groups, all members are informed about
   *      this change anyway).
   */
  public setChatProfileImage(accountId: T.U32, chatId: T.U32, imagePath: (string|null)): Promise<null> {
    return (this._transport.request('set_chat_profile_image', [accountId, chatId, imagePath] as RPC.Params)) as Promise<null>;
  }


  public setChatVisibility(accountId: T.U32, chatId: T.U32, visibility: T.ChatVisibility): Promise<null> {
    return (this._transport.request('set_chat_visibility', [accountId, chatId, visibility] as RPC.Params)) as Promise<null>;
  }


  public addDeviceMessage(accountId: T.U32, label: string, text: string): Promise<T.U32> {
    return (this._transport.request('add_device_message', [accountId, label, text] as RPC.Params)) as Promise<T.U32>;
  }

  /**
   *  Mark all messages in a chat as _noticed_.
   *  _Noticed_ messages are no longer _fresh_ and do not count as being unseen
   *  but are still waiting for being marked as "seen" using markseen_msgs()
   *  (IMAP/MDNs is not done for noticed messages).
   *
   *  Calling this function usually results in the event #DC_EVENT_MSGS_NOTICED.
   *  See also markseen_msgs().
   */
  public marknoticedChat(accountId: T.U32, chatId: T.U32): Promise<null> {
    return (this._transport.request('marknoticed_chat', [accountId, chatId] as RPC.Params)) as Promise<null>;
  }


  public getFirstUnreadMessageOfChat(accountId: T.U32, chatId: T.U32): Promise<(T.U32|null)> {
    return (this._transport.request('get_first_unread_message_of_chat', [accountId, chatId] as RPC.Params)) as Promise<(T.U32|null)>;
  }

  /**
   * Set mute duration of a chat.
   *
   * The UI can then call is_chat_muted() when receiving a new message
   * to decide whether it should trigger an notification.
   *
   * Muted chats should not sound or vibrate
   * and should not show a visual notification in the system area.
   * Moreover, muted chats should be excluded from global badge counter
   * (get_fresh_msgs() skips muted chats therefore)
   * and the in-app, per-chat badge counter should use a less obtrusive color.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED.
   */
  public setChatMuteDuration(accountId: T.U32, chatId: T.U32, duration: T.MuteDuration): Promise<null> {
    return (this._transport.request('set_chat_mute_duration', [accountId, chatId, duration] as RPC.Params)) as Promise<null>;
  }

  /**
   * Check whether the chat is currently muted (can be changed by set_chat_mute_duration()).
   *
   * This is available as a standalone function outside of fullchat, because it might be only needed for notification
   */
  public isChatMuted(accountId: T.U32, chatId: T.U32): Promise<boolean> {
    return (this._transport.request('is_chat_muted', [accountId, chatId] as RPC.Params)) as Promise<boolean>;
  }

  /**
   * Mark messages as presented to the user.
   * Typically, UIs call this function on scrolling through the message list,
   * when the messages are presented at least for a little moment.
   * The concrete action depends on the type of the chat and on the users settings
   * (dc_msgs_presented() may be a better name therefore, but well. :)
   *
   * - For normal chats, the IMAP state is updated, MDN is sent
   *   (if set_config()-options `mdns_enabled` is set)
   *   and the internal state is changed to @ref DC_STATE_IN_SEEN to reflect these actions.
   *
   * - For contact requests, no IMAP or MDNs is done
   *   and the internal state is not changed therefore.
   *   See also marknoticed_chat().
   *
   * Moreover, timer is started for incoming ephemeral messages.
   * This also happens for contact requests chats.
   *
   * One #DC_EVENT_MSGS_NOTICED event is emitted per modified chat.
   */
  public markseenMsgs(accountId: T.U32, msgIds: (T.U32)[]): Promise<null> {
    return (this._transport.request('markseen_msgs', [accountId, msgIds] as RPC.Params)) as Promise<null>;
  }


  public messageListGetMessageIds(accountId: T.U32, chatId: T.U32, flags: T.U32): Promise<(T.U32)[]> {
    return (this._transport.request('message_list_get_message_ids', [accountId, chatId, flags] as RPC.Params)) as Promise<(T.U32)[]>;
  }


  public messageGetMessage(accountId: T.U32, messageId: T.U32): Promise<T.Message> {
    return (this._transport.request('message_get_message', [accountId, messageId] as RPC.Params)) as Promise<T.Message>;
  }


  public messageGetMessages(accountId: T.U32, messageIds: (T.U32)[]): Promise<Record<T.U32,T.Message>> {
    return (this._transport.request('message_get_messages', [accountId, messageIds] as RPC.Params)) as Promise<Record<T.U32,T.Message>>;
  }

  /**
   * Fetch info desktop needs for creating a notification for a message
   */
  public messageGetNotificationInfo(accountId: T.U32, messageId: T.U32): Promise<T.MessageNotificationInfo> {
    return (this._transport.request('message_get_notification_info', [accountId, messageId] as RPC.Params)) as Promise<T.MessageNotificationInfo>;
  }

  /**
   * Delete messages. The messages are deleted on the current device and
   * on the IMAP server.
   */
  public deleteMessages(accountId: T.U32, messageIds: (T.U32)[]): Promise<null> {
    return (this._transport.request('delete_messages', [accountId, messageIds] as RPC.Params)) as Promise<null>;
  }

  /**
   * Get an informational text for a single message. The text is multiline and may
   * contain e.g. the raw text of the message.
   *
   * The max. text returned is typically longer (about 100000 characters) than the
   * max. text returned by dc_msg_get_text() (about 30000 characters).
   */
  public getMessageInfo(accountId: T.U32, messageId: T.U32): Promise<string> {
    return (this._transport.request('get_message_info', [accountId, messageId] as RPC.Params)) as Promise<string>;
  }

  /**
   * Asks the core to start downloading a message fully.
   * This function is typically called when the user hits the "Download" button
   * that is shown by the UI in case `download_state` is `'Available'` or `'Failure'`
   *
   * On success, the @ref DC_MSG "view type of the message" may change
   * or the message may be replaced completely by one or more messages with other message IDs.
   * That may happen e.g. in cases where the message was encrypted
   * and the type could not be determined without fully downloading.
   * Downloaded content can be accessed as usual after download.
   *
   * To reflect these changes a @ref DC_EVENT_MSGS_CHANGED event will be emitted.
   */
  public downloadFullMessage(accountId: T.U32, messageId: T.U32): Promise<null> {
    return (this._transport.request('download_full_message', [accountId, messageId] as RPC.Params)) as Promise<null>;
  }

  /**
   * Search messages containing the given query string.
   * Searching can be done globally (chat_id=0) or in a specified chat only (chat_id set).
   *
   * Global chat results are typically displayed using dc_msg_get_summary(), chat
   * search results may just hilite the corresponding messages and present a
   * prev/next button.
   *
   * For global search, result is limited to 1000 messages,
   * this allows incremental search done fast.
   * So, when getting exactly 1000 results, the result may be truncated;
   * the UIs may display sth. as "1000+ messages found" in this case.
   * Chat search (if a chat_id is set) is not limited.
   */
  public searchMessages(accountId: T.U32, query: string, chatId: (T.U32|null)): Promise<(T.U32)[]> {
    return (this._transport.request('search_messages', [accountId, query, chatId] as RPC.Params)) as Promise<(T.U32)[]>;
  }


  public messageIdsToSearchResults(accountId: T.U32, messageIds: (T.U32)[]): Promise<Record<T.U32,T.MessageSearchResult>> {
    return (this._transport.request('message_ids_to_search_results', [accountId, messageIds] as RPC.Params)) as Promise<Record<T.U32,T.MessageSearchResult>>;
  }

  /**
   * Get a single contact options by ID.
   */
  public contactsGetContact(accountId: T.U32, contactId: T.U32): Promise<T.Contact> {
    return (this._transport.request('contacts_get_contact', [accountId, contactId] as RPC.Params)) as Promise<T.Contact>;
  }

  /**
   * Add a single contact as a result of an explicit user action.
   *
   * Returns contact id of the created or existing contact
   */
  public contactsCreateContact(accountId: T.U32, email: string, name: (string|null)): Promise<T.U32> {
    return (this._transport.request('contacts_create_contact', [accountId, email, name] as RPC.Params)) as Promise<T.U32>;
  }

  /**
   * Returns contact id of the created or existing DM chat with that contact
   */
  public contactsCreateChatByContactId(accountId: T.U32, contactId: T.U32): Promise<T.U32> {
    return (this._transport.request('contacts_create_chat_by_contact_id', [accountId, contactId] as RPC.Params)) as Promise<T.U32>;
  }


  public contactsBlock(accountId: T.U32, contactId: T.U32): Promise<null> {
    return (this._transport.request('contacts_block', [accountId, contactId] as RPC.Params)) as Promise<null>;
  }


  public contactsUnblock(accountId: T.U32, contactId: T.U32): Promise<null> {
    return (this._transport.request('contacts_unblock', [accountId, contactId] as RPC.Params)) as Promise<null>;
  }


  public contactsGetBlocked(accountId: T.U32): Promise<(T.Contact)[]> {
    return (this._transport.request('contacts_get_blocked', [accountId] as RPC.Params)) as Promise<(T.Contact)[]>;
  }


  public contactsGetContactIds(accountId: T.U32, listFlags: T.U32, query: (string|null)): Promise<(T.U32)[]> {
    return (this._transport.request('contacts_get_contact_ids', [accountId, listFlags, query] as RPC.Params)) as Promise<(T.U32)[]>;
  }

  /**
   * Get a list of contacts.
   * (formerly called getContacts2 in desktop)
   */
  public contactsGetContacts(accountId: T.U32, listFlags: T.U32, query: (string|null)): Promise<(T.Contact)[]> {
    return (this._transport.request('contacts_get_contacts', [accountId, listFlags, query] as RPC.Params)) as Promise<(T.Contact)[]>;
  }


  public contactsGetContactsByIds(accountId: T.U32, ids: (T.U32)[]): Promise<Record<T.U32,T.Contact>> {
    return (this._transport.request('contacts_get_contacts_by_ids', [accountId, ids] as RPC.Params)) as Promise<Record<T.U32,T.Contact>>;
  }

  /**
   * Get encryption info for a contact.
   * Get a multi-line encryption info, containing your fingerprint and the
   * fingerprint of the contact, used e.g. to compare the fingerprints for a simple out-of-band verification.
   */
  public getContactEncryptionInfo(accountId: T.U32, contactId: T.U32): Promise<string> {
    return (this._transport.request('get_contact_encryption_info', [accountId, contactId] as RPC.Params)) as Promise<string>;
  }

  /**
   * Check if an e-mail address belongs to a known and unblocked contact.
   * To get a list of all known and unblocked contacts, use contacts_get_contacts().
   *
   * To validate an e-mail address independently of the contact database
   * use check_email_validity().
   */
  public lookupContactIdByAddr(accountId: T.U32, addr: string): Promise<(T.U32|null)> {
    return (this._transport.request('lookup_contact_id_by_addr', [accountId, addr] as RPC.Params)) as Promise<(T.U32|null)>;
  }

  /**
   * Returns all message IDs of the given types in a chat.
   * Typically used to show a gallery.
   *
   * The list is already sorted and starts with the oldest message.
   * Clients should not try to re-sort the list as this would be an expensive action
   * and would result in inconsistencies between clients.
   *
   * Setting `chat_id` to `None` (`null` in typescript) means get messages with media
   * from any chat of the currently used account.
   */
  public chatGetMedia(accountId: T.U32, chatId: (T.U32|null), messageType: T.Viewtype, orMessageType2: (T.Viewtype|null), orMessageType3: (T.Viewtype|null)): Promise<(T.U32)[]> {
    return (this._transport.request('chat_get_media', [accountId, chatId, messageType, orMessageType2, orMessageType3] as RPC.Params)) as Promise<(T.U32)[]>;
  }

  /**
   * Search next/previous message based on a given message and a list of types.
   * Typically used to implement the "next" and "previous" buttons
   * in a gallery or in a media player.
   *
   * one combined call for getting chat::get_next_media for both directions
   * the manual chat::get_next_media in only one direction is not exposed by the jsonrpc yet
   */
  public chatGetNeighboringMedia(accountId: T.U32, msgId: T.U32, messageType: T.Viewtype, orMessageType2: (T.Viewtype|null), orMessageType3: (T.Viewtype|null)): Promise<[(T.U32|null),(T.U32|null)]> {
    return (this._transport.request('chat_get_neighboring_media', [accountId, msgId, messageType, orMessageType2, orMessageType3] as RPC.Params)) as Promise<[(T.U32|null),(T.U32|null)]>;
  }

  /**
   * Indicate that the network likely has come back.
   * or just that the network conditions might have changed
   */
  public maybeNetwork(): Promise<null> {
    return (this._transport.request('maybe_network', [] as RPC.Params)) as Promise<null>;
  }

  /**
   * Get the current connectivity, i.e. whether the device is connected to the IMAP server.
   * One of:
   * - DC_CONNECTIVITY_NOT_CONNECTED (1000-1999): Show e.g. the string "Not connected" or a red dot
   * - DC_CONNECTIVITY_CONNECTING (2000-2999): Show e.g. the string "Connecting…" or a yellow dot
   * - DC_CONNECTIVITY_WORKING (3000-3999): Show e.g. the string "Getting new messages" or a spinning wheel
   * - DC_CONNECTIVITY_CONNECTED (>=4000): Show e.g. the string "Connected" or a green dot
   *
   * We don't use exact values but ranges here so that we can split up
   * states into multiple states in the future.
   *
   * Meant as a rough overview that can be shown
   * e.g. in the title of the main screen.
   *
   * If the connectivity changes, a #DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
   */
  public getConnectivity(accountId: T.U32): Promise<T.U32> {
    return (this._transport.request('get_connectivity', [accountId] as RPC.Params)) as Promise<T.U32>;
  }

  /**
   * Get an overview of the current connectivity, and possibly more statistics.
   * Meant to give the user more insight about the current status than
   * the basic connectivity info returned by get_connectivity(); show this
   * e.g., if the user taps on said basic connectivity info.
   *
   * If this page changes, a #DC_EVENT_CONNECTIVITY_CHANGED will be emitted.
   *
   * This comes as an HTML from the core so that we can easily improve it
   * and the improvement instantly reaches all UIs.
   */
  public getConnectivityHtml(accountId: T.U32): Promise<string> {
    return (this._transport.request('get_connectivity_html', [accountId] as RPC.Params)) as Promise<string>;
  }


  public webxdcSendStatusUpdate(accountId: T.U32, instanceMsgId: T.U32, updateStr: string, description: string): Promise<null> {
    return (this._transport.request('webxdc_send_status_update', [accountId, instanceMsgId, updateStr, description] as RPC.Params)) as Promise<null>;
  }


  public webxdcGetStatusUpdates(accountId: T.U32, instanceMsgId: T.U32, lastKnownSerial: T.U32): Promise<string> {
    return (this._transport.request('webxdc_get_status_updates', [accountId, instanceMsgId, lastKnownSerial] as RPC.Params)) as Promise<string>;
  }

  /**
   * Get info from a webxdc message
   */
  public messageGetWebxdcInfo(accountId: T.U32, instanceMsgId: T.U32): Promise<T.WebxdcMessageInfo> {
    return (this._transport.request('message_get_webxdc_info', [accountId, instanceMsgId] as RPC.Params)) as Promise<T.WebxdcMessageInfo>;
  }

  /**
   * Forward messages to another chat.
   *
   * All types of messages can be forwarded,
   * however, they will be flagged as such (dc_msg_is_forwarded() is set).
   *
   * Original sender, info-state and webxdc updates are not forwarded on purpose.
   */
  public forwardMessages(accountId: T.U32, messageIds: (T.U32)[], chatId: T.U32): Promise<null> {
    return (this._transport.request('forward_messages', [accountId, messageIds, chatId] as RPC.Params)) as Promise<null>;
  }


  public removeDraft(accountId: T.U32, chatId: T.U32): Promise<null> {
    return (this._transport.request('remove_draft', [accountId, chatId] as RPC.Params)) as Promise<null>;
  }

  /**
   *  Get draft for a chat, if any.
   */
  public getDraft(accountId: T.U32, chatId: T.U32): Promise<(T.Message|null)> {
    return (this._transport.request('get_draft', [accountId, chatId] as RPC.Params)) as Promise<(T.Message|null)>;
  }


  public sendVideochatInvitation(accountId: T.U32, chatId: T.U32): Promise<T.U32> {
    return (this._transport.request('send_videochat_invitation', [accountId, chatId] as RPC.Params)) as Promise<T.U32>;
  }

  /**
   * Returns the messageid of the sent message
   */
  public miscSendTextMessage(accountId: T.U32, text: string, chatId: T.U32): Promise<T.U32> {
    return (this._transport.request('misc_send_text_message', [accountId, text, chatId] as RPC.Params)) as Promise<T.U32>;
  }


  public miscSendMsg(accountId: T.U32, chatId: T.U32, text: (string|null), file: (string|null), location: ([T.F64,T.F64]|null), quotedMessageId: (T.U32|null)): Promise<[T.U32,T.Message]> {
    return (this._transport.request('misc_send_msg', [accountId, chatId, text, file, location, quotedMessageId] as RPC.Params)) as Promise<[T.U32,T.Message]>;
  }


  public miscSetDraft(accountId: T.U32, chatId: T.U32, text: (string|null), file: (string|null), quotedMessageId: (T.U32|null)): Promise<null> {
    return (this._transport.request('misc_set_draft', [accountId, chatId, text, file, quotedMessageId] as RPC.Params)) as Promise<null>;
  }


}
