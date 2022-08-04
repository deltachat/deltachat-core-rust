const { default: dc } = require('./dist')

const ac = new dc('test1233490')

console.log('[1]')

ac.startJsonRpcHandler(console.log)

console.log('[2]')
console.log(
  ac.jsonRpcRequest(
    JSON.stringify({
      jsonrpc: '2.0',
      method: 'batch_set_config',
      id: 3,
      params: [
        69,
        {
          addr: '',
          mail_user: '',
          mail_pw: '',
          mail_server: '',
          mail_port: '',
          mail_security: '',
          imap_certificate_checks: '',
          send_user: '',
          send_pw: '',
          send_server: '',
          send_port: '',
          send_security: '',
          smtp_certificate_checks: '',
          socks5_enabled: '0',
          socks5_host: '',
          socks5_port: '',
          socks5_user: '',
          socks5_password: '',
        },
      ],
    })
  )
)

console.log('[3]')

setTimeout(() => {
  console.log('[4]')
  ac.close() // This segfaults -> TODO Findout why?

  console.log('still living')
}, 1000)
