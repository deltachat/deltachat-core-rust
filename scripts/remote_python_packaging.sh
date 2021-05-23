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
rsync --delete --files-from=.rsynclist -az ./ "$SSHTARGET:$BUILDDIR-arm64"

set +x

# we have to create a remote file for the remote-docker run 
# so we can do a simple ssh command with a TTY
# so that when our job dies, all container-runs are aborted.
# sidenote: the circle-ci machinery will kill ongoing jobs
# if there are new commits and we want to ensure that 
# everything is terminated/cleaned up and we have no orphaned
# useless still-running docker-containers consuming resources. 

for arch in "" "-arm64"; do

ssh $SSHTARGET bash -c "cat >${BUILDDIR}${arch}/exec_docker_run" <<_HERE
    set +x -e
    shopt -s huponexit
    cd ${BUILDDIR}${arch}
    export DCC_NEW_TMP_EMAIL=$DCC_NEW_TMP_EMAIL
    set -x

    # run everything else inside docker 
    docker run -e DCC_NEW_TMP_EMAIL \
       --rm -it -v \$(pwd):/mnt -w /mnt \
       deltachat/coredeps${arch} scripts/run_all.sh

_HERE

done

echo "--- Running $CIRCLE_JOB remotely"

echo "--- Building aarch64 wheels"
ssh -t $SSHTARGET bash "$BUILDDIR-arm64/exec_docker_run"

echo "--- Building x86_64 wheels"
ssh -t $SSHTARGET bash "$BUILDDIR/exec_docker_run"

mkdir -p workspace 

# Wheels
for arch in "" "-arm64"; do
rsync -avz "$SSHTARGET:$BUILDDIR${arch}/python/.docker-tox/wheelhouse/*manylinux201*" workspace/wheelhouse/
done

# Source packages
rsync -avz "$SSHTARGET:$BUILDDIR${arch}/python/.docker-tox/dist/*" workspace/wheelhouse/

# Documentation
rsync -avz "$SSHTARGET:$BUILDDIR/python/doc/_build/" workspace/py-docs
