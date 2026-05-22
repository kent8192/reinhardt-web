#!/usr/bin/env bash
# Sanity-report each crate's `## [0.1.0]` block against its commit history.
#
# Usage:
#   ./scripts/verify_changelog_coverage.sh           # report-only (always exit 0)
#   ./scripts/verify_changelog_coverage.sh --strict  # exit 1 on any LOW row
#
# Behavior:
#   For every CHANGELOG.md under the workspace, count:
#     - commits affecting that crate's path since the earliest alpha tag
#     - bullets inside the `## [0.1.0]` block
#
#   Output one row per CHANGELOG with a status:
#     OK   — bullet count is healthy relative to commit count
#     LOW  — bullet/commit ratio is below WARN_RATIO_PCT (informational; many
#            crates are dominated by chore/ci commits that legitimately do not
#            produce CHANGELOG bullets, so this is *not* a hard failure)
#     FAIL — `## [0.1.0]` block is missing entirely, OR the crate has more
#            than FAIL_MIN_COMMITS commits but zero bullets (a real gap)
#
#   Exit status:
#     0 — no FAIL rows (LOW rows are tolerated; LOW indicates "please eyeball"
#         rather than "broken")
#     0 — same as above when --strict is *not* passed even if LOW rows exist
#     1 — any FAIL row, OR (--strict and any LOW row)

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

WARN_RATIO_PCT=30
FAIL_MIN_COMMITS=10
STRICT=0
HARD_FAIL=0
SOFT_LOW=0

for arg in "$@"; do
	case "$arg" in
		--strict) STRICT=1 ;;
		-h|--help)
			sed -n '2,24p' "$0"
			exit 0
			;;
		*)
			echo "Unknown argument: $arg" >&2
			exit 2
			;;
	esac
done

check_one() {
	local changelog="$1"
	local crate_path="$2"
	local tag_prefix="$3"

	# Hard-fail when the `## [0.1.0]` block is absent entirely.
	if ! grep -qE '^## \[0\.1\.0\][^-]' "$changelog"; then
		printf "FAIL  %-50s missing `## [0.1.0]` block\n" "$crate_path"
		HARD_FAIL=1
		return
	fi

	# Locate the earliest alpha tag for this crate (fallback: rc.* tags, then skip).
	local first_tag
	first_tag=$(git tag -l "${tag_prefix}v0.1.0-alpha*" | sort -V | head -n1 || true)
	if [[ -z "$first_tag" ]]; then
		first_tag=$(git tag -l "${tag_prefix}v0.1.0-rc*" | sort -V | head -n1 || true)
	fi
	if [[ -z "$first_tag" ]]; then
		printf "SKIP  %-50s no alpha/rc tag for prefix %s\n" "$crate_path" "$tag_prefix"
		return
	fi

	local commits
	commits=$(git log "${first_tag}..HEAD" --oneline -- "$crate_path" 2>/dev/null | wc -l | tr -d ' ')
	local bullets
	bullets=$(awk '/^## \[0\.1\.0\][^-]/,/^## \[0\.1\.0-/' "$changelog" 2>/dev/null | grep -c '^- ' || true)

	# Hard-fail: real aggregation gap (many commits, zero bullets).
	if [[ "$commits" -gt "$FAIL_MIN_COMMITS" && "$bullets" -eq 0 ]]; then
		printf "FAIL  %-50s commits=%d bullets=0 (clear aggregation gap)\n" "$crate_path" "$commits"
		HARD_FAIL=1
		return
	fi

	# Guard: zero-commit crates (no new tag history) — report bullet count only.
	if [[ "$commits" -eq 0 ]]; then
		if [[ "$bullets" -gt 0 ]]; then
			printf "OK    %-50s bullets=%d (no new commits)\n" "$crate_path" "$bullets"
		else
			printf "LOW   %-50s empty 0.1.0 block (no commits, no bullets)\n" "$crate_path"
			SOFT_LOW=1
		fi
		return
	fi

	local ratio_pct=$(( bullets * 100 / commits ))
	if [[ "$ratio_pct" -lt "$WARN_RATIO_PCT" ]]; then
		printf "LOW   %-50s commits=%d bullets=%d (%d%% — below %d%%, informational)\n" "$crate_path" "$commits" "$bullets" "$ratio_pct" "$WARN_RATIO_PCT"
		SOFT_LOW=1
	else
		printf "OK    %-50s commits=%d bullets=%d (%d%%)\n" "$crate_path" "$commits" "$bullets" "$ratio_pct"
	fi
}

# Root CHANGELOG covers the whole workspace.
if [[ -f "CHANGELOG.md" ]]; then
	check_one "CHANGELOG.md" "." "reinhardt-web@"
fi

# Per-crate CHANGELOGs (including nested macros sub-crates).
while IFS= read -r changelog; do
	[[ -f "$changelog" ]] || continue
	crate_dir="$(dirname "$changelog")"
	rel_segments="${crate_dir#crates/}"
	parent="${rel_segments%%/*}"
	leaf="${rel_segments##*/}"
	if [[ "$parent" == "$leaf" ]]; then
		crate_name="$parent"
	else
		# Nested macros sub-crate: crates/reinhardt-pages/macros → reinhardt-pages-macros.
		crate_name="${parent}-${leaf}"
	fi
	check_one "$changelog" "$crate_dir" "${crate_name}@"
done < <(find crates -name CHANGELOG.md -type f | sort)

echo

if [[ "$HARD_FAIL" -ne 0 ]]; then
	echo "FAIL: one or more CHANGELOGs has a real aggregation gap (see FAIL rows)." >&2
	exit 1
fi

if [[ "$SOFT_LOW" -ne 0 && "$STRICT" -eq 1 ]]; then
	echo "Strict mode: LOW rows present (see above). Re-run without --strict to ignore." >&2
	exit 1
fi

if [[ "$SOFT_LOW" -ne 0 ]]; then
	echo "Coverage report: some crates flagged LOW. These are informational; LOW typically reflects chore/ci/test commits that legitimately do not produce CHANGELOG bullets. Use --strict to gate on them."
else
	echo "All CHANGELOGs look healthy."
fi
