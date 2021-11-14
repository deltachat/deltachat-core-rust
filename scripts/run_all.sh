#!/bin/bash
#
# Build the Delta Chat Core Rust library, Python wheels and docs 

set -e -x

# Perform clean build of core and install.
export TOXWORKDIR=.docker-tox

# compile core lib

export PATH=/root/.cargo/bin:$PATH
cargo build --release -p deltachat_ffi
# cargo test --all --all-features

# Statically link against libdeltachat.a.
export DCC_RS_DEV=$(pwd)
export DCC_RS_TARGET=release

# Configure access to a base python and to several python interpreters
# needed by tox below.
export PATH=$PATH:/opt/python/cp37-cp37m/bin
export PYTHONDONTWRITEBYTECODE=1
pushd /bin
rm -f python3.7
ln -s /opt/python/cp37-cp37m/bin/python3.7
rm -f python3.8
ln -s /opt/python/cp38-cp38/bin/python3.8
rm -f python3.9
ln -s /opt/python/cp39-cp39/bin/python3.9
popd

pushd python
# prepare a clean tox run
rm -rf tests/__pycache__
rm -rf src/deltachat/__pycache__
mkdir -p $TOXWORKDIR

# disable live-account testing to speed up test runs and wheel building
# XXX we may switch on some live-tests on for better ensurances 
# Note that the independent remote_tests_python step does all kinds of
# live-testing already. 
unset DCC_NEW_TMP_EMAIL
tox --workdir "$TOXWORKDIR" -e py37,py38,py39,auditwheels
popd


echo -----------------------
echo generating python docs
echo -----------------------
(cd python && tox --workdir "$TOXWORKDIR" -e doc)
