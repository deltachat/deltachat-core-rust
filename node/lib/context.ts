/* eslint-disable camelcase */

import binding from './binding'
import { C, EventId2EventName } from './constants'
import { Chat } from './chat'
import { ChatList } from './chatlist'
import { Contact } from './contact'
import { Message } from './message'
import { Lot } from './lot'
import { Locations } from './locations'
import rawDebug from 'debug'
import { AccountManager } from './deltachat'
import { join } from 'path'
import { EventEmitter } from 'stream'
const debug = rawDebug('deltachat:node:index')

const noop = function () {}
interface NativeContext {}

/**
 * Wrapper around dcn_context_t*
 *
 * only acts as event emitter when created in standalone mode (without account manager)
 * with `Context.open`
 */
export class Context extends EventEmitter {
  constructor(
    readonly manager: AccountManager | null,
    private inner_dcn_context: NativeContext,
    readonly account_id: number | null
  ) {
    super()
    debug('DeltaChat constructor')
  }

  /** Opens a stanalone context (without an account manager)
   * automatically starts the event handler */
  static open(cwd: string): Context {
    const dbFile = join(cwd, 'db.sqlite')
    const context = new Context(null, binding.dcn_context_new(dbFile), null)
    debug('Opened context')
    function handleCoreEvent(
      eventId: number,
      data1: number,
      data2: number | string
    ) {
      const eventString = EventId2EventName[eventId]
      debug(eventString, data1, data2)
      if (!context.emit) {
        console.log('Received an event but EventEmitter is already destroyed.')
        console.log(eventString, data1, data2)
        return
      }
      context.emit(eventString, data1, data2)
      context.emit('ALL', eventString, data1, data2)
    }
    binding.dcn_start_event_handler(
      context.dcn_context,
      handleCoreEvent.bind(this)
    )
    debug('Started event handler')
    return context
  }

  get dcn_context() {
    return this.inner_dcn_context
  }

  get is_open() {
    return Boolean(binding.dcn_context_is_open())
  }

  open(passphrase?: string) {
    return Boolean(
      binding.dcn_context_open(this.dcn_context, passphrase ? passphrase : '')
    )
  }

  unref() {
    binding.dcn_context_unref(this.dcn_context)
    ;(this.inner_dcn_context as any) = null
  }

  acceptChat(chatId: number) {
    binding.dcn_accept_chat(this.dcn_context, chatId)
  }

  blockChat(chatId: number) {
    binding.dcn_block_chat(this.dcn_context, chatId)
  }

  addAddressBook(addressBook: string) {
    debug(`addAddressBook ${addressBook}`)
    return binding.dcn_add_address_book(this.dcn_context, addressBook)
  }

  addContactToChat(chatId: number, contactId: number) {
    debug(`addContactToChat ${chatId} ${contactId}`)
    return Boolean(
      binding.dcn_add_contact_to_chat(
        this.dcn_context,
        Number(chatId),
        Number(contactId)
      )
    )
  }

  addDeviceMessage(label: string, msg: Message | string) {
    debug(`addDeviceMessage ${label} ${msg}`)
    if (!msg) {
      throw new Error('invalid msg parameter')
    }
    if (typeof label !== 'string') {
      throw new Error('invalid label parameter, must be a string')
    }
    if (typeof msg === 'string') {
      const msgObj = this.messageNew()
      msgObj.setText(msg)
      msg = msgObj
    }
    if (!msg.dc_msg) {
      throw new Error('invalid msg object')
    }
    return binding.dcn_add_device_msg(this.dcn_context, label, msg.dc_msg)
  }

  setChatVisibility(
    chatId: number,
    visibility:
      | C.DC_CHAT_VISIBILITY_NORMAL
      | C.DC_CHAT_VISIBILITY_ARCHIVED
      | C.DC_CHAT_VISIBILITY_PINNED
  ) {
    debug(`setChatVisibility ${chatId} ${visibility}`)
    binding.dcn_set_chat_visibility(
      this.dcn_context,
      Number(chatId),
      visibility
    )
  }

