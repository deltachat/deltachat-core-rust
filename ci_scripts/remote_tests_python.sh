#!/bin/bash 

export BRANCH=${CIRCLE_BRANCH:?branch to build}
export REPONAME=${CIRCLE_PROJECT_REPONAME:?repository name}
export SSHTARGET=ci@b1.delta.chat

# we construct the BUILDDIR such that we can easily share the
# CARGO_TARGET_DIR between runs ("..")
export BUILDDIR=ci_builds/$REPONAME/$BRANCH/${CIRCLE_JOB:?jobname}/${CIRCLE_BUILD_NUM:?circle-build-number}

set -e

echo "--- Copying files to $SSHTARGET:$BUILDDIR"

ssh -oStrictHostKeyChecking=no  $SSHTARGET mkdir -p "$BUILDDIR"
git ls-tree -r $BRANCH -r --name-only >.rsynclist 
rsync --files-from=.rsynclist -az ./ "$SSHTARGET:$BUILDDIR"
# we seem to need .git for setuptools_scm versioning 
rsync -az .git "$SSHTARGET:$BUILDDIR"

echo "--- Running $CIRCLE_JOB remotely"

ssh $SSHTARGET <<_HERE
    set +x -e
    cd $BUILDDIR
    # let's share the target dir with our last run on this branch/job-type
    # cargo will make sure to block/unblock us properly 
    export CARGO_TARGET_DIR=\`pwd\`/../target
    export TARGET=release
    export DCC_PY_LIVECONFIG=$DCC_PY_LIVECONFIG

    rm -rf virtualenv venv
    virtualenv -q -p python3.7 venv 
    source venv/bin/activate
    set -x

    pip install -q tox virtualenv
    bash ci_scripts/run-python-test.sh 
_HERE
