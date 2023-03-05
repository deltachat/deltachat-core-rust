#!/usr/bin/env bash
#
# Script to reupload `deltachat-rpc-server` binaries produced by `deltachat-rpc-server.yml` into release.
#
set -euo pipefail
set -x
VERSION_TAG=v1.111.0

# JSON such as
# {"databaseId":4336818281,"headBranch":"v1.111.0","status":"completed"}
# or empty string if there is no workflow running.
WORKFLOW_RUN_JSON="$(gh run list --workflow=deltachat-rpc-server.yml --json 'headBranch,databaseId,status' --jq "[.[] |  select(.headBranch==\"$VERSION_TAG\")][0]")"

if test -z "$WORKFLOW_RUN_JSON"; then
	echo Starting the workflow
	gh workflow run deltachat-rpc-server.yml --ref "$VERSION_TAG"
	return 0
fi

if test "$(echo "$WORKFLOW_RUN_JSON" | jq -r '.status')" = "completed"; then
	RUN_ID="$(echo "$WORKFLOW_RUN_JSON" | jq -r '.databaseId')"

	rm -fr run-assets
	mkdir -p run-assets
	gh run download $RUN_ID --dir run-assets

	rm -fr upload-assets
	mkdir -p upload-assets
	for x in run-assets/*; do
		x="$(basename $x)"
		if test -f run-assets/$x/deltachat-rpc-server; then
			mv run-assets/$x/deltachat-rpc-server upload-assets/$x
		fi

		if test -f run-assets/$x/deltachat-rpc-server.exe; then
			mv run-assets/$x/deltachat-rpc-server.exe upload-assets/$x
		fi
	done

	cd upload-assets
	gh release upload "$VERSION_TAG" *
fi
