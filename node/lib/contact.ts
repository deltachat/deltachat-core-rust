import { integerToHexColor } from './util'

/* eslint-disable camelcase */

import binding from './binding'
const debug = require('debug')('deltachat:node:contact')

interface NativeContact {}
/**
 * Wrapper around dc_contact_t*
 */
export class Contact {
  constructor(public dc_contact: NativeContact) {
    debug('Contact constructor')
  }

  toJson() {
    debug('toJson')
    return {
      address: this.getAddress(),
      color: this.color,
      authName: this.authName,
      status: this.status,
      displayName: this.getDisplayName(),
      id: this.getId(),
      lastSeen: this.lastSeen,
      name: this.getName(),
      profileImage: this.getProfileImage(),
      nameAndAddr: this.getNameAndAddress(),
      isBlocked: this.isBlocked(),
      isVerified: this.isVerified(),
    }
  }

  getAddress(): string {
    return binding.dcn_contact_get_addr(this.dc_contact)
  }

  /** Get original contact name.
   * This is the name of the contact as defined by the contact themself.
   * If the contact themself does not define such a name,
   * an empty string is returned. */
  get authName(): string {
    return binding.dcn_contact_get_auth_name(this.dc_contact)
  }

  get color(): string {
    return integerToHexColor(binding.dcn_contact_get_color(this.dc_contact))
  }

  /**
   * contact's status
   *
   * Status is the last signature received in a message from this contact.
   */
  get status(): string {
    return binding.dcn_contact_get_status(this.dc_contact)
  }

  getDisplayName(): string {
    return binding.dcn_contact_get_display_name(this.dc_contact)
  }

  getId(): number {
    return binding.dcn_contact_get_id(this.dc_contact)
  }

  get lastSeen(): number {
    return binding.dcn_contact_get_last_seen(this.dc_contact)
  }

  getName(): string {
    return binding.dcn_contact_get_name(this.dc_contact)
  }

  getNameAndAddress(): string {
    return binding.dcn_contact_get_name_n_addr(this.dc_contact)
  }

  getProfileImage(): string {
    return binding.dcn_contact_get_profile_image(this.dc_contact)
  }

  isBlocked() {
    return Boolean(binding.dcn_contact_is_blocked(this.dc_contact))
  }

  isVerified() {
    return Boolean(binding.dcn_contact_is_verified(this.dc_contact))
  }
}
