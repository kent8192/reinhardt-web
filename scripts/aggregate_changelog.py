#!/usr/bin/env python3
"""Aggregate alpha.* / rc.* CHANGELOG entries into a single [0.1.0] block.

Inserts a consolidated `## [0.1.0] - <date>` section immediately below
`## [Unreleased]` in the target CHANGELOG.md. Preserves all existing
`## [0.1.0-rc.N]` and `## [0.1.0-alpha.N]` blocks unchanged.

Usage:
    python3 scripts/aggregate_changelog.py CHANGELOG.md
    python3 scripts/aggregate_changelog.py --release-date 2026-05-22 CHANGELOG.md
    python3 scripts/aggregate_changelog.py --dry-run CHANGELOG.md  # prints diff
    python3 scripts/aggregate_changelog.py --target-version 0.1.0 CHANGELOG.md

The script auto-detects the crate name from existing compare URLs in the
target file (e.g., `compare/reinhardt-core@v0.1.0-rc.29...`).
"""

from __future__ import annotations

import argparse
import datetime as _dt
import difflib
import re
import sys
from collections import OrderedDict
from pathlib import Path

# Output section order: Keep a Changelog first, then release-plz extras.
SECTION_ORDER = [
	"Breaking Changes",
	"Added",
	"Changed",
	"Deprecated",
	"Removed",
	"Fixed",
	"Security",
	"Performance",
	"Documentation",
	"Maintenance",
	"Testing",
	"Styling",
	"Reverted",
	"Other",
]

# Patterns that flood release notes; collapse to a single counter line.
NOISE_PATTERNS = [
	(re.compile(r"^- apply CodeRabbit auto-fixes\b", re.IGNORECASE), "apply CodeRabbit auto-fixes"),
	(re.compile(r"^- apply Copilot review\b", re.IGNORECASE), "apply Copilot review"),
	(re.compile(r"^- address Copilot review (feedback|on PR|threads on PR)\b", re.IGNORECASE), "address Copilot review feedback"),
	(re.compile(r"^- address CodeRabbit review (feedback|on PR)\b", re.IGNORECASE), "address CodeRabbit review feedback"),
]

# Lines that are placeholders or non-informative boilerplate. Dropped silently.
SKIP_PATTERNS = [
	re.compile(r"^- N/A\s*$", re.IGNORECASE),
	re.compile(r"^- Work in progress features\b", re.IGNORECASE),
	re.compile(r"^- Initial crates\.io release\s*$", re.IGNORECASE),
	re.compile(r"^- Initial release\s*$", re.IGNORECASE),
	# Sub-Crate Updates template placeholder (literal documentation artifact).
	re.compile(r"^- `\[crate-name\]` updated to v\[version\]"),
]

VERSION_HEADER = re.compile(
	r"^## \[(?P<version>0\.1\.0-(?:alpha|rc)\.\d+)\](?:\([^)]+\))?\s*(?:-\s*(?P<date>\S+))?\s*$"
)
SUBSECTION_HEADER = re.compile(r"^### (?P<name>[A-Za-z][A-Za-z0-9 /-]*)\s*$")
COMPARE_URL = re.compile(
	r"compare/(?P<crate>[A-Za-z0-9_-]+)@v0\.1\.0-(?:alpha|rc)\.\d+"
)
BREAKING_INLINE = re.compile(r"\[\*\*breaking\*\*\]", re.IGNORECASE)
UNRELEASED_HEADER = re.compile(r"^## \[Unreleased\]\s*$")


def detect_crate_name(text: str) -> str:
	"""Auto-detect the crate name from any compare URL in the file."""
	m = COMPARE_URL.search(text)
	if not m:
		raise SystemExit(
			"Could not auto-detect crate name from compare URLs in the file. "
			"Pass --crate <name> explicitly."
		)
	return m.group("crate")