  blockContact(contactId: number, block: boolean) {
    debug(`blockContact ${contactId} ${block}`)
    binding.dcn_block_contact(
      this.dcn_context,
      Number(contactId),
      block ? 1 : 0
    )
  }

  checkQrCode(qrCode: string) {
    debug(`checkQrCode ${qrCode}`)
    const dc_lot = binding.dcn_check_qr(this.dcn_context, qrCode)
    let result = dc_lot ? new Lot(dc_lot) : null
    if (result) {
      return { id: result.getId(), ...result.toJson() }
    }
    return result
  }

  configure(opts: any): Promise<void> {
    return new Promise((resolve, reject) => {
      debug('configure')

      const onSuccess = () => {
        removeListeners()
        resolve()
      }
      const onFail = (error: string) => {
        removeListeners()
        reject(new Error(error))
      }

      let onConfigure: (...args: any[]) => void
      if (this.account_id === null) {
        onConfigure = (data1: number, data2: string) => {
          if (data1 === 0) return onFail(data2)
          else if (data1 === 1000) return onSuccess()
        }
      } else {
        onConfigure = (accountId: number, data1: number, data2: string) => {
          if (this.account_id !== accountId) {
            return
          }
          if (data1 === 0) return onFail(data2)
          else if (data1 === 1000) return onSuccess()
        }
      }

      const removeListeners = () => {
        ;(this.manager || this).removeListener(
          'DC_EVENT_CONFIGURE_PROGRESS',
          onConfigure
        )
      }

      const registerListeners = () => {
        ;(this.manager || this).on('DC_EVENT_CONFIGURE_PROGRESS', onConfigure)
      }

      registerListeners()

      if (!opts) opts = {}
      Object.keys(opts).forEach((key) => {
        const value = opts[key]
        this.setConfig(key, value)
      })

      binding.dcn_configure(this.dcn_context)
    })
  }

  continueKeyTransfer(messageId: number, setupCode: string) {
    debug(`continueKeyTransfer ${messageId}`)
    return new Promise((resolve, reject) => {
      binding.dcn_continue_key_transfer(
        this.dcn_context,
        Number(messageId),
        setupCode,
        (result: number) => resolve(result === 1)
      )
    })
  }
  /** @returns chatId */
  createBroadcastList(): number {
    debug(`createBroadcastList`)
    return binding.dcn_create_broadcast_list(this.dcn_context)
  }

  /** @returns chatId */
  createChatByContactId(contactId: number): number {
    debug(`createChatByContactId ${contactId}`)
    return binding.dcn_create_chat_by_contact_id(
      this.dcn_context,
      Number(contactId)
    )
  }

  /** @returns contactId */
  createContact(name: string, addr: string): number {
    debug(`createContact ${name} ${addr}`)
    return binding.dcn_create_contact(this.dcn_context, name, addr)
  }

  /**
   *
   * @param chatName The name of the chat that should be created
   * @param is_protected Whether the chat should be protected at creation time
   * @returns chatId
   */
  createGroupChat(chatName: string, is_protected: boolean = false): number {
    debug(`createGroupChat ${chatName} [protected:${is_protected}]`)
    return binding.dcn_create_group_chat(
      this.dcn_context,
      is_protected ? 1 : 0,
      chatName
    )
  }

  deleteChat(chatId: number) {
    debug(`deleteChat ${chatId}`)
    binding.dcn_delete_chat(this.dcn_context, Number(chatId))
  }

  deleteContact(contactId: number) {
    debug(`deleteContact ${contactId}`)
    return Boolean(
      binding.dcn_delete_contact(this.dcn_context, Number(contactId))
    )
  }

  deleteMessages(messageIds: number[]) {
    if (!Array.isArray(messageIds)) {
      messageIds = [messageIds]
    }
    messageIds = messageIds.map((id) => Number(id))
    debug('deleteMessages', messageIds)
    binding.dcn_delete_msgs(this.dcn_context, messageIds)
  }

