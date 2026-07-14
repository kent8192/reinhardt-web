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

scan_dependency_table_dotted_keys() {
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

		/^[[:space:]]*\[/ {
			table = normalized_table($0)
			dependency_table = table ~ /^\[(dependencies|dev-dependencies|build-dependencies)\]$/ || table ~ /^\[workspace[.]dependencies\]$/ || table ~ /^\[target[.].*[.](dependencies|dev-dependencies|build-dependencies)\]$/
		}

		dependency_table && /^[[:space:]]*["\047]?anyhow["\047]?[[:space:]]*[.][[:space:]]*["\047]?[[:alnum:]_-]+["\047]?[[:space:]]*=/ {
			print substr(FILENAME, length(root) + 1) ":" FNR ":" $0
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

scan \
	--glob 'Cargo.toml' \
	--regexp "^[[:space:]]*(anyhow|\"anyhow\"|'anyhow')[[:space:]]*=" \
	--regexp "^[[:space:]]*\\[[^]#]*(dependencies|\"dependencies\"|'dependencies'|dev-dependencies|\"dev-dependencies\"|'dev-dependencies'|build-dependencies|\"build-dependencies\"|'build-dependencies')[[:space:]]*[.][[:space:]]*(anyhow|\"anyhow\"|'anyhow')[[:space:]]*\\]" \
	--regexp "^[[:space:]]*([^#=]+[.][[:space:]]*)?(dependencies|\"dependencies\"|'dependencies'|dev-dependencies|\"dev-dependencies\"|'dev-dependencies'|build-dependencies|\"build-dependencies\"|'build-dependencies')[[:space:]]*[.][[:space:]]*(anyhow|\"anyhow\"|'anyhow')([[:space:]]*[.][[:space:]]*[[:alnum:]_-]+)?[[:space:]]*=" \
	--regexp "package[[:space:]]*=[[:space:]]*(\"anyhow\"|'anyhow')" \
	--regexp 'dep:anyhow'

scan_dependency_table_dotted_keys

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
