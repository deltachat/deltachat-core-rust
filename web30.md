# Web30: at least ten times more interesting than Web3 :) 

We combine secure chat-messaging and web tech in a unique way to fix deep shortcomings of both Web2 and Web3 to provide: 

- decentralized secure interactive web apps **without coins or chains or stores**
- super-simple UX: no-login, no-account and no-server paradigm 
- a new fun P2P playground with very low barrier of entry 

## How it works  

1. Send a self-contained web30-app to a chat (as archive or bundled html) 
2. When receiver or sender click on the web30 app message open a sandboxed system web view from the unpacked web30 app 
3. relay app-state update messages between web30 apps by exposing send/receive message functionality to the sandboxed web30 apps

Note that sandboxed web30 app are barred from making any network requests, reloading code or resources. They need to bring all the resources with them (offline first!). Note that we can use the same sandboxing technology that is relied upon for online banking, social media, shopping etc. 

## Benefits to users 

- full web interactivity available in an offline first manner

- no need for logins or accounts because apps run privately and started
  from an existing social chat context 

- no peer discovery mechanisms neccessary because you start from a chat group 

- web30 app messaging is e2e-encrypted by default and there is no
  way to read or collect app state information, not even by the web30 app developers 
  because they can not perform any IO outside the constrained send/receive offered by DC. 

- no need to think about creating or representing yourself as a wallet :)


## Benefits to developers of web30 apps

Web30 empowers FOSS developments in unprecedented ways: 

- no worrying about hosting a server or configuring DNS 
- no worrying about registering at app stores for distribution
- no worrying about login/password/expiry procedures or leaks 
- no worrying about cryptographic algos or e2e-encryption complexity

On the flip side, you need to learn how to do state updates between instances of your web30 apps. This is a classic P2P problem and there are CRDT and simpler methods to arrange decentralized state. But no DHT or blockchain needed and no Crypto or coin needed, either ;) 

## Other important Benefits 

Apps can become small again. 640KB should be enough for everybody, right? :) 

Web30 is designed to run and to be tested with end-users in low-resourced precarious contexts that suffer internet and power outages regularly. 

Simple model both for users and devs that can be explained in one minute and does not require diving into a quagmire of terminology and code complexity (yes, talking about you, Web3!)

As web30 does not require browsers to make remote network requests, a lot of the attack surfaces and code complexity is cut out. 

## sidenotes

the title "web30" is a work title and might change :)

