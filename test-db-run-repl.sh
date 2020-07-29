#!/bin/bash

#
# command to run example cmd application
#

#cargo run --features="rustyline" --example repl -- test.db
cargo run --example repl --features repl -- test.db

