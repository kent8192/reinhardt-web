#!/usr/bin/env bash
# Freeze the current develop/m.n.l branch from alpha to rc by bumping every
# Cargo.toml's prerelease suffix from `-alpha.N` to `-rc.1`. This marks the
# API freeze point: from this commit onward, only bug fixes are permitted
# (enforced by review per instructions/STABILITY_POLICY.md RC phase).
#
# See issue #4541 and instructions/RELEASE_PROCESS.md "Develop Branch
# Release Workflow" (DBR-2).
#
# Usage:
#   # While on develop/0.2.0
#   ./scripts/freeze-develop-to-rc.sh
#   git commit -am "chore(release): freeze develop/0.2.0 at 0.2.0-rc.1"
#
# Exit codes:
#   0  Cargo.toml files were updated.
#   1  Invalid arguments.
#   2  Could not determine current branch, or no files were updated.
#   3  Root version is not a `<target>-alpha.N` (wrong phase for freeze).

set -euo pipefail

if [ $# -ne 0 ]; then
	echo "Usage: $0" >&2
	echo "(no arguments — target version is auto-detected from current branch)" >&2
	exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CURRENT_BRANCH="$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
if [ -z "$CURRENT_BRANCH" ]; then
	echo "Error: could not determine current branch (detached HEAD?)" >&2
	exit 2
fi

if ! [[ "$CURRENT_BRANCH" =~ ^develop/([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
	echo "Error: current branch '$CURRENT_BRANCH' is not develop/X.Y.Z" >&2
	exit 2
fi
TARGET="${BASH_REMATCH[1]}"
TARGET_RE="${TARGET//./\\.}"

PRIOR="$(awk -F'"' '/^version = / { print $2; exit }' "$REPO_ROOT/Cargo.toml")"
if ! [[ "$PRIOR" =~ ^${TARGET_RE}-alpha\.[0-9]+$ ]]; then
	echo "Error: root version '$PRIOR' is not '$TARGET-alpha.N'" >&2
	echo "Freeze is only valid when the develop branch is in the alpha phase." >&2
	exit 3
fi

NEW="${TARGET}-rc.1"

echo "Freezing develop branch:"
echo "  prior version: $PRIOR"
echo "  new version:   $NEW"
echo

# Replace any `version = "X.Y.Z-alpha.N"` (any N) with `version = "X.Y.Z-rc.1"`.
SED_EXPR="s/(version[[:space:]]*=[[:space:]]*\")${TARGET_RE}-alpha\.[0-9]+(\")/\\1${NEW}\\2/g"

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
echo "Done: $CHANGED Cargo.toml file(s) frozen to '$NEW'."
echo
echo "Next steps:"
echo "  1. Review the diff:    git diff"
echo "  2. Commit:             git commit -am 'chore(release): freeze $CURRENT_BRANCH at $NEW'"
echo "  3. Push:               git push origin $CURRENT_BRANCH"
