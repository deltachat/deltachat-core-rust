import * as T from "./types.js"
import { RawClient } from "./client.js"
import { TinyEmitter } from "tiny-emitter";
export class Context<T> extends TinyEmitter<T> {
  constructor(
    private controller: {rpc: RawClient},
    readonly accountId: T.U32
  ){
    super()
  }

  public removeAccount(): Promise<null> {
     return this.controller.rpc.removeAccount(this.accountId)
  }

  /**
   * Get top-level info for an account.
   */
  public getAccountInfo(): Promise<T.Account> {
     return this.controller.rpc.getAccountInfo(this.accountId)
  }

  /**
   * Get the combined filesize of an account in bytes
   */
  public getAccountFileSize(): Promise<T.U64> {
     return this.controller.rpc.getAccountFileSize(this.accountId)
  }

  /**
   * Returns provider for the given domain.
   *
   * This function looks up domain in offline database.
   *
   * For compatibility, email address can be passed to this function
   * instead of the domain.
   */
  public getProviderInfo(email: string): Promise<(T.ProviderInfo|null)> {
     return this.controller.rpc.getProviderInfo(this.accountId, email)
  }

  /**
   * Checks if the context is already configured.
   */
  public isConfigured(): Promise<boolean> {
     return this.controller.rpc.isConfigured(this.accountId)
  }

  /**
   * Get system info for an account.
   */
  public getInfo(): Promise<Record<string,string>> {
     return this.controller.rpc.getInfo(this.accountId)
  }


  public setConfig(key: string, value: (string|null)): Promise<null> {
     return this.controller.rpc.setConfig(this.accountId, key, value)
  }


  public batchSetConfig(config: Record<string,(string|null)>): Promise<null> {
     return this.controller.rpc.batchSetConfig(this.accountId, config)
  }

  /**
   * Set configuration values from a QR code. (technically from the URI that is stored in the qrcode)
   * Before this function is called, `checkQr()` should confirm the type of the
   * QR code is `account` or `webrtcInstance`.
   *
   * Internally, the function will call dc_set_config() with the appropriate keys,
   */
  public setConfigFromQr(qrContent: string): Promise<null> {
     return this.controller.rpc.setConfigFromQr(this.accountId, qrContent)
  }


  public checkQr(qrContent: string): Promise<T.Qr> {
     return this.controller.rpc.checkQr(this.accountId, qrContent)
  }


  public getConfig(key: string): Promise<(string|null)> {
     return this.controller.rpc.getConfig(this.accountId, key)
  }


  public batchGetConfig(keys: (string)[]): Promise<Record<string,(string|null)>> {
     return this.controller.rpc.batchGetConfig(this.accountId, keys)
  }

  /**
   * Configures this account with the currently set parameters.
   * Setup the credential config before calling this.
   */
  public configure(): Promise<null> {
     return this.controller.rpc.configure(this.accountId)
  }

  /**
   * Signal an ongoing process to stop.
   */
  public stopOngoingProcess(): Promise<null> {
     return this.controller.rpc.stopOngoingProcess(this.accountId)
  }


  public exportSelfKeys(path: string, passphrase: (string|null)): Promise<null> {
     return this.controller.rpc.exportSelfKeys(this.accountId, path, passphrase)
  }


