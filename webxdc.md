# webxdc: at least 200 times more interesting than Web3 :) 

webxdc combines secure chat-messaging and web tech to provide fertile ground for
a decentralized secure web app eco-system without servers, coins or chains. 

## How webxdc works  

webxdc apps use existing chat groups (instead of web servers) to communicate with each other, with roughly this initial technical work flow: 

1. Send a message to a chat with an webxdc app (archive or bundled html) as attachment. 

2. On tapping chat members open a sandboxed system web view from the unpacked webxdc app.
   Sandboxed webxdc apps are barred from making any network requests, reloading code or resources. 

3. webxdc apps can not reach the internet themselves and rather can only send "app-state" update message to other app users in the chat group.

While this describes an already working "proof of concept" usage there are already a lot of further reaching discussions. However, we prefer to avoid "early over-generalizing" and rather evolve further mechanisms and features from real-world needs, especially from people living in precarious contexts with bad networks and affected by internet outages.  


## Benefits to users of webxdc apps 

- full & fast web interactivity available in an offline first manner

- no need for logins, accounts or discovery mechanisms because apps start and run privately 
  from an existing social chat context 

- privacy by protocol: leaking of content or metadata from app usage
  virtually impossible. No need for annoying GDPR or cookie consent dialogues. 

- webxdc app messaging is e2e-encrypted by default and there is no
  way to read or collect app state information, not even by webxdc developers 
  because their apps can not contact anyone outside the chat. 

- no need to think about creating or representing yourself as a wallet :)


## Benefits to developers of webxdc apps

webxdc empowers FOSS developments in unprecedented ways: 

- well-known paradim: use all the existing JS/html5 libraries and designs of your choice
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

## Is webxdc tied to Delta Chat? 

Webxdc apps could be used by other chat messengers. However, most of those are busy with introducing advertising or blockchains so we are not holding our breath right now. Rest assured, when the time comes and interest rises we'll be there and open to collaboration :)

