# webxdc: at least 200 times more interesting than Web3 :) 

webxdc combines secure chat-messaging and web tech in a unique way to fix deep shortcomings of both Web2 and "Web3" to provide: 

- super-simple UX: no-login, no-account and no-server paradigm 
- a new fun P2P web app development playground with very low barrier of entry 
- decentralized secure interactive web apps **without coins, chains or stores**

## How it works  

1. Send a self-contained webxdc-app to a chat (as archive or bundled html) 
2. When receiver or sender click on the webxdc app message open a sandboxed system web view from the unpacked webxdc app 
3. relay app-state update messages between webxdc apps by exposing send/receive message functionality to the sandboxed webxdc apps

Note that sandboxed webxdc apps are barred from making any network requests, reloading code or resources. We use the same sandboxing technology that is relied upon for online banking, social media, shopping and various other browser tabs, with the added blocking of network requests. They need to bring all the resources with them, allowing users to interactively work with the app in an offline-first manner. 

## Benefits to users 

- full & fast web interactivity available in an offline first manner

- no need for logins or accounts because apps start and run privately 
  from an existing social chat context 

- no peer discovery mechanisms neccessary because you start from a chat group 

- webxdc app messaging is e2e-encrypted by default and there is no
  way to read or collect app state information, not even by the webxdc app developers 
  because they can not perform any IO outside the constrained send/receive offered by DC. 

- no need to think about creating or representing yourself as a wallet :)


## Benefits to developers of webxdc apps

webxdc empowers FOSS developments in unprecedented ways: 

- use all the existing JS/html5 libraries and designs of your choice
- very minimal API to arrange for automatically encrypted encrypted app-to-app communications 
- no worrying about hosting a server or configuring DNS or AWS or the like
- no worrying about registering at app stores for distribution
- no worrying about login/password/expiry procedures or leaks 
- no worrying about cryptographic algos or e2e-encryption complexity

On the flip side, you need to learn how to do state updates between instances of your webxdc apps. This is a classic P2P problem and there are simple (send full state update) and advanced ways (use CRDTs or similar) to arrange decentralized state. In any case, there is no DHT let alone blockchain needed and thus no Crypto or coin needed, either.


## Other important Benefits 

Apps can become small again. 640KB should be enough for everybody, right? :) 

Network usage is minimal. Webxdc is designed to run and to be tested with end-users in low-resourced precarious contexts that suffer internet and power outages regularly. 

Simple dev and usage model. Things can be explained in a few minutes and do not require diving into a quagmire of terminology and code complexity (yes, talking about you, Web3!)

As webxdc does not require browsers to make remote network requests, a lot of the related attack surfaces and code complexity is cut out. 
