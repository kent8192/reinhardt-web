#!/usr/bin/env bash
# Initialize a develop/m.n.l branch by bumping every Cargo.toml from the
# previous stable version (currently on main) to <target>-alpha.1.
#
# This is the first commit on a fresh develop branch. Subsequent pushes are
# handled automatically by release-plz on `develop/**`, which increments the
# alpha counter (alpha.1 -> alpha.2 -> ...). See issue #4541 and
# instructions/RELEASE_PROCESS.md "Develop Branch Release Workflow" (DBR-1).
#
# Usage:
#   git checkout -b develop/0.2.0 main
#   ./scripts/init-develop-branch.sh 0.2.0
#   git commit -am "chore(release): initialize develop/0.2.0 at 0.2.0-alpha.1"
#
# Exit codes:
#   0  Cargo.toml files were updated.
#   1  Invalid arguments.
#   2  Could not read prior version, or no files were updated (idempotent
#      re-run, version skew, or wrong target).

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

# Best-effort branch check (skip gracefully if detached HEAD or git missing).
CURRENT_BRANCH="$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
EXPECTED_BRANCH="develop/${TARGET}"
if [ -n "$CURRENT_BRANCH" ] && [ "$CURRENT_BRANCH" != "$EXPECTED_BRANCH" ]; then
	echo "Warning: current branch '$CURRENT_BRANCH' is not '$EXPECTED_BRANCH'." >&2
fi

PRIOR="$(awk -F'"' '/^version = / { print $2; exit }' "$REPO_ROOT/Cargo.toml")"
if [ -z "$PRIOR" ]; then
	echo "Error: could not read version from root Cargo.toml" >&2
	exit 2
fi

NEW="${TARGET}-alpha.1"

if [ "$PRIOR" = "$NEW" ]; then
	echo "Root Cargo.toml is already at $NEW; nothing to do."
	exit 0
fi

echo "Initializing develop branch:"
echo "  prior version: $PRIOR"
echo "  new version:   $NEW"
echo

PRIOR_RE="${PRIOR//./\\.}"
SED_EXPR="s/(version[[:space:]]*=[[:space:]]*\")${PRIOR_RE}(\")/\\1${NEW}\\2/g"

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

echo
if [ "$CHANGED" -eq 0 ]; then
	echo "Warning: no Cargo.toml files were updated." >&2
	exit 2
fi
echo "Done: $CHANGED Cargo.toml file(s) bumped to '$NEW'."
echo
echo "Next steps:"
echo "  1. Review the diff:    git diff"
echo "  2. Commit:             git commit -am 'chore(release): initialize $EXPECTED_BRANCH at $NEW'"
echo "  3. Push:               git push -u origin $EXPECTED_BRANCH"
