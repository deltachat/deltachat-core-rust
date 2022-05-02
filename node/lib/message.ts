/* eslint-disable camelcase */

import binding from './binding'
import { C } from './constants'
import { Lot } from './lot'
import { Chat } from './chat'
const debug = require('debug')('deltachat:node:message')

export enum MessageDownloadState {
  Available = C.DC_DOWNLOAD_AVAILABLE,
  Done = C.DC_DOWNLOAD_DONE,
  Failure = C.DC_DOWNLOAD_FAILURE,
  InProgress = C.DC_DOWNLOAD_IN_PROGRESS,
}

/**
 * Helper class for message states so you can do e.g.
 *
 * if (msg.getState().isPending()) { .. }
 *
 */
export class MessageState {
  constructor(public state: number) {
    debug(`MessageState constructor ${state}`)
  }

  isUndefined() {
    return this.state === C.DC_STATE_UNDEFINED
  }

  isFresh() {
    return this.state === C.DC_STATE_IN_FRESH
  }

  isNoticed() {
    return this.state === C.DC_STATE_IN_NOTICED
  }

  isSeen() {
    return this.state === C.DC_STATE_IN_SEEN
  }

  isPending() {
    return this.state === C.DC_STATE_OUT_PENDING
  }

  isFailed() {
    return this.state === C.DC_STATE_OUT_FAILED
  }

  isDelivered() {
    return this.state === C.DC_STATE_OUT_DELIVERED
  }

  isReceived() {
    return this.state === C.DC_STATE_OUT_MDN_RCVD
  }
}

/**
 * Helper class for message types so you can do e.g.
 *
 * if (msg.getViewType().isVideo()) { .. }
 *
 */
export class MessageViewType {
  constructor(public viewType: number) {
    debug(`MessageViewType constructor ${viewType}`)
  }

  isText() {
    return this.viewType === C.DC_MSG_TEXT
  }

  isImage() {
    return this.viewType === C.DC_MSG_IMAGE || this.viewType === C.DC_MSG_GIF
  }

  isGif() {
    return this.viewType === C.DC_MSG_GIF
  }

  isAudio() {
    return this.viewType === C.DC_MSG_AUDIO || this.viewType === C.DC_MSG_VOICE
  }

  isVoice() {
    return this.viewType === C.DC_MSG_VOICE
  }

  isVideo() {
    return this.viewType === C.DC_MSG_VIDEO
  }

  isFile() {
    return this.viewType === C.DC_MSG_FILE
  }

  isVideochatInvitation() {
    return this.viewType === C.DC_MSG_VIDEOCHAT_INVITATION
  }
}

interface NativeMessage {}
/**
 * Wrapper around dc_msg_t*
 */
export class Message {
  constructor(public dc_msg: NativeMessage) {
    debug('Message constructor')
  }

  toJson() {
    debug('toJson')
    const quotedMessage = this.getQuotedMessage()
    const viewType = binding.dcn_msg_get_viewtype(this.dc_msg)
    return {
      chatId: this.getChatId(),
      webxdcInfo: viewType == C.DC_MSG_WEBXDC ? this.webxdcInfo : null,
      downloadState: this.downloadState,
      duration: this.getDuration(),
      file: this.getFile(),
      fromId: this.getFromId(),
      id: this.getId(),
      quotedText: this.getQuotedText(),
      quotedMessageId: quotedMessage ? quotedMessage.getId() : null,
      receivedTimestamp: this.getReceivedTimestamp(),
      sortTimestamp: this.getSortTimestamp(),
      text: this.getText(),
      timestamp: this.getTimestamp(),
      hasLocation: this.hasLocation(),
      hasHTML: this.hasHTML,
      viewType,
      state: binding.dcn_msg_get_state(this.dc_msg),
      hasDeviatingTimestamp: this.hasDeviatingTimestamp(),
      showPadlock: this.getShowpadlock(),
      summary: this.getSummary().toJson(),
      subject: this.subject,
      isSetupmessage: this.isSetupmessage(),
      isInfo: this.isInfo(),
      isForwarded: this.isForwarded(),
      dimensions: {
        height: this.getHeight(),
        width: this.getWidth(),
      },
      videochatType: this.getVideochatType(),
      videochatUrl: this.getVideochatUrl(),
      overrideSenderName: this.overrideSenderName,
      parentId: this.parent?.getId(),
    }
  }

  getChatId(): number {
    return binding.dcn_msg_get_chat_id(this.dc_msg)
  }

  get webxdcInfo(): { name: string; icon: string; summary: string } | null {
    let info = binding.dcn_msg_get_webxdc_info(this.dc_msg)
    return info
      ? JSON.parse(binding.dcn_msg_get_webxdc_info(this.dc_msg))
      : null
  }

  get downloadState(): MessageDownloadState {
    return binding.dcn_msg_get_download_state(this.dc_msg)
  }

  get parent(): Message | null {
    let msg = binding.dcn_msg_get_parent(this.dc_msg)
    return msg ? new Message(msg) : null
  }

  getDuration(): number {
    return binding.dcn_msg_get_duration(this.dc_msg)
  }

  getFile(): string {
    return binding.dcn_msg_get_file(this.dc_msg)
  }