  forwardMessages(messageIds: number[], chatId: number) {
    if (!Array.isArray(messageIds)) {
      messageIds = [messageIds]
    }
    messageIds = messageIds.map((id) => Number(id))
    debug('forwardMessages', messageIds)
    binding.dcn_forward_msgs(this.dcn_context, messageIds, chatId)
  }

  getBlobdir(): string {
    debug('getBlobdir')
    return binding.dcn_get_blobdir(this.dcn_context)
  }

  getBlockedCount(): number {
    debug('getBlockedCount')
    return binding.dcn_get_blocked_cnt(this.dcn_context)
  }

  getBlockedContacts(): number[] {
    debug('getBlockedContacts')
    return binding.dcn_get_blocked_contacts(this.dcn_context)
  }

  getChat(chatId: number) {
    debug(`getChat ${chatId}`)
    const dc_chat = binding.dcn_get_chat(this.dcn_context, Number(chatId))
    return dc_chat ? new Chat(dc_chat) : null
  }

  getChatContacts(chatId: number): number[] {
    debug(`getChatContacts ${chatId}`)
    return binding.dcn_get_chat_contacts(this.dcn_context, Number(chatId))
  }

  getChatIdByContactId(contactId: number): number {
    debug(`getChatIdByContactId ${contactId}`)
    return binding.dcn_get_chat_id_by_contact_id(
      this.dcn_context,
      Number(contactId)
    )
  }

  getChatMedia(
    chatId: number,
    msgType1: number,
    msgType2: number,
    msgType3: number
  ): number[] {
    debug(`getChatMedia ${chatId}`)
    return binding.dcn_get_chat_media(
      this.dcn_context,
      Number(chatId),
      msgType1,
      msgType2 || 0,
      msgType3 || 0
    )
  }

  getMimeHeaders(messageId: number): string {
    debug(`getMimeHeaders ${messageId}`)
    return binding.dcn_get_mime_headers(this.dcn_context, Number(messageId))
  }

  getChatlistItemSummary(chatId: number, messageId: number) {
    debug(`getChatlistItemSummary ${chatId} ${messageId}`)
    return new Lot(
      binding.dcn_chatlist_get_summary2(this.dcn_context, chatId, messageId)
    )
  }

  getChatMessages(chatId: number, flags: number, marker1before: number) {
    debug(`getChatMessages ${chatId} ${flags} ${marker1before}`)
    return binding.dcn_get_chat_msgs(
      this.dcn_context,
      Number(chatId),
      flags,
      marker1before
    )
  }

  /**
   * Get encryption info for a chat.
   * Get a multi-line encryption info, containing encryption preferences of all members.
   * Can be used to find out why messages sent to group are not encrypted.
   *
   * @param chatId ID of the chat to get the encryption info for.
   * @return Multi-line text, must be released using dc_str_unref() after usage.
   */
  getChatEncrytionInfo(chatId: number): string {
    return binding.dcn_get_chat_encrinfo(this.dcn_context, chatId)
  }

  getChats(listFlags: number, queryStr: string, queryContactId: number) {
    debug('getChats')
    const result = []
    const list = this.getChatList(listFlags, queryStr, queryContactId)
    const count = list.getCount()
    for (let i = 0; i < count; i++) {
      result.push(list.getChatId(i))
    }
    return result
  }

  getChatList(listFlags: number, queryStr: string, queryContactId: number) {
    listFlags = listFlags || 0
    queryStr = queryStr || ''
    queryContactId = queryContactId || 0
    debug(`getChatList ${listFlags} ${queryStr} ${queryContactId}`)
    return new ChatList(
      binding.dcn_get_chatlist(
        this.dcn_context,
        listFlags,
        queryStr,
        Number(queryContactId)
      )
    )
  }

  getConfig(key: string): string {
    debug(`getConfig ${key}`)
    return binding.dcn_get_config(this.dcn_context, key)
  }

  getContact(contactId: number) {
    debug(`getContact ${contactId}`)
    const dc_contact = binding.dcn_get_contact(
      this.dcn_context,
      Number(contactId)
    )
    return dc_contact ? new Contact(dc_contact) : null
  }

