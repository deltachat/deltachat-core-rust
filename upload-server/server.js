const p = require('path')
const crypto = require('crypto')
const express = require('express')
const fs = require('fs')
const { pipeline } = require('stream')

const app = express()

const config = {
  path: process.env.UPLOAD_PATH || p.resolve('./uploads'),
  port: process.env.PORT || 8080,
  hostname: process.env.HOSTNAME || '0.0.0.0',
  baseurl: process.env.BASE_URL || null
}

if (!config.baseurl.endsWith('/')) config.baseurl = config.baseurl + '/'

if (!fs.existsSync(config.path)) {
  fs.mkdirSync(config.path, { recursive: true })
}

const baseUrl = config.baseurl || `http://${config.hostname}:${config.port}/`

app.post('*', (req, res) => {
  const filename = crypto.randomBytes(12).toString('hex')
  const ws = fs.createWriteStream(p.join(config.path, filename))
  pipeline(req, ws, err => {
    if (err) console.error(err)
    if (err) res.status(500).send(err.message)
    const url = baseUrl + filename
    console.log('file uploaded: ' + filename)
    res.send(url)
  })
})

app.get('/:filename', (req, res) => {
  const filepath = p.normalize(p.join(config.path, req.params.filename))
  if (!filepath.startsWith(config.path)) {
    return res.code(500).send('bad request')
  }
  const rs = fs.createReadStream(filepath)
  res.setHeader('content-type', 'application/octet-stream')
  pipeline(rs, res, err => {
    if (err) console.error(err)
    if (err) res.status(500).send(err.message)
    res.end()
  })
})

app.listen(config.port, err => {
  if (err) console.error(err)
  else console.log('Listening on ' + baseUrl)
})
