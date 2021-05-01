#!/bin/bash
set -xe

JOB=${1:?need to specify 'rust' or 'python'}
BRANCH="$(git branch | grep \* | cut -d ' ' -f2)"
REPONAME="$(basename $(git rev-parse --show-toplevel))"

time bash "scripts/remote_tests_$JOB.sh" "$USER-$BRANCH-$REPONAME"