  getContactEncryptionInfo(contactId: number) {
    debug(`getContactEncryptionInfo ${contactId}`)
    return binding.dcn_get_contact_encrinfo(this.dcn_context, Number(contactId))
  }

  getContacts(listFlags: number, query: string) {
    listFlags = listFlags || 0
    query = query || ''
    debug(`getContacts ${listFlags} ${query}`)
    return binding.dcn_get_contacts(this.dcn_context, listFlags, query)
  }

  wasDeviceMessageEverAdded(label: string) {
    debug(`wasDeviceMessageEverAdded ${label}`)
    const added = binding.dcn_was_device_msg_ever_added(this.dcn_context, label)
    return added === 1
  }

  getDraft(chatId: number) {
    debug(`getDraft ${chatId}`)
    const dc_msg = binding.dcn_get_draft(this.dcn_context, Number(chatId))
    return dc_msg ? new Message(dc_msg) : null
  }

  getFreshMessageCount(chatId: number): number {
    debug(`getFreshMessageCount ${chatId}`)
    return binding.dcn_get_fresh_msg_cnt(this.dcn_context, Number(chatId))
  }

  getFreshMessages() {
    debug('getFreshMessages')
    return binding.dcn_get_fresh_msgs(this.dcn_context)
  }

  getInfo() {
    debug('getInfo')
    const info = binding.dcn_get_info(this.dcn_context)
    return AccountManager.parseGetInfo(info)
  }

  getMessage(messageId: number) {
    debug(`getMessage ${messageId}`)
    const dc_msg = binding.dcn_get_msg(this.dcn_context, Number(messageId))
    return dc_msg ? new Message(dc_msg) : null
  }

  getMessageCount(chatId: number): number {
    debug(`getMessageCount ${chatId}`)
    return binding.dcn_get_msg_cnt(this.dcn_context, Number(chatId))
  }

  getMessageInfo(messageId: number): string {
    debug(`getMessageInfo ${messageId}`)
    return binding.dcn_get_msg_info(this.dcn_context, Number(messageId))
  }

  getMessageHTML(messageId: number): string {
    debug(`getMessageHTML ${messageId}`)
    return binding.dcn_get_msg_html(this.dcn_context, Number(messageId))
  }

  getNextMediaMessage(
    messageId: number,
    msgType1: number,
    msgType2: number,
    msgType3: number
  ) {
    debug(
      `getNextMediaMessage ${messageId} ${msgType1} ${msgType2} ${msgType3}`
    )
    return this._getNextMedia(messageId, 1, msgType1, msgType2, msgType3)
  }

  getPreviousMediaMessage(
    messageId: number,
    msgType1: number,
    msgType2: number,
    msgType3: number
  ) {
    debug(
      `getPreviousMediaMessage ${messageId} ${msgType1} ${msgType2} ${msgType3}`
    )
    return this._getNextMedia(messageId, -1, msgType1, msgType2, msgType3)
  }

  _getNextMedia(
    messageId: number,
    dir: number,
    msgType1: number,
    msgType2: number,
    msgType3: number
  ): number {
    return binding.dcn_get_next_media(
      this.dcn_context,
      Number(messageId),
      dir,
      msgType1 || 0,
      msgType2 || 0,
      msgType3 || 0
    )
  }

  getSecurejoinQrCode(chatId: number): string {
    debug(`getSecurejoinQrCode ${chatId}`)
    return binding.dcn_get_securejoin_qr(this.dcn_context, Number(chatId))
  }

  getSecurejoinQrCodeSVG(chatId: number): string {
    debug(`getSecurejoinQrCodeSVG ${chatId}`)
    return binding.dcn_get_securejoin_qr_svg(this.dcn_context, chatId)
  }

  startIO(): void {
    debug(`startIO`)
    binding.dcn_start_io(this.dcn_context)
  }

  stopIO(): void {
    debug(`stopIO`)
    binding.dcn_stop_io(this.dcn_context)
  }

