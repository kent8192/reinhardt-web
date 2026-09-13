#!/usr/bin/env bash
# Keep main's release comparison on its published stable release line.
set -euo pipefail

fail() {
	printf '::error::%s\n' "$*" >&2
	exit 1
}

repo_root="$(git rev-parse --show-toplevel)"
command="${1:-release-pr}"
if [ "$#" -gt 0 ]; then
	shift
fi
case "$command" in
	release-pr|update) ;;
	*) fail "Expected release-pr or update; publishing is not supported by this script." ;;
esac

baseline_temp=""
cleanup() {
	local status=$?
	if [ -n "$baseline_temp" ]; then
		if [ -e "$baseline_temp/repo/.git" ]; then
			# Only this invocation's disposable checkout may have generated lockfiles.
			if ! git -C "$repo_root" worktree remove --force "$baseline_temp/repo"; then
				status=1
			fi
		fi
		rm -rf "$baseline_temp"
	fi
	exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

args=("$command" --manifest-path "$repo_root/Cargo.toml" --config "$repo_root/release-plz.toml")
ref_name="${GITHUB_REF_NAME:-$(git branch --show-current)}"
if [ "$ref_name" = main ]; then
	manifest=$(cargo read-manifest --manifest-path "$repo_root/Cargo.toml")
	package=$(jq -er '.name' <<< "$manifest")
	version=$(jq -er '.version' <<< "$manifest")
	[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "main requires a stable package version, found '$version'."
	tag="$package@v$version"
	baseline_commit=$(git rev-parse --verify "refs/tags/$tag^{commit}") || fail "Missing stable release tag '$tag'."
	git merge-base --is-ancestor "$baseline_commit" HEAD || fail "Stable release tag '$tag' is not an ancestor of HEAD."

	baseline_temp=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/reinhardt-release-baseline.XXXXXX")
	git worktree add --quiet --detach "$baseline_temp/repo" "$baseline_commit"
	baseline_manifest="$baseline_temp/repo/Cargo.toml"
	baseline_package=$(cargo read-manifest --manifest-path "$baseline_manifest" | jq -er '[.name, .version] | join("@")')
	[ "$baseline_package" = "$package@$version" ] || fail "Stable release tag '$tag' contains '$baseline_package'."
	printf 'Comparing main against stable release %s (%s).\n' "$tag" "$baseline_commit" >&2
	args+=(--registry-manifest-path "$baseline_manifest")
elif [[ ! "$ref_name" =~ ^develop/[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	fail "Unsupported release base '$ref_name'; expected main or develop/X.Y.Z."
fi

if [ "$command" = release-pr ]; then
	: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required to create a release PR}"
	: "${GITHUB_TOKEN:?GITHUB_TOKEN is required to create a release PR}"
	# release-plz accepts its token through GIT_TOKEN without a command-line value.
	export GIT_TOKEN="$GITHUB_TOKEN"
	args+=(--repo-url "https://github.com/$GITHUB_REPOSITORY" --output json)
fi

release-plz "${args[@]}" "$@"
