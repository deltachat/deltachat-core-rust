# webxdc: at least 200 times more interesting than Web3 :) 

webxdc combines secure chat-messaging and web tech in a unique way to fix shortcomings of both Web2 and Web3 to provide: 

- enjoyable UX: no-login, no-account and no-server paradigm 
- a new fun P2P web app development playground with very low barrier of entry 
- decentralized secure interactive offline-first web apps without coins, chains or stores 

## How it works  

1. Send a message with an webxdc app (archive or bundled html) as attachment. 

2. On tapping open a sandboxed system web view from the unpacked webxdc app.

3. Route app-state updates through the chat by exposing send/receive app message API to the sandboxed web views. 

Sandboxed webxdc apps are barred from making any network requests, reloading code or resources. 
Webxdc apps rather need to bring all the resources with them.

## Benefits to users of webxdc apps 

- full & fast web interactivity available in an offline first manner

- no need for logins, accounts or discovery mechanisms because apps start and run privately 
  from an existing social chat context 

- privacy by protocol: leaking of content or metadata from app app usage virtually impossible

- webxdc app messaging is also e2e-encrypted by default and there is no
  way to read or collect app state information, not even by webxdc app developers 
  because apps can not contact anyone outside the chat

- no need to think about creating or representing yourself as a wallet :)


## Benefits to developers of webxdc apps

webxdc empowers FOSS developments in unprecedented ways: 

- just use all the existing JS/html5 libraries and designs of your choice
- quick onboarding: only a handful API methods to learn 
- serverless (but really): no worrying about hosting a server or configuring DNS, AWS etc 
- permissionless: no worrying about registering at app stores for distribution
- unbuerocratic: no worrying about login/password/expiry procedures or leaks 
- secure: no worrying about cryptographic algos or e2e-encryption complexity

On the flip side, you need to learn how to do state updates between instances of your webxdc apps. This is a classic P2P problem and there are simple (send full state update) and advanced ways (use CRDTs or similar) to arrange decentralized state. In any case, there is no DHT let alone blockchain needed and thus no Crypto or coin needed, either.


## Other important Benefits 

Apps can become small again. 640KB should be enough for everybody, right? :) 

Network usage is minimal. Webxdc is designed to run and to be tested with end-users in low-resourced precarious contexts that suffer internet and power outages regularly. 

As webxdc does not require browsers to make remote network requests, a lot of the related attack surfaces and code complexity is cut out. 
