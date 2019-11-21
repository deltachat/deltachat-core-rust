#!/bin/bash

set -xe
export CIRCLE_JOB=remote_tests_${1:?need to specify 'rust' or 'python'} 
export CIRCLE_BUILD_NUM=$USER
export CIRCLE_BRANCH=`git branch | grep \* | cut -d ' ' -f2`
export CIRCLE_PROJECT_REPONAME=$(basename `git rev-parse --show-toplevel`)

time bash ci_scripts/$CIRCLE_JOB.sh 
