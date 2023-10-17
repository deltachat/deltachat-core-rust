#!/bin/bash 

BUILD_ID=${1:?specify build ID}

SSHTARGET=${SSHTARGET-ci@b1.delta.chat}
BUILDDIR=ci_builds/$BUILD_ID

echo "--- Copying files to $SSHTARGET:$BUILDDIR"

set -xe

ssh -oBatchMode=yes -oStrictHostKeyChecking=no  $SSHTARGET mkdir -p "$BUILDDIR"
git ls-files >.rsynclist 
# we seem to need .git for setuptools_scm versioning 
find .git >>.rsynclist
rsync --delete --files-from=.rsynclist -az ./ "$SSHTARGET:$BUILDDIR"

set +x

echo "--- Running Python tests remotely"

ssh $SSHTARGET <<_HERE
    set +x -e

    # make sure all processes exit when ssh dies
    shopt -s huponexit

    export RUSTC_WRAPPER=\`which sccache\`
    cd $BUILDDIR
    export TARGET=release
    export CHATMAIL_DOMAIN=$CHATMAIL_DOMAIN

    #we rely on tox/virtualenv being available in the host
    #rm -rf virtualenv venv
    #virtualenv -q -p python3.7 venv 
    #source venv/bin/activate
    #pip install -q tox virtualenv

    set -x
    which python
    source \$HOME/venv/bin/activate
    which python

    bash scripts/run-python-test.sh 
_HERE
