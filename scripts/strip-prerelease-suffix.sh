#!/usr/bin/env bash
# Strip prerelease suffix from every Cargo.toml in the workspace.
#
# Used by .github/workflows/release-plz-promote.yml to graduate a
# develop/m.n.l release stream from prerelease (m.n.l-alpha.N or -rc.N) to
# stable (m.n.l). See issue #4541 and instructions/RELEASE_PROCESS.md
# "Develop Branch Release Workflow" (DBR-3).
#
# Usage:
#   ./scripts/strip-prerelease-suffix.sh <target-version>
#
# Arguments:
#   target-version  The stable version to graduate to (e.g., 0.2.0).
#                   Must be of the form X.Y.Z (no prerelease suffix).
#
# Behavior:
#   For every Cargo.toml under the repo root (excluding target/ and .git/),
#   replace `version = "X.Y.Z-(alpha|beta|rc).N"` with `version = "X.Y.Z"`,
#   where X.Y.Z equals the target argument. Both line-anchored TOML key
#   fields and inline workspace-dependency tables are rewritten.
#
# Exit codes:
#   0  At least one Cargo.toml was rewritten.
#   1  Invalid arguments.
#   2  No Cargo.toml contained a prerelease of the target (idempotent re-run
#      or wrong target).

set -euo pipefail

if [ $# -ne 1 ]; then
	echo "Usage: $0 <target-version>" >&2
	echo "Example: $0 0.2.0" >&2
	exit 1
fi

TARGET="$1"

if ! [[ "$TARGET" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "Error: target-version '$TARGET' must be X.Y.Z (no suffix)" >&2
	exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Escape dots for the sed pattern so 0.2.0 does not match 0X2X0.
TARGET_RE="${TARGET//./\\.}"
SED_EXPR="s/(version[[:space:]]*=[[:space:]]*\")${TARGET_RE}-(alpha|beta|rc)\.[0-9]+(\")/\\1${TARGET}\\3/g"

CARGO_FILES=()
while IFS= read -r f; do
	CARGO_FILES+=("$f")
done < <(find "$REPO_ROOT" \
	-name "Cargo.toml" \
	-not -path "*/target/*" \
	-not -path "*/.git/*" \
	| sort)

CHANGED=0
for f in "${CARGO_FILES[@]}"; do
	tmp="$(mktemp)"
	sed -E "$SED_EXPR" "$f" > "$tmp"
	if ! diff -q "$f" "$tmp" >/dev/null 2>&1; then
		cp "$tmp" "$f"
		rel="${f#"$REPO_ROOT"/}"
		echo "  updated: $rel"
		CHANGED=$((CHANGED + 1))
	fi
	rm -f "$tmp"
done

if [ "$CHANGED" -eq 0 ]; then
	echo "Warning: no Cargo.toml files contained a prerelease of '$TARGET'." >&2
	exit 2
fi

echo "Done: $CHANGED Cargo.toml file(s) graduated to '$TARGET'."
