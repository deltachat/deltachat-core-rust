#!/bin/bash 

export BRANCH=${CIRCLE_BRANCH:-master}
export REPONAME=${CIRCLE_PROJECT_REPONAME:-deltachat-core-rust}
export SSHTARGET=${SSHTARGET-ci@b1.delta.chat}

export BUILDDIR=ci_builds/$REPONAME/$BRANCH/${CIRCLE_JOB:?jobname}/${CIRCLE_BUILD_NUM:?circle-build-number}

echo "--- Copying files to $SSHTARGET:$BUILDDIR"

set -xe

ssh -oBatchMode=yes -oStrictHostKeyChecking=no  $SSHTARGET mkdir -p "$BUILDDIR"
git ls-files >.rsynclist 
# we seem to need .git for setuptools_scm versioning 
find .git >>.rsynclist
rsync --delete --files-from=.rsynclist -az ./ "$SSHTARGET:$BUILDDIR"

set +x

echo "--- Running $CIRCLE_JOB remotely"

ssh $SSHTARGET <<_HERE
    set +x -e

    # make sure all processes exit when ssh dies
    shopt -s huponexit

    export RUSTC_WRAPPER=\`which sccache\`
    cd $BUILDDIR
    export TARGET=release
    export DCC_NEW_TMP_EMAIL=$DCC_NEW_TMP_EMAIL

    #we rely on tox/virtualenv being available in the host
    #rm -rf virtualenv venv
    #virtualenv -q -p python3.7 venv 
    #source venv/bin/activate
    #pip install -q tox virtualenv

    set -x
    which python
    source \$HOME/venv/bin/activate
    which python

    bash ci_scripts/run-python-test.sh 
_HERE
