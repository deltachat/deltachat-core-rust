#!/bin/bash 

export BRANCH=${CIRCLE_BRANCH:?branch to build}
GITURL=https://github.com/deltachat/deltachat-core-rust

ssh -oStrictHostKeyChecking=no  ci@b1.delta.chat <<_HERE
    set -xe
    mkdir -p $BRANCH 
    cd $BRANCH/
    echo "--------------------------------------------------"
    echo "                Checkout"
    echo "--------------------------------------------------"

    if [ -d "deltachat-core-rust" ] ; then
        cd deltachat-core-rust 
        git fetch origin
        git clean -q -fd
        git checkout $BRANCH
        git reset --hard origin/$BRANCH
    else
        git clone $GITURL 
        cd deltachat-core-rust
        git checkout $BRANCH
    fi
    export TARGET=x86_64-unknown-linux-gnu

    echo "--------------------------------------------------"
    echo "                running rust tests"
    echo "--------------------------------------------------"
    bash ci_scripts/run-rust-test.sh

    echo "--------------------------------------------------"
    echo "             running python tests"
    echo "--------------------------------------------------"
    virtualenv -p python3.7 venv 
    source venv/bin/activate
    export DCC_PY_LIVECONFIG=$DCC_PY_LIVECONFIG
    export CARGO_TARGET_DIR=\`pwd\`/target-py
   
    pip install -q tox virtualenv
    bash ci_scripts/run-python-test.sh 
_HERE
