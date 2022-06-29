# TODO

- [ ] different test type to simulate two devices: to test autocrypt_initiate_key_transfer & autocrypt_continue_key_transfer

## MVP - Websocket server&client

For kaiOS and other experiments, like a deltachat "web" over network from an android phone.

- [ ] coverage for a majority of the API
- [ ] Blobs served
- [ ] Blob upload (for attachments, setting profile-picture, importing backup and so on)
- [ ] other way blobs can be addressed when using websocket vs. jsonrpc over dc-node
- [ ] Web push API? At least some kind of notification hook closure this lib can accept.

### Other Ideas for the Websocket server

- [ ] make sure there can only be one connection at a time to the ws
  - why? , it could give problems if its commanded from multiple connections
- [ ] encrypted connection?
- [ ] authenticated connection?
- [ ] Look into unit-testing for the proc macros?
- [ ] proc macro taking over doc comments to generated typescript file

## Desktop Apis

Incomplete todo for desktop api porting, just some remainders for points that might need more work:

- [ ] manual start/stop io functions in the api for context and accounts, so "not syncing all accounts" can still be done in desktop -> webserver should then not do start io on all accounts by default
