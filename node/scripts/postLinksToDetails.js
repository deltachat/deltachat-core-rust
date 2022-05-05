const { readFileSync } = require('fs')

const sha = JSON.parse(
  readFileSync(process.env['GITHUB_EVENT_PATH'], 'utf8')
).pull_request.head.sha

const base_url =
  'https://download.delta.chat/node/'

const GITHUB_API_URL =
  'https://api.github.com/repos/deltachat/deltachat-core-rust/statuses/' + sha

const file_url = process.env['URL']
const GITHUB_TOKEN = process.env['GITHUB_TOKEN']

const STATUS_DATA = {
  state: 'success',
  description: '⏩ Click on "Details" to download →',
  context: 'Download the node-bindings.tar.gz',
  target_url: base_url + file_url + '.tar.gz',
}

const http = require('https')

const options = {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'User-Agent': 'github-action ci for deltachat deskop',
    authorization: 'Bearer ' + GITHUB_TOKEN,
  },
}

const req = http.request(GITHUB_API_URL, options, function(res) {
  var chunks = []
  res.on('data', function(chunk) {
    chunks.push(chunk)
  })
  res.on('end', function() {
    var body = Buffer.concat(chunks)
    console.log(body.toString())
  })
})

req.write(JSON.stringify(STATUS_DATA))
req.end()