  public importSelfKeys(path: string, passphrase: (string|null)): Promise<null> {
     return this.controller.rpc.importSelfKeys(this.accountId, path, passphrase)
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
  public getFreshMsgs(): Promise<(T.U32)[]> {
     return this.controller.rpc.getFreshMsgs(this.accountId)
  }

  /**
   * Get the number of _fresh_ messages in a chat.
   * Typically used to implement a badge with a number in the chatlist.
   *
   * If the specified chat is muted,
   * the UI should show the badge counter "less obtrusive",
   * e.g. using "gray" instead of "red" color.
   */
  public getFreshMsgCnt(chatId: T.U32): Promise<T.Usize> {
     return this.controller.rpc.getFreshMsgCnt(this.accountId, chatId)
  }

  /**
   * Estimate the number of messages that will be deleted
   * by the set_config()-options `delete_device_after` or `delete_server_after`.
   * This is typically used to show the estimated impact to the user
   * before actually enabling deletion of old messages.
   */
  public estimateAutoDeletionCount(fromServer: boolean, seconds: T.I64): Promise<T.Usize> {
     return this.controller.rpc.estimateAutoDeletionCount(this.accountId, fromServer, seconds)
  }


  public autocryptInitiateKeyTransfer(): Promise<string> {
     return this.controller.rpc.autocryptInitiateKeyTransfer(this.accountId)
  }


  public autocryptContinueKeyTransfer(messageId: T.U32, setupCode: string): Promise<null> {
     return this.controller.rpc.autocryptContinueKeyTransfer(this.accountId, messageId, setupCode)
  }


  public getChatlistEntries(listFlags: (T.U32|null), queryString: (string|null), queryContactId: (T.U32|null)): Promise<(T.ChatListEntry)[]> {
     return this.controller.rpc.getChatlistEntries(this.accountId, listFlags, queryString, queryContactId)
  }


  public getChatlistItemsByEntries(entries: (T.ChatListEntry)[]): Promise<Record<T.U32,T.ChatListItemFetchResult>> {
     return this.controller.rpc.getChatlistItemsByEntries(this.accountId, entries)
  }


  public chatlistGetFullChatById(chatId: T.U32): Promise<T.FullChat> {
     return this.controller.rpc.chatlistGetFullChatById(this.accountId, chatId)
  }

  /**
   * get basic info about a chat,
   * use chatlist_get_full_chat_by_id() instead if you need more information
   */
  public getBasicChatInfo(chatId: T.U32): Promise<T.BasicChat> {
     return this.controller.rpc.getBasicChatInfo(this.accountId, chatId)
  }


  public acceptChat(chatId: T.U32): Promise<null> {
     return this.controller.rpc.acceptChat(this.accountId, chatId)
  }


  public blockChat(chatId: T.U32): Promise<null> {
     return this.controller.rpc.blockChat(this.accountId, chatId)
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
  public deleteChat(chatId: T.U32): Promise<null> {
     return this.controller.rpc.deleteChat(this.accountId, chatId)
  }

  /**
   * Get encryption info for a chat.
   * Get a multi-line encryption info, containing encryption preferences of all members.
   * Can be used to find out why messages sent to group are not encrypted.
   *
   * returns Multi-line text
   */
  public getChatEncryptionInfo(chatId: T.U32): Promise<string> {
     return this.controller.rpc.getChatEncryptionInfo(this.accountId, chatId)
  }

  /**
   * Get QR code (text and SVG) that will offer an Setup-Contact or Verified-Group invitation.
   * The QR code is compatible to the OPENPGP4FPR format
   * so that a basic fingerprint comparison also works e.g. with OpenKeychain.
   *
   * The scanning device will pass the scanned content to `checkQr()` then;
   * if `checkQr()` returns `askVerifyContact` or `askVerifyGroup`
   * an out-of-band-verification can be joined using `secure_join()`
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
  public getChatSecurejoinQrCodeSvg(chatId: (T.U32|null)): Promise<[string,string]> {
     return this.controller.rpc.getChatSecurejoinQrCodeSvg(this.accountId, chatId)
  }

  /**
   * Continue a Setup-Contact or Verified-Group-Invite protocol
   * started on another device with `get_chat_securejoin_qr_code_svg()`.
   * This function is typically called when `check_qr()` returns
   * type=AskVerifyContact or type=AskVerifyGroup.
   *
   * The function returns immediately and the handshake runs in background,
   * sending and receiving several messages.
   * During the handshake, info messages are added to the chat,
   * showing progress, success or errors.
   *
   * Subsequent calls of `secure_join()` will abort previous, unfinished handshakes.
   *
   * See https://countermitm.readthedocs.io/en/latest/new.html
   * for details about both protocols.
   *
   * **qr**: The text of the scanned QR code. Typically, the same string as given
   *     to `check_qr()`.
   *
   * **returns**: The chat ID of the joined chat, the UI may redirect to the this chat.
   *         A returned chat ID does not guarantee that the chat is protected or the belonging contact is verified.
   *
   */
  public secureJoin(qr: string): Promise<T.U32> {
     return this.controller.rpc.secureJoin(this.accountId, qr)
  }


  public leaveGroup(chatId: T.U32): Promise<null> {
     return this.controller.rpc.leaveGroup(this.accountId, chatId)
  }

  /**
   * Remove a member from a group.
   *
   * If the group is already _promoted_ (any message was sent to the group),
   * all group members are informed by a special status message that is sent automatically by this function.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
   */
  public removeContactFromChat(chatId: T.U32, contactId: T.U32): Promise<null> {
     return this.controller.rpc.removeContactFromChat(this.accountId, chatId, contactId)
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
  public addContactToChat(chatId: T.U32, contactId: T.U32): Promise<null> {
     return this.controller.rpc.addContactToChat(this.accountId, chatId, contactId)
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
  public getChatContacts(chatId: T.U32): Promise<(T.U32)[]> {
     return this.controller.rpc.getChatContacts(this.accountId, chatId)
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
  public createGroupChat(name: string, protect: boolean): Promise<T.U32> {
     return this.controller.rpc.createGroupChat(this.accountId, name, protect)
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
  public createBroadcastList(): Promise<T.U32> {
     return this.controller.rpc.createBroadcastList(this.accountId)
  }

  /**
   * Set group name.
   *
   * If the group is already _promoted_ (any message was sent to the group),
   * all group members are informed by a special status message that is sent automatically by this function.
   *
   * Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a status message was sent.
   */
  public setChatName(chatId: T.U32, newName: string): Promise<null> {
     return this.controller.rpc.setChatName(this.accountId, chatId, newName)
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
  public setChatProfileImage(chatId: T.U32, imagePath: (string|null)): Promise<null> {
     return this.controller.rpc.setChatProfileImage(this.accountId, chatId, imagePath)
  }


  public setChatVisibility(chatId: T.U32, visibility: T.ChatVisibility): Promise<null> {
     return this.controller.rpc.setChatVisibility(this.accountId, chatId, visibility)
  }


  public setChatEphemeralTimer(chatId: T.U32, timer: T.U32): Promise<null> {
     return this.controller.rpc.setChatEphemeralTimer(this.accountId, chatId, timer)
  }


  public getChatEphemeralTimer(chatId: T.U32): Promise<T.U32> {
     return this.controller.rpc.getChatEphemeralTimer(this.accountId, chatId)
  }


  public addDeviceMessage(label: string, text: string): Promise<T.U32> {
     return this.controller.rpc.addDeviceMessage(this.accountId, label, text)
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
  public marknoticedChat(chatId: T.U32): Promise<null> {
     return this.controller.rpc.marknoticedChat(this.accountId, chatId)
  }


  public getFirstUnreadMessageOfChat(chatId: T.U32): Promise<(T.U32|null)> {
     return this.controller.rpc.getFirstUnreadMessageOfChat(this.accountId, chatId)
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
  public setChatMuteDuration(chatId: T.U32, duration: T.MuteDuration): Promise<null> {
     return this.controller.rpc.setChatMuteDuration(this.accountId, chatId, duration)
  }

  /**
   * Check whether the chat is currently muted (can be changed by set_chat_mute_duration()).
   *
   * This is available as a standalone function outside of fullchat, because it might be only needed for notification
   */
  public isChatMuted(chatId: T.U32): Promise<boolean> {
     return this.controller.rpc.isChatMuted(this.accountId, chatId)
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
  public markseenMsgs(msgIds: (T.U32)[]): Promise<null> {
     return this.controller.rpc.markseenMsgs(this.accountId, msgIds)
  }


  public getMessageIds(chatId: T.U32, flags: T.U32): Promise<(T.U32)[]> {
     return this.controller.rpc.getMessageIds(this.accountId, chatId, flags)
  }


  public getMessageListItems(chatId: T.U32, flags: T.U32): Promise<(T.MessageListItem)[]> {
     return this.controller.rpc.getMessageListItems(this.accountId, chatId, flags)
  }


  public messageGetMessage(messageId: T.U32): Promise<T.Message> {
     return this.controller.rpc.messageGetMessage(this.accountId, messageId)
  }


  public getMessageHtml(messageId: T.U32): Promise<(string|null)> {
     return this.controller.rpc.getMessageHtml(this.accountId, messageId)
  }


  public messageGetMessages(messageIds: (T.U32)[]): Promise<Record<T.U32,T.Message>> {
     return this.controller.rpc.messageGetMessages(this.accountId, messageIds)
  }

  /**
   * Fetch info desktop needs for creating a notification for a message
   */
  public messageGetNotificationInfo(messageId: T.U32): Promise<T.MessageNotificationInfo> {
     return this.controller.rpc.messageGetNotificationInfo(this.accountId, messageId)
  }

  /**
   * Delete messages. The messages are deleted on the current device and
   * on the IMAP server.
   */
  public deleteMessages(messageIds: (T.U32)[]): Promise<null> {
     return this.controller.rpc.deleteMessages(this.accountId, messageIds)
  }

  /**
   * Get an informational text for a single message. The text is multiline and may
   * contain e.g. the raw text of the message.
   *
   * The max. text returned is typically longer (about 100000 characters) than the
   * max. text returned by dc_msg_get_text() (about 30000 characters).
   */
  public getMessageInfo(messageId: T.U32): Promise<string> {
     return this.controller.rpc.getMessageInfo(this.accountId, messageId)
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
  public downloadFullMessage(messageId: T.U32): Promise<null> {
     return this.controller.rpc.downloadFullMessage(this.accountId, messageId)
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
  public searchMessages(query: string, chatId: (T.U32|null)): Promise<(T.U32)[]> {
     return this.controller.rpc.searchMessages(this.accountId, query, chatId)
  }


  public messageIdsToSearchResults(messageIds: (T.U32)[]): Promise<Record<T.U32,T.MessageSearchResult>> {
     return this.controller.rpc.messageIdsToSearchResults(this.accountId, messageIds)
  }

  /**
   * Get a single contact options by ID.
   */
  public contactsGetContact(contactId: T.U32): Promise<T.Contact> {
     return this.controller.rpc.contactsGetContact(this.accountId, contactId)
  }

  /**
   * Add a single contact as a result of an explicit user action.
   *
   * Returns contact id of the created or existing contact
   */
  public contactsCreateContact(email: string, name: (string|null)): Promise<T.U32> {
     return this.controller.rpc.contactsCreateContact(this.accountId, email, name)
  }

  /**
   * Returns contact id of the created or existing DM chat with that contact
   */
  public contactsCreateChatByContactId(contactId: T.U32): Promise<T.U32> {
     return this.controller.rpc.contactsCreateChatByContactId(this.accountId, contactId)
  }


  public contactsBlock(contactId: T.U32): Promise<null> {
     return this.controller.rpc.contactsBlock(this.accountId, contactId)
  }


  public contactsUnblock(contactId: T.U32): Promise<null> {
     return this.controller.rpc.contactsUnblock(this.accountId, contactId)
  }


  public contactsGetBlocked(): Promise<(T.Contact)[]> {
     return this.controller.rpc.contactsGetBlocked(this.accountId)
  }


  public contactsGetContactIds(listFlags: T.U32, query: (string|null)): Promise<(T.U32)[]> {
     return this.controller.rpc.contactsGetContactIds(this.accountId, listFlags, query)
  }

  /**
   * Get a list of contacts.
   * (formerly called getContacts2 in desktop)
   */
  public contactsGetContacts(listFlags: T.U32, query: (string|null)): Promise<(T.Contact)[]> {
     return this.controller.rpc.contactsGetContacts(this.accountId, listFlags, query)
  }


  public contactsGetContactsByIds(ids: (T.U32)[]): Promise<Record<T.U32,T.Contact>> {
     return this.controller.rpc.contactsGetContactsByIds(this.accountId, ids)
  }


  public deleteContact(contactId: T.U32): Promise<boolean> {
     return this.controller.rpc.deleteContact(this.accountId, contactId)
  }


  public changeContactName(contactId: T.U32, name: string): Promise<null> {
     return this.controller.rpc.changeContactName(this.accountId, contactId, name)
  }

  /**
   * Get encryption info for a contact.
   * Get a multi-line encryption info, containing your fingerprint and the
   * fingerprint of the contact, used e.g. to compare the fingerprints for a simple out-of-band verification.
   */
  public getContactEncryptionInfo(contactId: T.U32): Promise<string> {
     return this.controller.rpc.getContactEncryptionInfo(this.accountId, contactId)
  }

  /**
   * Check if an e-mail address belongs to a known and unblocked contact.
   * To get a list of all known and unblocked contacts, use contacts_get_contacts().
   *
   * To validate an e-mail address independently of the contact database
   * use check_email_validity().
   */
  public lookupContactIdByAddr(addr: string): Promise<(T.U32|null)> {
     return this.controller.rpc.lookupContactIdByAddr(this.accountId, addr)
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
  public chatGetMedia(chatId: (T.U32|null), messageType: T.Viewtype, orMessageType2: (T.Viewtype|null), orMessageType3: (T.Viewtype|null)): Promise<(T.U32)[]> {
     return this.controller.rpc.chatGetMedia(this.accountId, chatId, messageType, orMessageType2, orMessageType3)
  }

  /**
   * Search next/previous message based on a given message and a list of types.
   * Typically used to implement the "next" and "previous" buttons
   * in a gallery or in a media player.
   *
   * one combined call for getting chat::get_next_media for both directions
   * the manual chat::get_next_media in only one direction is not exposed by the jsonrpc yet
   */
  public chatGetNeighboringMedia(msgId: T.U32, messageType: T.Viewtype, orMessageType2: (T.Viewtype|null), orMessageType3: (T.Viewtype|null)): Promise<[(T.U32|null),(T.U32|null)]> {
     return this.controller.rpc.chatGetNeighboringMedia(this.accountId, msgId, messageType, orMessageType2, orMessageType3)
  }


  public exportBackup(destination: string, passphrase: (string|null)): Promise<null> {
     return this.controller.rpc.exportBackup(this.accountId, destination, passphrase)
  }


  public importBackup(path: string, passphrase: (string|null)): Promise<null> {
     return this.controller.rpc.importBackup(this.accountId, path, passphrase)
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
  public getConnectivity(): Promise<T.U32> {
     return this.controller.rpc.getConnectivity(this.accountId)
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
  public getConnectivityHtml(): Promise<string> {
     return this.controller.rpc.getConnectivityHtml(this.accountId)
  }


  public getLocations(chatId: (T.U32|null), contactId: (T.U32|null), timestampBegin: T.I64, timestampEnd: T.I64): Promise<(T.Location)[]> {
     return this.controller.rpc.getLocations(this.accountId, chatId, contactId, timestampBegin, timestampEnd)
  }


  public webxdcSendStatusUpdate(instanceMsgId: T.U32, updateStr: string, description: string): Promise<null> {
     return this.controller.rpc.webxdcSendStatusUpdate(this.accountId, instanceMsgId, updateStr, description)
  }


  public webxdcGetStatusUpdates(instanceMsgId: T.U32, lastKnownSerial: T.U32): Promise<string> {
     return this.controller.rpc.webxdcGetStatusUpdates(this.accountId, instanceMsgId, lastKnownSerial)
  }

  /**
   * Get info from a webxdc message
   */
  public messageGetWebxdcInfo(instanceMsgId: T.U32): Promise<T.WebxdcMessageInfo> {
     return this.controller.rpc.messageGetWebxdcInfo(this.accountId, instanceMsgId)
  }

  /**
   * Forward messages to another chat.
   *
   * All types of messages can be forwarded,
   * however, they will be flagged as such (dc_msg_is_forwarded() is set).
   *
   * Original sender, info-state and webxdc updates are not forwarded on purpose.
   */
  public forwardMessages(messageIds: (T.U32)[], chatId: T.U32): Promise<null> {
     return this.controller.rpc.forwardMessages(this.accountId, messageIds, chatId)
  }


  public sendSticker(chatId: T.U32, stickerPath: string): Promise<T.U32> {
     return this.controller.rpc.sendSticker(this.accountId, chatId, stickerPath)
  }

  /**
   * Send a reaction to message.
   *
   * Reaction is a string of emojis separated by spaces. Reaction to a
   * single message can be sent multiple times. The last reaction
   * received overrides all previously received reactions. It is
   * possible to remove all reactions by sending an empty string.
   */
  public sendReaction(messageId: T.U32, reaction: (string)[]): Promise<T.U32> {
     return this.controller.rpc.sendReaction(this.accountId, messageId, reaction)
  }


  public removeDraft(chatId: T.U32): Promise<null> {
     return this.controller.rpc.removeDraft(this.accountId, chatId)
  }

  /**
   *  Get draft for a chat, if any.
   */
  public getDraft(chatId: T.U32): Promise<(T.Message|null)> {
     return this.controller.rpc.getDraft(this.accountId, chatId)
  }


  public sendVideochatInvitation(chatId: T.U32): Promise<T.U32> {
     return this.controller.rpc.sendVideochatInvitation(this.accountId, chatId)
  }


  public miscGetStickerFolder(): Promise<string> {
     return this.controller.rpc.miscGetStickerFolder(this.accountId)
  }

  /**
   * for desktop, get stickers from stickers folder,
   * grouped by the folder they are in.
   */
  public miscGetStickers(): Promise<Record<string,(string)[]>> {
     return this.controller.rpc.miscGetStickers(this.accountId)
  }

  /**
   * Returns the messageid of the sent message
   */
  public miscSendTextMessage(text: string, chatId: T.U32): Promise<T.U32> {
     return this.controller.rpc.miscSendTextMessage(this.accountId, text, chatId)
  }


  public miscSendMsg(chatId: T.U32, text: (string|null), file: (string|null), location: ([T.F64,T.F64]|null), quotedMessageId: (T.U32|null)): Promise<[T.U32,T.Message]> {
     return this.controller.rpc.miscSendMsg(this.accountId, chatId, text, file, location, quotedMessageId)
  }


  public miscSetDraft(chatId: T.U32, text: (string|null), file: (string|null), quotedMessageId: (T.U32|null)): Promise<null> {
     return this.controller.rpc.miscSetDraft(this.accountId, chatId, text, file, quotedMessageId)
  }


}
