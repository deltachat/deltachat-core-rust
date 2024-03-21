import path from 'path'
import { spawn } from './common.js'
import * as url from 'url'
const opts = {
  cwd: path.resolve(url.fileURLToPath(new URL('.', import.meta.url)), '../..'),
  stdio: 'inherit'
}

const buildArgs = [
  'build',
  '--release',
  '--features',
  'vendored,jsonrpc',
  '-p',
  'deltachat_ffi'
]

spawn('cargo', buildArgs, opts)