  stopOngoingProcess(): void {
    debug(`stopOngoingProcess`)
    binding.dcn_stop_ongoing_process(this.dcn_context)
  }

  /**
   *
   * @deprectated please use `AccountManager.getSystemInfo()` instead
   */
  static getSystemInfo() {
    return AccountManager.getSystemInfo()
  }

  getConnectivity(): number {
    return binding.dcn_get_connectivity(this.dcn_context)
  }

  getConnectivityHTML(): String {
    return binding.dcn_get_connectivity_html(this.dcn_context)
  }

  importExport(what: number, param1: string, param2 = '') {
    debug(`importExport ${what} ${param1} ${param2}`)
    binding.dcn_imex(this.dcn_context, what, param1, param2)
  }

  importExportHasBackup(dir: string) {
    debug(`importExportHasBackup ${dir}`)
    return binding.dcn_imex_has_backup(this.dcn_context, dir)
  }

  initiateKeyTransfer(): Promise<string> {
    return new Promise((resolve, reject) => {
      debug('initiateKeyTransfer2')
      binding.dcn_initiate_key_transfer(this.dcn_context, resolve)
    })
  }

  isConfigured() {
    debug('isConfigured')
    return Boolean(binding.dcn_is_configured(this.dcn_context))
  }

  isContactInChat(chatId: number, contactId: number) {
    debug(`isContactInChat ${chatId} ${contactId}`)
    return Boolean(
      binding.dcn_is_contact_in_chat(
        this.dcn_context,
        Number(chatId),
        Number(contactId)
      )
    )
  }

  /**
   *
   * @returns resulting chat id or 0 on error
   */
  joinSecurejoin(qrCode: string): number {
    debug(`joinSecurejoin ${qrCode}`)
    return binding.dcn_join_securejoin(this.dcn_context, qrCode)
  }

  lookupContactIdByAddr(addr: string): number {
    debug(`lookupContactIdByAddr ${addr}`)
    return binding.dcn_lookup_contact_id_by_addr(this.dcn_context, addr)
  }

  markNoticedChat(chatId: number) {
    debug(`markNoticedChat ${chatId}`)
    binding.dcn_marknoticed_chat(this.dcn_context, Number(chatId))
  }

  markSeenMessages(messageIds: number[]) {
    if (!Array.isArray(messageIds)) {
      messageIds = [messageIds]
    }
    messageIds = messageIds.map((id) => Number(id))
    debug('markSeenMessages', messageIds)
    binding.dcn_markseen_msgs(this.dcn_context, messageIds)
  }

  maybeNetwork() {
    debug('maybeNetwork')
    binding.dcn_maybe_network(this.dcn_context)
  }

  messageNew(viewType = C.DC_MSG_TEXT) {
    debug(`messageNew ${viewType}`)
    return new Message(binding.dcn_msg_new(this.dcn_context, viewType))
  }

  removeContactFromChat(chatId: number, contactId: number) {
    debug(`removeContactFromChat ${chatId} ${contactId}`)
    return Boolean(
      binding.dcn_remove_contact_from_chat(
        this.dcn_context,
        Number(chatId),
        Number(contactId)
      )
    )
  }

  /**
   *
   * @param chatId 	ID of the chat to search messages in. Set this to 0 for a global search.
   * @param query The query to search for.
   */
  searchMessages(chatId: number, query: string): number[] {
    debug(`searchMessages ${chatId} ${query}`)
    return binding.dcn_search_msgs(this.dcn_context, Number(chatId), query)
  }

  sendMessage(chatId: number, msg: string | Message) {
    debug(`sendMessage ${chatId}`)
    if (!msg) {
      throw new Error('invalid msg parameter')
    }
    if (typeof msg === 'string') {
      const msgObj = this.messageNew()
      msgObj.setText(msg)
      msg = msgObj
    }
    if (!msg.dc_msg) {
      throw new Error('invalid msg object')
    }
    return binding.dcn_send_msg(this.dcn_context, Number(chatId), msg.dc_msg)
  }

