const {default:dc} = require("./dist")

const ac = new dc("testdtrdtrh")

ac.startJSONRPCHandler(console.log)

setTimeout(()=>{
ac.close() // This segfaults -> TODO Findout why?

console.log("still living")
}, 1000)
