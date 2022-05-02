> Steps on how to get windows set up properly for the node bindings

## install git

E.g via <https://git-scm.com/download/win>

## install node

Download and install `v16` from <https://nodejs.org/en/>

## install rust

Download and run `rust-init.exe` from <https://www.rust-lang.org/tools/install>

## configure node for native addons

```
$ npm i node-gyp -g
$ npm i windows-build-tools -g
```

`windows-build-tools` will install `Visual Studio 2017` by default and should not mess with existing installations of `Visual Studio C++`.

## get the code

```
$ mkdir -p src/deltachat
$ cd src/deltachat
$ git clone https://github.com/deltachat/deltachat-node
```

## build the code

```
$ cd src/deltachat/deltachat-node
$ npm install
```