  downloadFullMessage(messageId: number) {
    binding.dcn_download_full_msg(this.dcn_context, messageId)
  }

  /**
   *
   * @returns {Promise<number>} Promise that resolves into the resulting message id
   */
  sendVideochatInvitation(chatId: number): Promise<number> {
    debug(`sendVideochatInvitation ${chatId}`)
    return new Promise((resolve, reject) => {
      binding.dcn_send_videochat_invitation(
        this.dcn_context,
        chatId,
        (result: number) => {
          if (result !== 0) {
            resolve(result)
          } else {
            reject(
              'Videochatinvitation failed to send, see error events for detailed info'
            )
          }
        }
      )
    })
  }

  setChatName(chatId: number, name: string) {
    debug(`setChatName ${chatId} ${name}`)
    return Boolean(
      binding.dcn_set_chat_name(this.dcn_context, Number(chatId), name)
    )
  }

  /**
   *
   * @param chatId
   * @param protect
   * @returns success boolean
   */
  setChatProtection(chatId: number, protect: boolean) {
    debug(`setChatProtection ${chatId} ${protect}`)
    return Boolean(
      binding.dcn_set_chat_protection(
        this.dcn_context,
        Number(chatId),
        protect ? 1 : 0
      )
    )
  }

  getChatEphemeralTimer(chatId: number): number {
    debug(`getChatEphemeralTimer ${chatId}`)
    return binding.dcn_get_chat_ephemeral_timer(
      this.dcn_context,
      Number(chatId)
    )
  }

  setChatEphemeralTimer(chatId: number, timer: number) {
    debug(`setChatEphemeralTimer ${chatId} ${timer}`)
    return Boolean(
      binding.dcn_set_chat_ephemeral_timer(
        this.dcn_context,
        Number(chatId),
        Number(timer)
      )
    )
  }

  setChatProfileImage(chatId: number, image: string) {
    debug(`setChatProfileImage ${chatId} ${image}`)
    return Boolean(
      binding.dcn_set_chat_profile_image(
        this.dcn_context,
        Number(chatId),
        image || ''
      )
    )
  }

  setConfig(key: string, value: string | boolean | number): number {
    debug(`setConfig (string) ${key} ${value}`)
    if (value === null) {
      return binding.dcn_set_config_null(this.dcn_context, key)
    } else {
      if (typeof value === 'boolean') {
        value = value === true ? '1' : '0'
      } else if (typeof value === 'number') {
        value = String(value)
      }
      return binding.dcn_set_config(this.dcn_context, key, value)
    }
  }

  setConfigFromQr(qrcodeContent: string): boolean {
    return Boolean(
      binding.dcn_set_config_from_qr(this.dcn_context, qrcodeContent)
    )
  }

  estimateDeletionCount(fromServer: boolean, seconds: number): number {
    debug(`estimateDeletionCount fromServer: ${fromServer} seconds: ${seconds}`)
    return binding.dcn_estimate_deletion_cnt(
      this.dcn_context,
      fromServer === true ? 1 : 0,
      seconds
    )
  }

  setStockTranslation(stockId: number, stockMsg: string) {
    debug(`setStockTranslation ${stockId} ${stockMsg}`)
    return Boolean(
      binding.dcn_set_stock_translation(
        this.dcn_context,
        Number(stockId),
        stockMsg
      )
    )
  }

  setDraft(chatId: number, msg: Message | null) {
    debug(`setDraft ${chatId}`)
    binding.dcn_set_draft(
      this.dcn_context,
      Number(chatId),
      msg ? msg.dc_msg : null
    )
  }

  setLocation(latitude: number, longitude: number, accuracy: number) {
    debug(`setLocation ${latitude}`)
    binding.dcn_set_location(
      this.dcn_context,
      Number(latitude),
      Number(longitude),
      Number(accuracy)
    )
  }

