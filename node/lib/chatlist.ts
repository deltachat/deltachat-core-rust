/* eslint-disable camelcase */

import binding from './binding.js'
import { Lot } from './lot.js'
import { Chat } from './chat.js'
import rawDebug from 'debug'
const debug = rawDebug('deltachat:node:chatlist')

interface NativeChatList {}
/**
 * Wrapper around dc_chatlist_t*
 */
export class ChatList {
  constructor(private dc_chatlist: NativeChatList) {
    debug('ChatList constructor')
    if (dc_chatlist === null) {
      throw new Error('native chat list can not be null')
    }
  }

  getChatId(index: number): number {
    debug(`getChatId ${index}`)
    return binding.dcn_chatlist_get_chat_id(this.dc_chatlist, index)
  }

  getCount(): number {
    debug('getCount')
    return binding.dcn_chatlist_get_cnt(this.dc_chatlist)
  }

  getMessageId(index: number): number {
    debug(`getMessageId ${index}`)
    return binding.dcn_chatlist_get_msg_id(this.dc_chatlist, index)
  }

  getSummary(index: number, chat?: Chat): Lot {
    debug(`getSummary ${index}`)
    const dc_chat = (chat && chat.dc_chat) || null
    return new Lot(
      binding.dcn_chatlist_get_summary(this.dc_chatlist, index, dc_chat)
    )
  }
}
