#!/usr/bin/env bash
# Validate that release branch prefixes cannot cross the main/develop boundary.
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
CLASSIFIER="$REPO_ROOT/.github/scripts/classify-release-push.sh"

make_pr_json() {
	local head_branch="$1"
	local base_branch="$2"

	jq -cn \
		--arg head_branch "$head_branch" \
		--arg base_branch "$base_branch" \
		'{headRefName: $head_branch, headRepository: {nameWithOwner: "kent8192/reinhardt-web"}, baseRefName: $base_branch, labels: [{name: "release"}], mergeCommit: {oid: "verified-sha"}}'
}

run_classifier() {
	local head_branch="$1"
	local base_branch="$2"

	COMMIT_MSG="Merge pull request #42 from kent8192/$head_branch" \
	RELEASE_PUSH_PR_JSON="$(make_pr_json "$head_branch" "$base_branch")" \
	GITHUB_REPOSITORY="kent8192/reinhardt-web" \
	GITHUB_REF_NAME="$base_branch" \
	GITHUB_SHA="verified-sha" \
	bash "$CLASSIFIER"
}

assert_accepts() {
	local head_branch="$1"
	local base_branch="$2"

	if ! run_classifier "$head_branch" "$base_branch" >/dev/null; then
		echo "Expected $head_branch -> $base_branch to be accepted" >&2
		exit 1
	fi
}

assert_rejects() {
	local head_branch="$1"
	local base_branch="$2"

	if run_classifier "$head_branch" "$base_branch" >/dev/null 2>&1; then
		echo "Expected $head_branch -> $base_branch to be rejected" >&2
		exit 1
	fi
}

assert_accepts "release-plz-2026-07-20T00-00-00Z" "main"
assert_accepts "develop-release-plz-2026-07-20T00-00-00Z" "develop/0.4.0"
assert_rejects "release-plz-2026-07-20T00-00-00Z" "develop/0.4.0"
assert_rejects "develop-release-plz-2026-07-20T00-00-00Z" "main"
