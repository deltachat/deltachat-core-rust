#!/bin/bash
#
# Build the Delta Chat C/Rust library
#
set -e -x

# perform clean build of core and install 
export TOXWORKDIR=.docker-tox

# build core library

cargo build --release -p deltachat_ffi

# configure access to a base python and 
# to several python interpreters needed by tox below
export PATH=$PATH:/opt/python/cp35-cp35m/bin
export PYTHONDONTWRITEBYTECODE=1
pushd /bin
ln -s /opt/python/cp27-cp27m/bin/python2.7
ln -s /opt/python/cp36-cp36m/bin/python3.6
ln -s /opt/python/cp37-cp37m/bin/python3.7
popd

#
# run python tests
#

if [ -n "$TESTS" ]; then 

    echo ----------------
    echo run python tests
    echo ----------------

    pushd python 
    # first run all tests ...
    rm -rf tests/__pycache__
    rm -rf src/deltachat/__pycache__
    export PYTHONDONTWRITEBYTECODE=1
    tox --workdir "$TOXWORKDIR" -e py27,py35,py36,py37
    popd
fi


if [ -n "$DOCS" ]; then 
    echo -----------------------
    echo generating python docs
    echo -----------------------
    (cd python && tox --workdir "$TOXWORKDIR" -e doc) 
fi
