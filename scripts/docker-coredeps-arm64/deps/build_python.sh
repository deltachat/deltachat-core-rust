#!/bin/bash

set -x -e

# we use the python3.6 environment as the base environment 
/opt/python/cp36-cp36m/bin/pip install tox devpi-client auditwheel 

pushd /usr/bin

ln -s /opt/_internal/cpython-3.6.*/bin/tox
ln -s /opt/_internal/cpython-3.6.*/bin/devpi
ln -s /opt/_internal/cpython-3.6.*/bin/auditwheel

popd
