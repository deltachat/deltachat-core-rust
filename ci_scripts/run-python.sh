#!/bin/bash
#
# Build the Delta Chat C/Rust library typically run in a docker
# container that contains all library deps but should also work
# outside if you have the dependencies installed on your system.

set -e -x

# Perform clean build of core and install.
export TOXWORKDIR=.docker-tox

# install core lib

export PATH=/root/.cargo/bin:$PATH
cargo build --release -p deltachat_ffi
# cargo test --all --all-features

# Statically link against libdeltachat.a.
export DCC_RS_DEV=$(pwd)

# Configure access to a base python and to several python interpreters
# needed by tox below.
export PATH=$PATH:/opt/python/cp35-cp35m/bin
export PYTHONDONTWRITEBYTECODE=1
pushd /bin
ln -s /opt/python/cp27-cp27m/bin/python2.7
ln -s /opt/python/cp36-cp36m/bin/python3.6
ln -s /opt/python/cp37-cp37m/bin/python3.7
popd

if [ -n "$TESTS" ]; then

    pushd python
    # prepare a clean tox run
    rm -rf tests/__pycache__
    rm -rf src/deltachat/__pycache__
    export PYTHONDONTWRITEBYTECODE=1

    # run tox. The circle-ci project env-var-setting DCC_PY_LIVECONFIG 
    # allows running of "liveconfig" tests but for speed reasons
    # we run them only for the highest python version we support
    
    # we split out qr-tests run to minimize likelyness of flaky tests
    # (some qr tests are pretty heavy in terms of send/received
    # messages and rust's imap code likely has concurrency problems) 
    tox --workdir "$TOXWORKDIR" -e py37 -- --reruns 3 -k "not qr"
    tox --workdir "$TOXWORKDIR" -e py37 -- --reruns 3 -k "qr"
    unset DCC_PY_LIVECONFIG
    tox --workdir "$TOXWORKDIR" -p4 -e lint,py35,py36,doc
    tox --workdir "$TOXWORKDIR" -e auditwheels
    popd
fi


# if [ -n "$DOCS" ]; then
#     echo -----------------------
#     echo generating python docs
#     echo -----------------------
#     (cd python && tox --workdir "$TOXWORKDIR" -e doc)
# fi
