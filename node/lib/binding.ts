import { join } from 'path'
import * as url from 'url'

/**
 * bindings are not typed yet.
 * if the available function names are required they can be found inside of `../src/module.c`
 */
import build from 'node-gyp-build'
export const bindings: any = build(
  join(url.fileURLToPath(new URL('.', import.meta.url)), '../')
)

export default bindings
