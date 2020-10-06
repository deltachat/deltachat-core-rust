#!/bin/bash 

export BRANCH=${CIRCLE_BRANCH:?branch to build}
export REPONAME=${CIRCLE_PROJECT_REPONAME:?repository name}
export SSHTARGET=${SSHTARGET-ci@b1.delta.chat}

export BUILDDIR=ci_builds/$REPONAME/$BRANCH/${CIRCLE_JOB:?jobname}/${CIRCLE_BUILD_NUM:?circle-build-number}

set -e

echo "--- Copying files to $SSHTARGET:$BUILDDIR"

ssh -oBatchMode=yes -oStrictHostKeyChecking=no  $SSHTARGET mkdir -p "$BUILDDIR"
git ls-files >.rsynclist 
rsync --delete --files-from=.rsynclist -az ./ "$SSHTARGET:$BUILDDIR"

echo "--- Running $CIRCLE_JOB remotely"

ssh $SSHTARGET <<_HERE
    set +x -e
    # make sure all processes exit when ssh dies
    shopt -s huponexit
    export RUSTC_WRAPPER=\`which sccache\`
    cd $BUILDDIR
    export TARGET=x86_64-unknown-linux-gnu
    export RUSTC_WRAPPER=sccache

    bash ci_scripts/run-rust-test.sh
_HERE

