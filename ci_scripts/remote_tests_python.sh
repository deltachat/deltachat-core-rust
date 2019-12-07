#!/bin/bash 

export BRANCH=${CIRCLE_BRANCH:?branch to build}
export REPONAME=${CIRCLE_PROJECT_REPONAME:?repository name}
export SSHTARGET=${SSHTARGET-ci@b1.delta.chat}

# we construct the BUILDDIR such that we can easily share the
# CARGO_TARGET_DIR between runs ("..")
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
    cd $BUILDDIR
    # let's share the target dir with our last run on this branch/job-type
    # cargo will make sure to block/unblock us properly 
    export CARGO_TARGET_DIR=\`pwd\`/../target
    export TARGET=release
    export DCC_PY_LIVECONFIG=$DCC_PY_LIVECONFIG

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
