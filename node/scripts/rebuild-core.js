const path = require('path')
const { spawn } = require('./common')
const opts = {
  cwd: path.resolve(__dirname, '../..'),
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
