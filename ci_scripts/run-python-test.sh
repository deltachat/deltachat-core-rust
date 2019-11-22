#!/bin/bash
#
# Run functional tests for Delta Chat core using the python bindings 
# and tox/pytest. 

set -e -x

# for core-building and python install step
export DCC_RS_TARGET=release
export DCC_RS_DEV=`pwd`

cd python

python install_python_bindings.py onlybuild

# remove and inhibit writing PYC files 
rm -rf tests/__pycache__
rm -rf src/deltachat/__pycache__
export PYTHONDONTWRITEBYTECODE=1

# run python tests (tox invokes pytest to run tests in python/tests)
#TOX_PARALLEL_NO_SPINNER=1 tox -e lint,doc
tox -e lint
tox -e doc,py37
