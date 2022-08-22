#!/bin/sh
#
# Build the Delta Chat Core Rust library, Python wheels and docs 

set -e -x

# Perform clean build of core and install.

# compile core lib

cargo build --release -p deltachat_ffi --features jsonrpc

# Statically link against libdeltachat.a.
export DCC_RS_DEV="$PWD"
export DCC_RS_TARGET=release

export PYTHONDONTWRITEBYTECODE=1
cd python

TOXWORKDIR=.docker-tox
# prepare a clean tox run
rm -rf tests/__pycache__
rm -rf src/deltachat/__pycache__
mkdir -p $TOXWORKDIR

# disable live-account testing to speed up test runs and wheel building
# XXX we may switch on some live-tests on for better ensurances 
# Note that the independent remote_tests_python step does all kinds of
# live-testing already. 
unset DCC_NEW_TMP_EMAIL

# Try to build wheels for a range of interpreters, but don't fail if they are not available.
# E.g. musllinux_1_1 does not have PyPy interpreters as of 2022-07-10
tox --workdir "$TOXWORKDIR" -e py37,py38,py39,py310,pypy37,pypy38,pypy39,auditwheels --skip-missing-interpreters true


echo -----------------------
echo generating python docs
echo -----------------------
tox --workdir "$TOXWORKDIR" -e doc
