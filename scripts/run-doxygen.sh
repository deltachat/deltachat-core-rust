#!/bin/sh
set -e

cd deltachat-ffi 
PROJECT_NUMBER=$(git log -1 --format="%h (%cd)") doxygen
