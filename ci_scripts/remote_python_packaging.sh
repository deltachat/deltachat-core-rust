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

# we have to create a remote file for the remote-docker run 
# so we can do a simple ssh command with a TTY
# so that when our job dies, all container-runs are aborted.
# sidenote: the circle-ci machinery will kill ongoing jobs
# if there are new commits and we want to ensure that 
# everything is terminated/cleaned up and we have no orphaned
# useless still-running docker-containers consuming resources. 

ssh $SSHTARGET bash -c "cat >$BUILDDIR/exec_docker_run" <<_HERE
    set +x -e
    shopt -s huponexit
    cd $BUILDDIR
    export DCC_NEW_TMP_EMAIL=$DCC_NEW_TMP_EMAIL
    set -x

    # run everything else inside docker 
    docker run -e DCC_NEW_TMP_EMAIL \
       --rm -it -v \$(pwd):/mnt -w /mnt \
       deltachat/coredeps ci_scripts/run_all.sh

_HERE

echo "--- Running $CIRCLE_JOB remotely"

ssh -t $SSHTARGET bash "$BUILDDIR/exec_docker_run"
mkdir -p workspace 
rsync -avz "$SSHTARGET:$BUILDDIR/python/.docker-tox/wheelhouse/*manylinux201*" workspace/wheelhouse/
rsync -avz "$SSHTARGET:$BUILDDIR/python/.docker-tox/dist/*" workspace/wheelhouse/
rsync -avz "$SSHTARGET:$BUILDDIR/python/doc/_build/" workspace/py-docs
