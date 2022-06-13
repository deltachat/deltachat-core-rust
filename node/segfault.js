const { default: dc } = require('./dist')

const ac = new dc('test123456')

ac.startJSONRPCHandler(console.log)

console.log(
  ac.jsonRPCRequest(
    JSON.stringify({
      jsonrpc: '2.0',
      method: 'get_all_account_ids',
      params: [],
      id: 2,
    })
  )
)

setTimeout(() => {
  ac.close() // This segfaults -> TODO Findout why?

  console.log('still living')
}, 1000)
