#!/bin/bash

# Small helper to easily run integration tests locally for development
# purposes.  Any arguments are passed straight to tox.  E.g. to run
# only one environment run with:
#
#   ./run-integration-tests.sh -e py35
#
# To also run with `pytest -x` use:
#
#   ./run-integration-tests.sh -e py35 -- -x

cargo build -p deltachat_ffi --release

# CFLAGS=-I`pwd`/deltachat-ffi
# LD_LIBRARY_PATH=`pwd`/target/release
# export CFLAGS
# export LD_LIBRARY_PATH
export DCC_RS_DEV=$(pwd)

pushd python
toxargs="$@"
if [ -e liveconfig ]; then
    toxargs="--liveconfig liveconfig $@"
fi
tox $toxargs
ret=$?
popd
exit $ret
