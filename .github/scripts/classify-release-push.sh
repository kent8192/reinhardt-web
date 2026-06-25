#!/usr/bin/env bash
set -euo pipefail

set_output() {
	local key="$1"
	local value="$2"

	printf '%s=%s\n' "$key" "$value"
	if [ -n "${GITHUB_OUTPUT:-}" ]; then
		printf '%s=%s\n' "$key" "$value" >> "$GITHUB_OUTPUT"
	fi
}

emit_classification() {
	local is_release_merge="$1"
	local pr_number="${2:-}"
	local head_branch="${3:-}"
	local base_branch="${4:-}"
	local merge_style="${5:-}"

	set_output "is_release_merge" "$is_release_merge"
	set_output "release_pr_number" "$pr_number"
	set_output "release_pr_head_branch" "$head_branch"
	set_output "release_pr_base_branch" "$base_branch"
	set_output "release_pr_merge_style" "$merge_style"
}

fail() {
	echo "::error::$*" >&2
	exit 1
}

commit_msg="${COMMIT_MSG:-}"
if [ -z "$commit_msg" ]; then
	echo "No commit message found; treating this push as an ordinary release-pr update."
	emit_classification "false"
	exit 0
fi

first_line=$(printf '%s\n' "$commit_msg" | head -n1)
pr_number=""
message_head_branch=""
merge_style=""

if [[ "$first_line" =~ ^Merge\ pull\ request\ \#([0-9]+)\ from\ [^/]+/(.+)$ ]]; then
	pr_number="${BASH_REMATCH[1]}"
	message_head_branch="${BASH_REMATCH[2]}"
	merge_style="merge"
elif [[ "$first_line" =~ ^chore:\ release\ \(#([0-9]+)\) ]]; then
	pr_number="${BASH_REMATCH[1]}"
	merge_style="squash"
else
	echo "Commit message is not a supported PR merge shape; treating this push as ordinary."
	emit_classification "false"
	exit 0
fi

if [ -n "${RELEASE_PUSH_PR_JSON:-}" ]; then
	pr_json="$RELEASE_PUSH_PR_JSON"
else
	repo="${GITHUB_REPOSITORY:-}"
	if [ -z "$repo" ]; then
		fail "GITHUB_REPOSITORY is required when RELEASE_PUSH_PR_JSON is not provided"
	fi
	pr_json=$(gh pr view "$pr_number" \
		--repo "$repo" \
		--json headRefName,headRepository,baseRefName,labels,mergeCommit)
fi

if ! jq -e . >/dev/null <<< "$pr_json"; then
	fail "Could not parse PR #$pr_number metadata as JSON"
fi

head_branch=$(jq -r '.headRefName // ""' <<< "$pr_json")
head_repo=$(jq -r '.headRepository.nameWithOwner // ""' <<< "$pr_json")
base_branch=$(jq -r '.baseRefName // ""' <<< "$pr_json")
merge_commit_sha=$(jq -r '.mergeCommit.oid // ""' <<< "$pr_json")
has_release_label=$(jq -r '[.labels[]?.name] | index("release") != null' <<< "$pr_json")

if [ -z "$head_branch" ]; then
	fail "PR #$pr_number has no head branch metadata"
fi

case "$head_branch" in
	release-plz-*|develop-release-plz-*)
		;;
	*)
		echo "PR #$pr_number head '$head_branch' is not a release-plz head branch; treating this push as ordinary."
		emit_classification "false" "$pr_number" "$head_branch" "$base_branch" "$merge_style"
		exit 0
		;;
esac

expected_repo="${GITHUB_REPOSITORY:-}"
if [ -n "$expected_repo" ] && [ "$head_repo" != "$expected_repo" ]; then
	fail "PR #$pr_number head repository '$head_repo' does not match '$expected_repo'"
fi

if [ "$merge_style" = "merge" ] && [ -n "$message_head_branch" ] && [ "$message_head_branch" != "$head_branch" ]; then
	fail "Merge commit branch '$message_head_branch' does not match PR #$pr_number head '$head_branch'"
fi

expected_base="${GITHUB_REF_NAME:-}"
if [ -n "$expected_base" ] && [ "$base_branch" != "$expected_base" ]; then
	fail "PR #$pr_number base '$base_branch' does not match pushed ref '$expected_base'"
fi

if [ "$has_release_label" != "true" ]; then
	fail "PR #$pr_number head '$head_branch' is release-shaped but does not have the 'release' label"
fi

expected_sha="${GITHUB_SHA:-}"
if [ -n "$expected_sha" ] && [ "$merge_commit_sha" != "$expected_sha" ]; then
	fail "Pushed SHA '$expected_sha' does not match PR #$pr_number merge commit '$merge_commit_sha'"
fi

echo "PR #$pr_number is a verified Release PR merge from '$head_branch'."
emit_classification "true" "$pr_number" "$head_branch" "$base_branch" "$merge_style"
