#!/bin/sh
codespell \
  --skip './test-data,./.git,node_modules,.mypy_cache,./src/provider/data.rs,.tox,site-packages,target,Cargo.lock,*.js.map,package-lock.json,./proptest-regressions' \
  --ignore-words-list crate,keypair,keypairs,iif
