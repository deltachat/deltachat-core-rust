const spawnSync = require('child_process').spawnSync

const verbose = isVerbose()

function spawn (cmd, args, opts) {
  log(`>> spawn: ${cmd} ${args.join(' ')}`)
  const result = spawnSync(cmd, args, opts)
  if (result.status === null) {
    console.error(`Could not find ${cmd}`)
    process.exit(1)
  } else if (result.status !== 0) {
    console.error(`${cmd} failed with code ${result.status}`)
    process.exit(1)
  }
}

function log (...args) {
  if (verbose) console.log(...args)
}

function isVerbose () {
  const loglevel = process.env.npm_config_loglevel
  return loglevel === 'verbose' || process.env.CI === 'true'
}

module.exports = { spawn, log, isVerbose, verbose }
