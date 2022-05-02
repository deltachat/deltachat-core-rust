/* eslint-disable camelcase */

const binding = require('../binding')
const debug = require('debug')('deltachat:node:lot')

interface NativeLot {}
/**
 * Wrapper around dc_lot_t*
 */
export class Lot {
  constructor(public dc_lot: NativeLot) {
    debug('Lot constructor')
  }

  toJson() {
    debug('toJson')
    return {
      state: this.getState(),
      text1: this.getText1(),
      text1Meaning: this.getText1Meaning(),
      text2: this.getText2(),
      timestamp: this.getTimestamp(),
    }
  }

  getId(): number {
    return binding.dcn_lot_get_id(this.dc_lot)
  }

  getState(): number {
    return binding.dcn_lot_get_state(this.dc_lot)
  }

  getText1(): string {
    return binding.dcn_lot_get_text1(this.dc_lot)
  }

  getText1Meaning(): string {
    return binding.dcn_lot_get_text1_meaning(this.dc_lot)
  }

  getText2(): string {
    return binding.dcn_lot_get_text2(this.dc_lot)
  }

  getTimestamp(): number {
    return binding.dcn_lot_get_timestamp(this.dc_lot)
  }
}
