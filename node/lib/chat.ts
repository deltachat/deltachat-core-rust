/* eslint-disable camelcase */

import binding from './binding'
import rawDebug from 'debug'
const debug = rawDebug('deltachat:node:chat')
import { C } from './constants'
import { integerToHexColor } from './util'
import { ChatJSON } from './types'

interface NativeChat {}
/**
 * Wrapper around dc_chat_t*
 */

export class Chat {
  constructor(public dc_chat: NativeChat) {
    debug('Chat constructor')
  }

  getVisibility():
    | C.DC_CHAT_VISIBILITY_NORMAL
    | C.DC_CHAT_VISIBILITY_ARCHIVED
    | C.DC_CHAT_VISIBILITY_PINNED {
    return binding.dcn_chat_get_visibility(this.dc_chat)
  }

  get color(): string {
    return integerToHexColor(binding.dcn_chat_get_color(this.dc_chat))
  }

  getId(): number {
    return binding.dcn_chat_get_id(this.dc_chat)
  }

  getName(): string {
    return binding.dcn_chat_get_name(this.dc_chat)
  }

  getProfileImage(): string {
    return binding.dcn_chat_get_profile_image(this.dc_chat)
  }

  getType(): number {
    return binding.dcn_chat_get_type(this.dc_chat)
  }

  isSelfTalk(): boolean {
    return Boolean(binding.dcn_chat_is_self_talk(this.dc_chat))
  }

  isContactRequest(): boolean {
    return Boolean(binding.dcn_chat_is_contact_request(this.dc_chat))
  }

  isUnpromoted(): boolean {
    return Boolean(binding.dcn_chat_is_unpromoted(this.dc_chat))
  }

  isProtected(): boolean {
    return Boolean(binding.dcn_chat_is_protected(this.dc_chat))
  }

  get canSend(): boolean {
    return Boolean(binding.dcn_chat_can_send(this.dc_chat))
  }

  isDeviceTalk(): boolean {
    return Boolean(binding.dcn_chat_is_device_talk(this.dc_chat))
  }

  isSingle(): boolean {
    return this.getType() === C.DC_CHAT_TYPE_SINGLE
  }

  isGroup(): boolean {
    return this.getType() === C.DC_CHAT_TYPE_GROUP
  }

  isMuted(): boolean {
    return Boolean(binding.dcn_chat_is_muted(this.dc_chat))
  }

  toJson(): ChatJSON {
    debug('toJson')
    const visibility = this.getVisibility()
    return {
      archived: visibility === C.DC_CHAT_VISIBILITY_ARCHIVED,
      pinned: visibility === C.DC_CHAT_VISIBILITY_PINNED,
      color: this.color,
      id: this.getId(),
      name: this.getName(),
      profileImage: this.getProfileImage(),
      type: this.getType(),
      isSelfTalk: this.isSelfTalk(),
      isUnpromoted: this.isUnpromoted(),
      isProtected: this.isProtected(),
      canSend: this.canSend,
      isDeviceTalk: this.isDeviceTalk(),
      isContactRequest: this.isContactRequest(),
      muted: this.isMuted(),
    }
  }
}
