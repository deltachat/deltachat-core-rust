# deltachat-upload-server

Demo server for the HTTP file upload feature.

### Usage

```
npm install
node server.js
```

Configure with environment variables:
* `UPLOAD_PATH`: Path to upload files to (default: `./uploads`)
* `PORT`: Port to listen on (default: `8080`)
* `HOSTNAME`: Hostname to listen on (default: `0.0.0.0`)
* `BASEURL`: Base URL for generated links (default: `http://[hostname]:[port]/`)

