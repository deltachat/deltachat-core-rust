const fs = require('fs')
const path = require('path')

if (process.platform !== 'win32') {
  console.log('postinstall: not windows, so skipping!')
  process.exit(0)
}

const from = path.resolve(
  __dirname,
  '..',
  'deltachat-core-rust',
  'target',
  'release',
  'deltachat.dll'
)

const getDestination = () => {
  const argv = process.argv
  if (argv.length === 3 && argv[2] === '--prebuild') {
    return path.resolve(
      __dirname,
      '..',
      'prebuilds',
      'win32-x64',
      'deltachat.dll'
    )
  } else {
    return path.resolve(
      __dirname,
      '..',
      'build',
      'Release',
      'deltachat.dll'
    )
  }
}

const dest = getDestination()

copy(from, dest, (err) => {
  if (err) throw err
  console.log(`postinstall: copied ${from} to ${dest}`)
})

function copy (from, to, cb) {
  fs.stat(from, (err, st) => {
    if (err) return cb(err)
    fs.readFile(from, (err, buf) => {
      if (err) return cb(err)
      fs.writeFile(to, buf, (err) => {
        if (err) return cb(err)
        fs.chmod(to, st.mode, cb)
      })
    })
  })
}
