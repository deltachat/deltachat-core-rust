# Webxdc Developer Reference

(This document may eventually be merged with the [webxdc guidebook](https://deltachat.github.io/webxdc_docs/), where you may currently find other useful information.)

## Webxdc File Format

- a **Webxdc** is a **ZIP-file** with the extension `.xdc`
- the ZIP-file must use the default compression methods as of RFC 1950,
  this is "Deflate" or "Store"
- the ZIP-file must contain at least the file `index.html`
- if the Webxdc is started, `index.html` is opened in a restricted webview
  that allow accessing resources only from the ZIP-file


## Webxdc API

There are some additional APIs available once `webxdc.js` is included
(the file will be provided by the concrete implementations,
no need to add `webxdc.js` to your ZIP-file):

```html
<script src="webxdc.js"></script>
```

### sendUpdate()

```js
window.webxdc.sendUpdate(update, descr);
```

A Webxdc is usually shared in a chat and run independently on each peer.
To get a shared state, the peers use `sendUpdate()` to send updates to each other.

- `update`: an object with the following properties:  
    - `update.payload`: any javascript primitive, array or object.
    - `update.info`: optional, short, informational message that will be added to the chat,
       eg. "Alice voted" or "Bob scored 123 in MyGame".
       usually only one line of text is shown
       and if there are series of info messages, older ones may be dropped.
       use this option sparingly to not spam the chat.
    - `update.document`: optional, name of the document in edit,
       must not be used eg. in games where the Webxdc does not create documents
    - `update.summary`: optional, short text, shown beside Webxdc icon;
       it is recommended to use some aggregated value,  eg. "8 votes", "Highscore: 123"

- `descr`: short, human-readable description what this update is about.
  this is shown eg. as a fallback text in an email program.

All peers, including the sending one,
will receive the update by the callback given to `setUpdateListener()`.

There are situations where the user cannot send messages to a chat,
eg. if the webxdc instance comes as a contact request or if the user has left a group.
In these cases, you can still call `sendUpdate()`,
however, the update won't be sent to other peers
and you won't get the update by `setUpdateListener()`.


### setUpdateListener()

```js
let promise = window.webxdc.setUpdateListener((update) => {}, serial);
```

With `setUpdateListener()` you define a callback that receives the updates
sent by `sendUpdate()`. The callback is called for updates sent by you or other peers.
The `serial` specifies the last serial that you know about (defaults to 0). 
The returned promise resolves when the listener has processed all the update messages known at the time when  `setUpdateListener` was called. 

Each `update` which is passed to the callback comes with the following properties: 

- `update.payload`: equals the payload given to `sendUpdate()`

- `update.serial`: the serial number of this update.
  Serials are larger `0` and newer serials have higher numbers.
  There may be gaps in the serials
  and it is not guaranteed that the next serial is exactly incremented by one.

- `update.max_serial`: the maximum serial currently known.
  If `max_serial` equals `serial` this update is the last update (until new network messages arrive).

- `update.info`: optional, short, informational message (see `sendUpdate()`)

- `update.document`: optional, document name as set by the sender, (see `sendUpdate()`),
  implementations show the document name eg. beside the app icon or in the title bar

- `update.summary`: optional, short text, shown beside icon (see `sendUpdate()`)


### selfAddr

```js
window.webxdc.selfAddr
```

Property with the peer's own address.
This is esp. useful if you want to differ between different peers -
just send the address along with the payload,
and, if needed, compare the payload addresses against selfAddr() later on.


### selfName

```js
window.webxdc.selfName
```

Property with the peer's own name.
This is name chosen by the user in their settings,
if there is nothing set, that defaults to the peer's address.


## manifest.toml

If the ZIP-file contains a `manifest.toml` in its root directory,
some basic information are read and used from there.

the `manifest.toml` has the following format

```toml
name = "My Name"
source_code_url = "https://example.org/orga/repo"
```

- `name` - The name of the Webxdc.
  If no name is set or if there is no manifest, the filename is used as the Webxdc name.

- `source_code_url` - Optional URL where the source code of the Webxdc and maybe other information can be found.
  UI may make the url accessible via a "Help" menu in the Webxdc window.


## Webxdc Icon

If the ZIP-root contains an `icon.png` or `icon.jpg`,
these files are used as the icon for the Webxdc.
The icon should be a square at reasonable width/height;
round corners etc. will be added by the implementations as needed.
If no icon is set, a default icon will be used.


## Webxdc Examples

The following example shows an input field and  every input is show on all peers.

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8"/>
    <script src="webxdc.js"></script>
  </head>
  <body>
    <input id="input" type="text"/>
    <a href="" onclick="sendMsg(); return false;">Send</a>
    <p id="output"></p>
    <script>
    
      function sendMsg() {
        msg = document.getElementById("input").value;
        window.webxdc.sendUpdate({payload: msg}, 'Someone typed "'+msg+'".');
      }
    
      function receiveUpdate(update) {
        document.getElementById('output').innerHTML += update.payload + "<br>";
      }
    
      window.webxdc.setUpdateListener(receiveUpdate, 0);
    </script>
  </body>
</html>
```

[Webxdc Development Tool](https://github.com/deltachat/webxdc-dev)
offers an **Webxdc Simulator** that can be used in many browsers without any installation needed.
You can also use that repository as a template for your own Webxdc -
just clone and start adapting things to your need.


### Advanced Examples

- [2048](https://github.com/adbenitez/2048.xdc)
- [Draw](https://github.com/adbenitez/draw.xdc)
- [Poll](https://github.com/r10s/webxdc-poll/)
- [Tic Tac Toe](https://github.com/Simon-Laux/tictactoe.xdc)
- Even more with [Topic #webxdc on Github](https://github.com/topics/webxdc) or in the [webxdc GitHub organization](https://github.com/webxdc)


## Closing Remarks

- older devices might not have the newest js features in their webview,
  you may want to transpile your code down to an older js version eg. with https://babeljs.io
- viewport and scaling features are implementation specific,
  if you want to have an explicit behavior, you can add eg.
  `<meta name="viewport" content="initial-scale=1; user-scalable=no">` to your Webxdc
- the `<title>` tag should not be used and its content is usually not displayed;
  instead, use the `name` property from `manifest.toml`
- there are tons of ideas for enhancements of the API and the file format,
  eg. in the future, we will may define icon- and manifest-files,
  allow to aggregate the state or add metadata.
