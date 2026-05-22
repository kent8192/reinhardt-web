#!/usr/bin/env bash
# Verify each crate's `## [0.1.0]` block carries enough bullets relative to its
# commit history since v0.1.0-alpha.1 (or the earliest tag found).
#
# Usage:
#   ./scripts/verify_changelog_coverage.sh
#
# Behavior:
#   For every CHANGELOG.md under the workspace, count:
#     - commits affecting that crate's path since the earliest alpha tag
#     - bullets inside the `## [0.1.0]` block
#   Warn if the ratio is < 30% (very rough sanity threshold — the goal is to
#   catch crates accidentally skipped by the aggregator, not to validate
#   per-commit coverage which would be too noisy for fix-up commits).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

WARN_RATIO_PCT=30
FAIL=0

check_one() {
	local changelog="$1"
	local crate_path="$2"
	local tag_prefix="$3"

	# Locate the earliest alpha tag for this crate (fallback: alpha.1 of reinhardt-web).
	local first_tag
	first_tag=$(git tag -l "${tag_prefix}v0.1.0-alpha*" | sort -V | head -n1 || true)
	if [[ -z "$first_tag" ]]; then
		first_tag=$(git tag -l "${tag_prefix}v0.1.0-rc*" | sort -V | head -n1 || true)
	fi
	if [[ -z "$first_tag" ]]; then
		echo "SKIP  $crate_path (no alpha/rc tag found for prefix ${tag_prefix})"
		return
	fi

	local commits
	commits=$(git log "${first_tag}..HEAD" --oneline -- "$crate_path" 2>/dev/null | wc -l | tr -d ' ')
	local bullets
	bullets=$(awk '/^## \[0\.1\.0\][^-]/,/^## \[0\.1\.0-/' "$changelog" 2>/dev/null | grep -c '^- ' || true)

	# Guard: zero-commit crates (rare; just confirm a block exists).
	if [[ "$commits" -eq 0 ]]; then
		if [[ "$bullets" -gt 0 ]]; then
			printf "OK    %-50s bullets=%d (no new commits)\n" "$crate_path" "$bullets"
		else
			printf "WARN  %-50s empty 0.1.0 block (no commits, no bullets)\n" "$crate_path"
		fi
		return
	fi

	local ratio_pct=$(( bullets * 100 / commits ))
	if [[ "$ratio_pct" -lt "$WARN_RATIO_PCT" ]]; then
		printf "WARN  %-50s commits=%d bullets=%d (%d%% — below %d%%)\n" "$crate_path" "$commits" "$bullets" "$ratio_pct" "$WARN_RATIO_PCT"
		FAIL=1
	else
		printf "OK    %-50s commits=%d bullets=%d (%d%%)\n" "$crate_path" "$commits" "$bullets" "$ratio_pct"
	fi
}

# Root CHANGELOG covers the whole workspace.
if [[ -f "CHANGELOG.md" ]]; then
	check_one "CHANGELOG.md" "." "reinhardt-web@"
fi

# Per-crate CHANGELOGs.
for changelog in crates/*/CHANGELOG.md; do
	[[ -f "$changelog" ]] || continue
	crate_dir="$(dirname "$changelog")"
	crate_name="$(basename "$crate_dir")"
	check_one "$changelog" "$crate_dir" "${crate_name}@"
done

if [[ "$FAIL" -ne 0 ]]; then
	echo
	echo "Coverage check produced warnings. Review WARN lines above." >&2
	exit 1
fi

echo
echo "All CHANGELOGs cleared the ${WARN_RATIO_PCT}% coverage threshold."