  /*
   * @param chatId Chat-id to get location information for.
   *     0 to get locations independently of the chat.
   * @param contactId Contact id to get location information for.
   *     If also a chat-id is given, this should be a member of the given chat.
   *     0 to get locations independently of the contact.
   * @param timestampFrom Start of timespan to return.
   *     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
   *     0 for "start from the beginning".
   * @param timestampTo End of timespan to return.
   *     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
   *     0 for "all up to now".
   * @return Array of locations, NULL is never returned.
   *     The array is sorted decending;
   *     the first entry in the array is the location with the newest timestamp.
   *
   * Examples:
   * // get locations from the last hour for a global map
   * getLocations(0, 0, time(NULL)-60*60, 0);
   *
   * // get locations from a contact for a global map
   * getLocations(0, contact_id, 0, 0);
   *
   * // get all locations known for a given chat
   * getLocations(chat_id, 0, 0, 0);
   *
   * // get locations from a single contact for a given chat
   * getLocations(chat_id, contact_id, 0, 0);
   */

  getLocations(
    chatId: number,
    contactId: number,
    timestampFrom = 0,
    timestampTo = 0
  ) {
    const locations = new Locations(
      binding.dcn_get_locations(
        this.dcn_context,
        Number(chatId),
        Number(contactId),
        timestampFrom,
        timestampTo
      )
    )
    return locations.toJson()
  }

  /**
   *
   * @param duration The duration (0 for no mute, -1 for forever mute, everything else is is the relative mute duration from now in seconds)
   */
  setChatMuteDuration(chatId: number, duration: number) {
    return Boolean(
      binding.dcn_set_chat_mute_duration(this.dcn_context, chatId, duration)
    )
  }

  /** get information about the provider */
  getProviderFromEmail(email: string) {
    debug('DeltaChat.getProviderFromEmail')
    const provider = binding.dcn_provider_new_from_email(
      this.dcn_context,
      email
    )
    if (!provider) {
      return undefined
    }
    return {
      before_login_hint: binding.dcn_provider_get_before_login_hint(provider),
      overview_page: binding.dcn_provider_get_overview_page(provider),
      status: binding.dcn_provider_get_status(provider),
    }
  }

  sendWebxdcStatusUpdate<T>(
    msgId: number,
    json: WebxdcSendingStatusUpdate<T>,
    descr: string
  ) {
    return Boolean(
      binding.dcn_send_webxdc_status_update(
        this.dcn_context,
        msgId,
        JSON.stringify(json),
        descr
      )
    )
  }

  getWebxdcStatusUpdates<T>(
    msgId: number,
    serial = 0
  ): WebxdcReceivedStatusUpdate<T>[] {
    return JSON.parse(
      binding.dcn_get_webxdc_status_updates(this.dcn_context, msgId, serial)
    )
  }

  /** the string contains the binary data, it is an "u8 string", maybe we will use a more efficient type in the future. */
  getWebxdcBlob(message: Message, filename: string): Buffer | null {
    return binding.dcn_msg_get_webxdc_blob(message.dc_msg, filename)
  }
}

type WebxdcSendingStatusUpdate<T> = {
  /** the payload, deserialized json:
   * any javascript primitive, array or object. */
  payload: T
  /** optional, short, informational message that will be added to the chat,
   * eg. "Alice voted" or "Bob scored 123 in MyGame";
   * usually only one line of text is shown,
   * use this option sparingly to not spam the chat. */
  info?: string
  /** optional, short text, shown beside app icon;
   * it is recommended to use some aggregated value,
   * eg. "8 votes", "Highscore: 123" */
  summary?: string
}

type WebxdcReceivedStatusUpdate<T> = {
  /** the payload, deserialized json */
  payload: T
  /** the serial number of this update. Serials are larger `0` and newer serials have higher numbers. */
  serial: number
  /** the maximum serial currently known.
   *  If `max_serial` equals `serial` this update is the last update (until new network messages arrive). */
  max_serial: number
  /** optional, short, informational message. */
  info?: string
  /** optional, short text, shown beside app icon. If there are no updates, an empty JSON-array is returned. */
  summary?: string
}
