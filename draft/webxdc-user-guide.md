# Webxdc User Guide

## Webxdc File Format

- a **Webxdc app** is a **ZIP-file** with the extension `.xdc`
- the ZIP-file must contain at least the file `index.html`
- if the Webxdc app is started, `index.html` is opened in a restricted webview
  that allow accessing resources only from the ZIP-file


## Webxdc API

Webxdc apps can do anything that is allowed in webviews -
html, css, images, canvas, javascript and so on.

There are some additional APIs available once `webxdc.js` is included
(the file will be provided by the concrete implementations,
no need to add `webxdc.js` to your ZIP-file):

```
<script src="webxdc.js></script>
```

### sendUpdate()

```
window.webxdc.sendUpdate(descr, payload);
```

Webxdc apps are usually shared in a chat and run independently on each peer.
To get a shared state, the peers use `sendUpdate()` to send updates to each other.

- `descr`: short, human-readable description what this update is about.
  this is shown eg. as a fallback text in an email program.
- `payload`: any javascript primitive, array or object.

All peers, including the sending one,
will receive the update by the callback given to `setUpdateListener()`.

### setUpdateListener()

```
window.webxdc.setUpdateListener((update) => {});
```

With `setUpdateListener()` you define a callback that receives the updates
sent by `sendUpdate()`.

- `update`: passed to the callback on updates.
- `update.payload`: equals the payload given to `sendUpdate()`

The callback is called for updates sent by you or other peers.


### getAllUpdates()

```
payloads = window.webxdc.getAllUpdates()
```

In case your Webxdc was just started,
you may want to reconstruct the state from the last run -
and also incorporate updates that may have arrived while the app was not running.

- `payloads`: the function returns all previous updates in an array, 
  eg. `[{payload: "foo"},{payload: "bar"}]`
  if `webxdc.sendUpdate("foo"); webxdc.sendUpdate("bar");` was called on the last run.

### selfAddr()

```
addr = window.webxdc.selfAddr()
```

Returns the peer's own address.
This is esp. useful if you want to differ between different peers -
just send the address along with the payload,
and, if needed, compare the payload addresses against selfAddr() later on.


## Webxdc Example

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
        window.webxdc.sendUpdate('Someone typed "'+msg+'".', msg);
      }
    
      function receiveUpdate(update) {
        document.getElementById('output').innerHTML += update.payload + "<br>";
      }
    
      window.webxdc.setUpdateListener(receiveUpdate);
      window.webxdc.getAllUpdates().forEach(receiveUpdate);

    </script>
  </body>
</html>
```

For a more advanved example, see https://github.com/r10s/webxdc-poll/ .


## Closing Remarks

- older devices might not have the newest js features in their webview,
  you may want to transpile your code down to an older js version eg. with https://babeljs.io
- there are tons of ideas for enhancements of the API and the file format,
  eg. in the future, we will may define icon- and manifest-files,
  allow to aggregate the state or add metadata.