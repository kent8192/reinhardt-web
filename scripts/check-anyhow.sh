#!/usr/bin/env bash
# Reject direct dynamic-error dependencies and owned source or API-documentation usage.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCAN_ROOT="$(cd "${1:-$DEFAULT_ROOT}" && pwd)"

COMMON_ARGS=(
	--line-number
	--with-filename
	--no-heading
	--color never
	--glob '!.git/**'
	--glob '!target/**'
	--glob '!vendor/**'
	--glob '!Cargo.lock'
	--glob '!CHANGELOG*'
	--glob '!docs/superpowers/**'
)

FOUND=0

scan() {
	local output
	local status

	set +e
	output=$(cd "$SCAN_ROOT" && rg "${COMMON_ARGS[@]}" "$@")
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

scan \
	--glob 'Cargo.toml' \
	--regexp '^[[:space:]]*anyhow[[:space:]]*=' \
	--regexp 'dep:anyhow'

scan \
	--glob '*.rs' \
	--regexp 'anyhow::' \
	--regexp 'anyhow[[:space:]]*!' \
	--regexp 'use[[:space:]]+anyhow::\{[^}]*\b(bail|ensure|Context)\b'

scan \
	--glob '*.md' \
	--regexp 'anyhow::' \
	--regexp 'anyhow[[:space:]]*!' \
	--regexp 'use[[:space:]]+anyhow'

if [ "$FOUND" -ne 0 ]; then
	echo "Remove direct anyhow dependencies and replace owned usage with repository error types." >&2
	exit 1
fi

echo "anyhow-check: OK"
