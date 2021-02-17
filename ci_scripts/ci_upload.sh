#!/bin/bash

if [ -z "$DEVPI_LOGIN" ] ; then 
    echo "required: password for 'dc' user on https://m.devpi/net/dc index"
    exit 0
fi

set -xe

PYDOCDIR=${1:?directory with python docs}
WHEELHOUSEDIR=${2:?directory with pre-built wheels}
DOXYDOCDIR=${3:?directory where doxygen docs to be found}
SSHTARGET=ci@b1.delta.chat

    
export BRANCH=${CIRCLE_BRANCH:?specify branch for uploading purposes}

export BUILDDIR=ci_builds/$REPONAME/$BRANCH/${CIRCLE_JOB:?jobname}/${CIRCLE_BUILD_NUM:?circle-build-number}/wheelhouse


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

ssh -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null $SSHTARGET mkdir -p $BUILDDIR 
scp -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null ci_scripts/cleanup_devpi_indices.py $SSHTARGET:$BUILDDIR 
rsync -avz \
  -e "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null" \
  $WHEELHOUSEDIR \
  $SSHTARGET:$BUILDDIR 

ssh $SSHTARGET <<_HERE
    set +x -e
    # make sure all processes exit when ssh dies
    shopt -s huponexit

    # we rely on the "venv" virtualenv on the remote account to exist 
    source venv/bin/activate
    cd $BUILDDIR

    devpi use https://m.devpi.net
    devpi login dc --password $DEVPI_LOGIN

    N_BRANCH=${BRANCH//[\/]}

    devpi use dc/$$N_BRANCH || {
        devpi index -c $$N_BRANCH 
        devpi use dc/$$N_BRANCH
    }
    devpi index $$N_BRANCH bases=/root/pypi
    devpi upload deltachat*

    # remove devpi non-master dc indices if thy are too old
    # this script was copied above 
    python cleanup_devpi_indices.py
_HERE
