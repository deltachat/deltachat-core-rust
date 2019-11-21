#!/bin/bash 
export BRANCH=${CIRCLE_BRANCH:?branch to build}

export GITURL=${CIRCLE_REPOSITORY_URL:?git url of core-rust repo}

export SSHTARGET=ci@b1.delta.chat

# we construct the BUILDDIR such that we can easily share the
# CARGO_TARGET_DIR between runs ("..")
export BUILDDIR=ci_builds/$BRANCH/${CIRCLE_JOB:?jobname}/${CIRCLE_BUILD_NUM:?circle-build-number}

set -e

echo "--- Copying files to $SSHTARGET:$BUILDDIR"

ssh -oStrictHostKeyChecking=no  $SSHTARGET mkdir -p "$BUILDDIR"
git ls-tree -r $BRANCH -r --name-only >.rsynclist 
rsync --files-from=.rsynclist -az ./ "$SSHTARGET:$BUILDDIR"

echo "--- Running $CIRCLE_JOB remotely"

ssh $SSHTARGET <<_HERE
    set +x -e
    cd $BUILDDIR
    # let's share the target dir with our last run on this branch/job-type
    # cargo will make sure to block/unblock us properly 
    export CARGO_TARGET_DIR=\`pwd\`/../target
    export TARGET=x86_64-unknown-linux-gnu
    export RUSTC_WRAPPER=sccache

    bash ci_scripts/run-rust-test.sh
_HERE

