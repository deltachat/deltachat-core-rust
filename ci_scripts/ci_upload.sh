#!/bin/bash

if [ -z "$DEVPI_LOGIN" ] ; then 
    echo "required: password for 'dc' user on https://m.devpi/net/dc index"
    exit 1
fi

set -xe

#DOXYDOCDIR=${1:?directory where doxygen docs to be found}
PYDOCDIR=${1:?directory with python docs}
WHEELHOUSEDIR=${2:?directory with pre-built wheels}

export BRANCH=${CIRCLE_BRANCH:?specify branch for uploading purposes}


# python docs to py.delta.chat
rsync -avz \
  -e "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null" \
  "$PYDOCDIR/html/" \
  delta@py.delta.chat:build/${BRANCH}

# C docs to c.delta.chat
rsync -avz \
  -e "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null" \
  "$DOXYDOCDIR/html/" \
  delta@py.delta.chat:build-c/${BRANCH}

echo -----------------------
echo upload wheels 
echo -----------------------

# Bundle external shared libraries into the wheels
pushd $WHEELHOUSEDIR

pip install devpi-client
devpi use https://m.devpi.net
devpi login dc --password $DEVPI_LOGIN

devpi use dc/$BRANCH || {
    devpi index -c $BRANCH 
    devpi use dc/$BRANCH
}
devpi index $BRANCH bases=/root/pypi
devpi upload deltachat*.whl

popd
