#!/bin/bash
#
# Run Python functional test for Delta Chat core. 
#

set -e -x

# build the core library 
cargo build --release -p deltachat_ffi --target=$TARGET

# Statically link against libdeltachat.a.
export DCC_RS_DEV=$(pwd)

cd python

# remove and inhibit writing PYC files 
rm -rf tests/__pycache__
rm -rf src/deltachat/__pycache__
export PYTHONDONTWRITEBYTECODE=1

# make sure we have proper settings to run Online tests 
X=${DCC_PY_LIVECONFIG:?need env var to run Online tests}

# run python tests (tox invokes pytest to run tests in python/tests)
# we split out qr-tests run to minimize likelyness of flaky tests
# (some qr tests are pretty heavy in terms of send/received
# messages and async-imap's likely has concurrency problems, 
# eg https://github.com/async-email/async-imap/issues/4 )
tox -e lint,py37 -- --reruns 3 -k "not qr" tests/
tox -e py37 -- --reruns 5 -k "qr" tests/
unset DCC_PY_LIVECONFIG

