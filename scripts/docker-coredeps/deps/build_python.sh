#!/bin/bash

set -x -e

# we use the python3.7 environment as the base environment
/opt/python/cp37-cp37m/bin/pip install tox devpi-client auditwheel

pushd /usr/bin

ln -s /opt/_internal/cpython-3.7.*/bin/tox
ln -s /opt/_internal/cpython-3.7.*/bin/devpi
ln -s /opt/_internal/cpython-3.7.*/bin/auditwheel

popd
