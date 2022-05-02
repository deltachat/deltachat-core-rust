const path = require('path')
const { spawn } = require('./common')
const opts = {
  cwd: path.resolve(__dirname, '../deltachat-core-rust'),
  stdio: 'inherit'
}

const buildArgs = [
  'build',
  '--release',
  '--features',
  'vendored',
  '-p',
  'deltachat_ffi'
]

spawn('cargo', buildArgs, opts)
