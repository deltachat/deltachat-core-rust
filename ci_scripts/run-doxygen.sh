#!/usr/bin/env bash

set -ex

cd deltachat-ffi 
PROJECT_NUMBER=$(git log -1 --format "%h (%cd)") doxygen 
