const p = require('path')
const express = require('express')
const fs = require('fs')
const { pipeline } = require('stream')

const app = express()

const config = {
  path: process.env.UPLOAD_PATH || p.resolve('./uploads'),
  port: process.env.PORT || 8080,
  hostname: process.env.HOSTNAME || '0.0.0.0',
  baseurl: process.env.BASE_URL
}

if (!config.baseurl) config.baseurl = `http://${config.hostname}:${config.port}/`
if (!config.baseurl.endsWith('/')) config.baseurl = config.baseurl + '/'

if (!fs.existsSync(config.path)) {
  fs.mkdirSync(config.path, { recursive: true })
}

app.use('/:filename', checkFilenameMiddleware)
app.put('/:filename', (req, res) => {
  const uploadpath = req.uploadpath
  const filename = req.params.filename
  fs.stat(uploadpath, (err, stat) => {
    if (err && err.code !== 'ENOENT') {
      console.error('error', err.message)
      return res.code(500).send('internal server error')
    }
    if (stat) return res.status(500).send('filename in use')

    const ws = fs.createWriteStream(uploadpath)
    pipeline(req, ws, err => {
      if (err) {
        console.error('error', err.message)
        return res.status(500).send('internal server error')
      }
      console.log('file uploaded: ' + uploadpath)
      const url = config.baseurl + filename
      res.end(url)
    })
  })
})

app.get('/:filename', (req, res) => {
  const uploadpath = req.uploadpath
  const rs = fs.createReadStream(uploadpath)
  res.setHeader('content-type', 'application/octet-stream')
  pipeline(rs, res, err => {
    if (err) console.error('error', err.message)
    if (err) return res.status(500).send(err.message)
  })
})

function checkFilenameMiddleware (req, res, next) {
  const filename = req.params.filename
  if (!filename) return res.status(500).send('missing filename')
  if (!filename.match(/^[a-zA-Z0-9]{26,32}$/)) {
    return res.status(500).send('illegal filename')
  }
  const uploadpath = p.normalize(p.join(config.path, req.params.filename))
  if (!uploadpath.startsWith(config.path)) {
    return res.code(500).send('bad request')
  }
  req.uploadpath = uploadpath
  next()
}

app.listen(config.port, err => {
  if (err) console.error(err)
  else console.log(`Listening on ${config.baseurl}`)
})