  getFilebytes(): number {
    return binding.dcn_msg_get_filebytes(this.dc_msg)
  }

  getFilemime(): string {
    return binding.dcn_msg_get_filemime(this.dc_msg)
  }

  getFilename(): string {
    return binding.dcn_msg_get_filename(this.dc_msg)
  }

  getFromId(): number {
    return binding.dcn_msg_get_from_id(this.dc_msg)
  }

  getHeight(): number {
    return binding.dcn_msg_get_height(this.dc_msg)
  }

  getId(): number {
    return binding.dcn_msg_get_id(this.dc_msg)
  }

  getQuotedText(): string {
    return binding.dcn_msg_get_quoted_text(this.dc_msg)
  }

  getQuotedMessage(): Message | null {
    const dc_msg = binding.dcn_msg_get_quoted_msg(this.dc_msg)
    return dc_msg ? new Message(dc_msg) : null
  }

  getReceivedTimestamp(): number {
    return binding.dcn_msg_get_received_timestamp(this.dc_msg)
  }

  getSetupcodebegin() {
    return binding.dcn_msg_get_setupcodebegin(this.dc_msg)
  }

  getShowpadlock() {
    return Boolean(binding.dcn_msg_get_showpadlock(this.dc_msg))
  }

  getSortTimestamp(): number {
    return binding.dcn_msg_get_sort_timestamp(this.dc_msg)
  }

  getState() {
    return new MessageState(binding.dcn_msg_get_state(this.dc_msg))
  }

  getSummary(chat?: Chat) {
    const dc_chat = (chat && chat.dc_chat) || null
    return new Lot(binding.dcn_msg_get_summary(this.dc_msg, dc_chat))
  }

  get subject(): string {
    return binding.dcn_msg_get_subject(this.dc_msg)
  }

  getSummarytext(approxCharacters: number): string {
    approxCharacters = approxCharacters || 0
    return binding.dcn_msg_get_summarytext(this.dc_msg, approxCharacters)
  }

  getText(): string {
    return binding.dcn_msg_get_text(this.dc_msg)
  }

  getTimestamp(): number {
    return binding.dcn_msg_get_timestamp(this.dc_msg)
  }

  getViewType() {
    return new MessageViewType(binding.dcn_msg_get_viewtype(this.dc_msg))
  }

  getVideochatType(): number {
    return binding.dcn_msg_get_videochat_type(this.dc_msg)
  }

  getVideochatUrl(): string {
    return binding.dcn_msg_get_videochat_url(this.dc_msg)
  }

  getWidth(): number {
    return binding.dcn_msg_get_width(this.dc_msg)
  }

  get overrideSenderName(): string {
    return binding.dcn_msg_get_override_sender_name(this.dc_msg)
  }

  hasDeviatingTimestamp() {
    return binding.dcn_msg_has_deviating_timestamp(this.dc_msg)
  }

  hasLocation() {
    return Boolean(binding.dcn_msg_has_location(this.dc_msg))
  }

  get hasHTML() {
    return Boolean(binding.dcn_msg_has_html(this.dc_msg))
  }

  isDeadDrop() {
    // TODO: Fix
    //return this.getChatId() === C.DC_CHAT_ID_DEADDROP
    return false
  }

  isForwarded() {
    return Boolean(binding.dcn_msg_is_forwarded(this.dc_msg))
  }

  isIncreation() {
    return Boolean(binding.dcn_msg_is_increation(this.dc_msg))
  }

  isInfo() {
    return Boolean(binding.dcn_msg_is_info(this.dc_msg))
  }

  isSent() {
    return Boolean(binding.dcn_msg_is_sent(this.dc_msg))
  }

  isSetupmessage() {
    return Boolean(binding.dcn_msg_is_setupmessage(this.dc_msg))
  }

  latefilingMediasize(width: number, height: number, duration: number) {
    binding.dcn_msg_latefiling_mediasize(this.dc_msg, width, height, duration)
  }

  setDimension(width: number, height: number) {
    binding.dcn_msg_set_dimension(this.dc_msg, width, height)
    return this
  }

  setDuration(duration: number) {
    binding.dcn_msg_set_duration(this.dc_msg, duration)
    return this
  }

  setFile(file: string, mime?: string) {
    if (typeof file !== 'string') throw new Error('Missing filename')
    binding.dcn_msg_set_file(this.dc_msg, file, mime || '')
    return this
  }

  setLocation(longitude: number, latitude: number) {
    binding.dcn_msg_set_location(this.dc_msg, longitude, latitude)
    return this
  }

  setQuote(quotedMessage: Message | null) {
    binding.dcn_msg_set_quote(this.dc_msg, quotedMessage?.dc_msg)
    return this
  }

  setText(text: string) {
    binding.dcn_msg_set_text(this.dc_msg, text)
    return this
  }

  setHTML(html: string) {
    binding.dcn_msg_set_html(this.dc_msg, html)
    return this
  }

  setOverrideSenderName(senderName: string) {
    binding.dcn_msg_set_override_sender_name(this.dc_msg, senderName)
    return this
  }

  /** Force the message to be sent in plain text.
   *
   * This API is for bots, there is no need to expose it in the UI.
   */
  forcePlaintext() {
    binding.dcn_msg_force_plaintext(this.dc_msg)
  }
}