def parse_existing_sections(text: str) -> "OrderedDict[str, list[str]]":
	"""Walk every `## [0.1.0-(alpha|rc).N]` block and collect bullet items per `### Section`.

	Returns a mapping section_name -> list of bullet lines (in encounter order),
	with case-insensitive de-duplication preserving the first occurrence.
	Noise patterns are collapsed.
	"""
	sections: "OrderedDict[str, list[str]]" = OrderedDict()
	seen_per_section: dict[str, set[str]] = {}
	noise_counts: dict[str, int] = {pattern_label: 0 for _, pattern_label in NOISE_PATTERNS}
	noise_section: dict[str, str] = {}

	lines = text.splitlines()
	i = 0
	in_target = False
	current_section: str | None = None

	while i < len(lines):
		line = lines[i]

		if VERSION_HEADER.match(line):
			in_target = True
			current_section = None
			i += 1
			continue

		# Non-target version header => stop processing previous block.
		if line.startswith("## ") and not VERSION_HEADER.match(line):
			in_target = False
			current_section = None
			i += 1
			continue

		if not in_target:
			i += 1
			continue

		sub_match = SUBSECTION_HEADER.match(line)
		if sub_match:
			current_section = sub_match.group("name").strip()
			sections.setdefault(current_section, [])
			seen_per_section.setdefault(current_section, set())
			i += 1
			continue

		if current_section and line.startswith("- "):
			# Drop pure placeholder / boilerplate lines.
			if any(pat.match(line) for pat in SKIP_PATTERNS):
				i += 1
				continue

			# Collapse noise lines.
			collapsed = False
			for pattern, label in NOISE_PATTERNS:
				if pattern.match(line):
					noise_counts[label] += 1
					noise_section.setdefault(label, current_section)
					collapsed = True
					break
			if collapsed:
				i += 1
				continue

			# Multi-line bullets (continuation indented two spaces).
			bullet_lines = [line]
			j = i + 1
			while j < len(lines) and lines[j].startswith("  "):
				bullet_lines.append(lines[j])
				j += 1
			bullet_text = "\n".join(bullet_lines)
			normalized = re.sub(r"\s+", " ", bullet_text.lower()).strip()
			if normalized not in seen_per_section[current_section]:
				seen_per_section[current_section].add(normalized)
				sections[current_section].append(bullet_text)
			i = j
			continue

		i += 1

	# Emit collapsed noise as a single counter line in its host section.
	for label, count in noise_counts.items():
		if count == 0:
			continue
		host = noise_section.get(label, "Fixed")
		sections.setdefault(host, [])
		sections[host].append(f"- {label} (consolidated across {count} occurrences)")

	return sections


def split_breaking(sections: "OrderedDict[str, list[str]]") -> "OrderedDict[str, list[str]]":
	"""Pull `[**breaking**]` items out of their original sections into Breaking Changes."""
	breaking: list[str] = []
	for name in list(sections.keys()):
		kept: list[str] = []
		for bullet in sections[name]:
			if BREAKING_INLINE.search(bullet):
				normalized = bullet if bullet.startswith("- ") else f"- {bullet}"
				breaking.append(normalized.rstrip())
			else:
				kept.append(bullet)
		sections[name] = kept

	if breaking:
		# Move Breaking Changes to the front per SECTION_ORDER.
		ordered: "OrderedDict[str, list[str]]" = OrderedDict()
		ordered["Breaking Changes"] = breaking
		for k, v in sections.items():
			if k != "Breaking Changes":
				ordered[k] = v
		return ordered
	return sections


