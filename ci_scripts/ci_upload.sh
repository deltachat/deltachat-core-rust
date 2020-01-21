#!/bin/bash

if [ -z "$DEVPI_LOGIN" ] ; then 
    echo "required: password for 'dc' user on https://m.devpi/net/dc index"
    exit 0
fi

set -xe

PYDOCDIR=${1:?directory with python docs}
WHEELHOUSEDIR=${2:?directory with pre-built wheels}
DOXYDOCDIR=${3:?directory where doxygen docs to be found}

export BRANCH=${CIRCLE_BRANCH:?specify branch for uploading purposes}


# python docs to py.delta.chat
ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null delta@py.delta.chat mkdir -p build/${BRANCH}
rsync -avz \
  --delete \
  -e "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null" \
  "$PYDOCDIR/html/" \
  delta@py.delta.chat:build/${BRANCH}

# C docs to c.delta.chat
ssh -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null delta@c.delta.chat mkdir -p build-c/${BRANCH}
rsync -avz \
  --delete \
  -e "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null" \
  "$DOXYDOCDIR/html/" \
  delta@c.delta.chat:build-c/${BRANCH}

echo -----------------------
echo upload wheels 
echo -----------------------

# Bundle external shared libraries into the wheels
pushd $WHEELHOUSEDIR

pip3 install -U pip
pip3 install devpi-client
devpi use https://m.devpi.net
devpi login dc --password $DEVPI_LOGIN

N_BRANCH=${BRANCH//[\/]}

devpi use dc/$N_BRANCH || {
    devpi index -c $N_BRANCH 
    devpi use dc/$N_BRANCH
}
devpi index $N_BRANCH bases=/root/pypi
devpi upload deltachat*

popd

# remove devpi non-master dc indices if thy are too old
python ci_scripts/cleanup_devpi_indices.py
