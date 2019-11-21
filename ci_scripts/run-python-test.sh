#!/bin/bash
#
# Run functional tests for Delta Chat core using the python bindings 
# and tox/pytest. 

set -e +x

# make sure we have proper settings to run Online tests 
X=${DCC_PY_LIVECONFIG:?need env var to run Online tests}
set -x

# for core-building and python install step
export DCC_RS_TARGET=release 
export DCC_RS_DEV=`pwd`

cd python

python install_python_bindings.py 

# remove and inhibit writing PYC files 
rm -rf tests/__pycache__
rm -rf src/deltachat/__pycache__
export PYTHONDONTWRITEBYTECODE=1

# run python tests (tox invokes pytest to run tests in python/tests)
# we split out qr-tests run to minimize likelyness of flaky tests
# (some qr tests are pretty heavy in terms of send/received
# messages and async-imap's likely has concurrency problems, 
# eg https://github.com/async-email/async-imap/issues/4 )
tox -e lint,py37 
unset DCC_PY_LIVECONFIG

