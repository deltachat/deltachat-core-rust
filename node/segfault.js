const { default: dc } = require('./dist')

const ac = new dc('test1233490')

console.log("[1]");

ac.startJsonRpcHandler(console.log)

console.log("[2]");
console.log(
  ac.jsonRpcRequest(
    JSON.stringify({
      jsonrpc: '2.0',
      method: 'get_all_account_ids',
      params: [],
      id: 2,
    })
  )
)

console.log("[3]");

setTimeout(() => {
    console.log("[4]");
  ac.close() // This segfaults -> TODO Findout why?

  console.log('still living')
}, 1000)