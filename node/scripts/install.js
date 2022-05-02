const {execSync} = require('child_process')
const {existsSync} = require('fs')
const {join} = require('path')

const run = (cmd) => {
  console.log('[i] running `' + cmd + '`')
  execSync(cmd, {stdio: 'inherit'})
}

// Build bindings
if (process.env.USE_SYSTEM_LIBDELTACHAT === 'true') {
  console.log('[i] USE_SYSTEM_LIBDELTACHAT is true, rebuilding c bindings and using pkg-config to retrieve lib paths and cflags of libdeltachat')
  run('npm run build:bindings:c:c')
} else {
  console.log('[i] Building rust core & c bindings, if possible use prebuilds')
  run('npm run install:prebuilds')
}

if (!existsSync(join(__dirname, '..', 'dist'))) {
  console.log('[i] Didn\'t find already built typescript bindings. Trying to transpile them. If this fail, make sure typescript is installed ;)')
  run('npm run build:bindings:ts')
}
