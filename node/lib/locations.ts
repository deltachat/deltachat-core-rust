/* eslint-disable camelcase */

const binding = require('../binding')
const debug = require('debug')('deltachat:node:locations')

interface NativeLocations {}
/**
 * Wrapper around dc_location_t*
 */
export class Locations {
  constructor(public dc_locations: NativeLocations) {
    debug('Locations constructor')
  }

  locationToJson(index: number) {
    debug('locationToJson')
    return {
      accuracy: this.getAccuracy(index),
      latitude: this.getLatitude(index),
      longitude: this.getLongitude(index),
      timestamp: this.getTimestamp(index),
      contactId: this.getContactId(index),
      msgId: this.getMsgId(index),
      chatId: this.getChatId(index),
      isIndependent: this.isIndependent(index),
      marker: this.getMarker(index),
    }
  }

  toJson(): ReturnType<Locations['locationToJson']>[] {
    debug('toJson')
    const locations = []
    const count = this.getCount()
    for (let index = 0; index < count; index++) {
      locations.push(this.locationToJson(index))
    }
    return locations
  }

  getCount(): number {
    return binding.dcn_array_get_cnt(this.dc_locations)
  }

  getAccuracy(index: number): number {
    return binding.dcn_array_get_accuracy(this.dc_locations, index)
  }

  getLatitude(index: number): number {
    return binding.dcn_array_get_latitude(this.dc_locations, index)
  }

  getLongitude(index: number): number {
    return binding.dcn_array_get_longitude(this.dc_locations, index)
  }

  getTimestamp(index: number): number {
    return binding.dcn_array_get_timestamp(this.dc_locations, index)
  }

  getMsgId(index: number): number {
    return binding.dcn_array_get_msg_id(this.dc_locations, index)
  }

  getContactId(index: number): number {
    return binding.dcn_array_get_contact_id(this.dc_locations, index)
  }

  getChatId(index: number): number {
    return binding.dcn_array_get_chat_id(this.dc_locations, index)
  }

  isIndependent(index: number): boolean {
    return binding.dcn_array_is_independent(this.dc_locations, index)
  }

  getMarker(index: number): string {
    return binding.dcn_array_get_marker(this.dc_locations, index)
  }
}
