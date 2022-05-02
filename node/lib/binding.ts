import { join } from 'path'

/**
 * bindings are not typed yet.
 * if the availible function names are required they can be found inside of `../src/module.c`
 */
export const bindings: any = require('node-gyp-build')(join(__dirname, '../'))

export default bindings
