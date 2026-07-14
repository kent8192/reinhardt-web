#!/usr/bin/env bash
# Reject direct dynamic-error dependencies and owned source or API-documentation usage.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCAN_ROOT="$(cd "${1:-$DEFAULT_ROOT}" && pwd)"

COMMON_OUTPUT_ARGS=(
	--line-number
	--with-filename
	--no-heading
	--color never
)

COMMON_EXCLUSIONS=(
	--glob '!**/.git/**'
	--glob '!**/target/**'
	--glob '!**/vendor/**'
	--glob '!Cargo.lock'
	--glob '!CHANGELOG*'
	--glob '!docs/superpowers/**'
)

FOUND=0

scan() {
	local output
	local status

	set +e
	output=$(cd "$SCAN_ROOT" && rg "$@" "${COMMON_OUTPUT_ARGS[@]}" "${COMMON_EXCLUSIONS[@]}")
	status=$?
	set -e

	if [ "$status" -gt 1 ]; then
		echo "anyhow-check: repository scan failed" >&2
		exit "$status"
	fi
	if [ "$status" -eq 0 ]; then
		printf '%s\n' "$output"
		FOUND=1
	fi
}

scan_cargo_manifests() {
	local manifest
	local manifests
	local output
	local status
	local -a manifest_paths=()

	set +e
	manifests=$(cd "$SCAN_ROOT" && rg --files --glob 'Cargo.toml' "${COMMON_EXCLUSIONS[@]}")
	status=$?
	set -e

	if [ "$status" -gt 1 ]; then
		echo "anyhow-check: repository scan failed" >&2
		exit "$status"
	fi
	if [ "$status" -eq 1 ]; then
		return
	fi

	while IFS= read -r manifest; do
		manifest_paths+=("$SCAN_ROOT/$manifest")
	done < <(printf '%s\n' "$manifests")

	set +e
	output=$(awk -v root="$SCAN_ROOT/" '
		function normalized_table(value) {
			sub(/[[:space:]]*#.*/, "", value)
			gsub(/[[:space:]]/, "", value)
			gsub(/["\047]/, "", value)
			return value
		}

		function is_dependency_table(value) {
			return value ~ /^\[(dependencies|dev-dependencies|build-dependencies)\]$/ || value ~ /^\[workspace[.]dependencies\]$/ || value ~ /^\[target[.].*[.](dependencies|dev-dependencies|build-dependencies)\]$/
		}

		function is_dependency_subtable(value) {
			return value ~ /^\[(dependencies|dev-dependencies|build-dependencies)[.][[:alnum:]_-]+\]$/ || value ~ /^\[workspace[.]dependencies[.][[:alnum:]_-]+\]$/ || value ~ /^\[target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.][[:alnum:]_-]+\]$/
		}

		function is_prohibited_dependency_entry(value) {
			return value ~ /^anyhow([.][[:alnum:]_-]+)?=/ || value ~ /^[[:alnum:]_-]+[.]package=anyhow$/ || value ~ /^[[:alnum:]_-]+=\{.*package=anyhow([,}])/
		}

		function is_prohibited_root_dependency_entry(value) {
			if (value ~ /^(dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^(dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else if (value ~ /^workspace[.]dependencies[.]/) {
				sub(/^workspace[.]dependencies[.]/, "", value)
			} else if (value ~ /^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else {
				return 0
			}
			return is_prohibited_dependency_entry(value)
		}

		function emit_match() {
			print substr(FILENAME, length(root) + 1) ":" FNR ":" $0
		}

		FNR == 1 {
			table = ""
			dependency_table = 0
			dependency_subtable = 0
			feature_table = 0
		}

		/^[[:space:]]*\[/ {
			table = normalized_table($0)
			dependency_table = is_dependency_table(table)
			dependency_subtable = is_dependency_subtable(table)
			feature_table = table == "[features]"
			if (dependency_subtable && table ~ /[.]anyhow\]$/) {
				emit_match()
			}
			next
		}

		{
			code = $0
			sub(/[[:space:]]*#.*/, "", code)
			compact = code
			gsub(/[[:space:]"\047]/, "", compact)
			matched = 0

			if (table == "" && is_prohibited_root_dependency_entry(compact)) {
				matched = 1
			} else if (dependency_table && is_prohibited_dependency_entry(compact)) {
				matched = 1
			} else if (dependency_subtable && compact == "package=anyhow") {
				matched = 1
			}

			if ((table == "" && compact ~ /^features[.][[:alnum:]_-]+=/ || feature_table) && compact ~ /dep:anyhow/) {
				matched = 1
			}

			if (matched) {
				emit_match()
			}
		}
	' "${manifest_paths[@]}")
	status=$?
	set -e

	if [ "$status" -ne 0 ]; then
		echo "anyhow-check: repository scan failed" >&2
		exit "$status"
	fi
	if [ -n "$output" ]; then
		printf '%s\n' "$output"
		FOUND=1
	fi
}

scan_cargo_manifests

scan \
	--glob '*.rs' \
	--regexp 'anyhow[[:space:]]*::' \
	--regexp 'anyhow[[:space:]]*!' \
	--regexp 'use[[:space:]]+anyhow([[:space:]]*;|[[:space:]]*::|[[:space:]]+as[[:space:]])'

scan \
	--glob '*.md' \
	--regexp 'anyhow[[:space:]]*::' \
	--regexp 'anyhow[[:space:]]*!' \
	--regexp 'use[[:space:]]+anyhow[[:space:]]*(;|::)'

if [ "$FOUND" -ne 0 ]; then
	echo "Remove direct anyhow dependencies and replace owned usage with repository error types." >&2
	exit 1
fi

echo "anyhow-check: OK"