def render_block(
	sections: "OrderedDict[str, list[str]]",
	crate: str,
	target_version: str,
	release_date: str,
	previous_tag: str,
) -> str:
	"""Render the [0.1.0] block with a compare URL header and ordered sections."""
	compare = (
		f"https://github.com/kent8192/reinhardt-web/compare/"
		f"{crate}@{previous_tag}...{crate}@v{target_version}"
	)
	header = f"## [{target_version}]({compare}) - {release_date}"
	out: list[str] = [header, ""]

	emitted = set()
	for name in SECTION_ORDER:
		bullets = sections.get(name)
		if not bullets:
			continue
		out.append(f"### {name}")
		out.append("")
		out.extend(bullets)
		out.append("")
		emitted.add(name)

	# Any unknown section names not in SECTION_ORDER → append in their original order.
	for name, bullets in sections.items():
		if name in emitted or not bullets:
			continue
		out.append(f"### {name}")
		out.append("")
		out.extend(bullets)
		out.append("")

	return "\n".join(out).rstrip() + "\n"


def latest_existing_tag(text: str) -> str:
	"""Return the largest existing `0.1.0-(alpha|rc).N` version from the file."""
	versions = re.findall(r"\[0\.1\.0-(?:alpha|rc)\.\d+\]", text)
	if not versions:
		raise SystemExit("No 0.1.0-alpha or 0.1.0-rc version headers found in file.")

	def sort_key(v: str) -> tuple[int, int]:
		# v looks like `[0.1.0-rc.30]` — sort rc > alpha, then by number.
		stage = 1 if "-rc." in v else 0
		n = int(re.search(r"\.(\d+)\]$", v).group(1))
		return (stage, n)

	versions.sort(key=sort_key, reverse=True)
	# Strip surrounding brackets, prefix with `v` (tag format).
	return "v" + versions[0].strip("[]")


def inject_block(text: str, block: str) -> str:
	"""Insert the rendered block immediately below `## [Unreleased]`."""
	lines = text.splitlines(keepends=True)
	for idx, line in enumerate(lines):
		if UNRELEASED_HEADER.match(line.rstrip("\n")):
			# Skip blank lines after the Unreleased header until next content.
			insert_at = idx + 1
			while insert_at < len(lines) and lines[insert_at].strip() == "":
				insert_at += 1
			prefix = "".join(lines[:insert_at])
			suffix = "".join(lines[insert_at:])
			separator = "" if prefix.endswith("\n\n") else ("\n" if prefix.endswith("\n") else "\n\n")
			return prefix + separator + block + "\n" + suffix
	raise SystemExit("Could not locate `## [Unreleased]` header to insert below.")


def main() -> int:
	p = argparse.ArgumentParser(description="Aggregate alpha/rc CHANGELOGs into a stable [X.Y.Z] block.")
	p.add_argument("changelog", type=Path, help="Path to CHANGELOG.md")
	p.add_argument("--release-date", default=_dt.date.today().isoformat(), help="Release date for the new entry (default: today)")
	p.add_argument("--target-version", default="0.1.0", help="Stable version to write (default: 0.1.0)")
	p.add_argument("--crate", default=None, help="Crate name (auto-detected from compare URLs if omitted)")
	p.add_argument("--dry-run", action="store_true", help="Print unified diff instead of writing the file")
	args = p.parse_args()

	text = args.changelog.read_text(encoding="utf-8")
	crate = args.crate or detect_crate_name(text)
	previous_tag = latest_existing_tag(text)
	sections = parse_existing_sections(text)
	sections = split_breaking(sections)

	if not any(sections.values()):
		print(f"NOTE: no aggregatable content found in {args.changelog}", file=sys.stderr)
		return 0

	block = render_block(sections, crate, args.target_version, args.release_date, previous_tag)
	new_text = inject_block(text, block)

	if args.dry_run:
		diff = difflib.unified_diff(
			text.splitlines(keepends=True),
			new_text.splitlines(keepends=True),
			fromfile=str(args.changelog),
			tofile=str(args.changelog) + " (aggregated)",
		)
		sys.stdout.writelines(diff)
		return 0

	args.changelog.write_text(new_text, encoding="utf-8")
	print(f"Aggregated {args.changelog} (crate={crate}, previous={previous_tag}, target=v{args.target_version}, date={args.release_date})")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
