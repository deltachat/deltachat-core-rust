import { C } from './constants'

export type ChatTypes =
  | C.DC_CHAT_TYPE_GROUP
  | C.DC_CHAT_TYPE_MAILINGLIST
  | C.DC_CHAT_TYPE_SINGLE
  | C.DC_CHAT_TYPE_UNDEFINED

export interface ChatJSON {
  archived: boolean
  pinned: boolean
  color: string
  id: number
  name: string
  profileImage: string
  type: number
  isSelfTalk: boolean
  isUnpromoted: boolean
  isProtected: boolean
  canSend: boolean
  isDeviceTalk: boolean
  isContactRequest: boolean
  muted: boolean
}
