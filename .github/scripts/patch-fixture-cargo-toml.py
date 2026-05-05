#!/usr/bin/env python3
"""Rewrite a generated fixture's Cargo.toml to point at this PR's HEAD
(workspace path) or at extracted .crate tarballs (publish-form mode).

Usage:
  patch-fixture-cargo-toml.py --manifest Cargo.toml --reinhardt-path /path/to/repo
  patch-fixture-cargo-toml.py --manifest Cargo.toml --use-packaged --pkg-stage /tmp/pkg-stage

Tracks: kent8192/reinhardt-web#4161
"""
from __future__ import annotations

import argparse
import re
import sys
import tarfile
from pathlib import Path


def workspace_form(manifest: Path, reinhardt_path: Path) -> None:
	"""Replace `reinhardt = { version = "..." }` with `path = "<reinhardt_path>"`."""
	text = manifest.read_text()
	pattern = re.compile(
		r'^reinhardt\s*=\s*\{\s*version\s*=\s*"[^"]*"\s*,',
		re.MULTILINE,
	)
	new_text, count = pattern.subn(
		f'reinhardt = {{ path = "{reinhardt_path}",',
		text,
	)
	if count == 0:
		print(
			"error: no `reinhardt = { version = \"...\" }` line found in manifest",
			file=sys.stderr,
		)
		sys.exit(2)
	manifest.write_text(new_text)


def _safe_extract(tf: tarfile.TarFile, dest: Path) -> None:
	"""Extract a tar archive into ``dest`` rejecting any member whose path
	would escape the destination (path traversal / zip-slip).

	`cargo package` tarballs are not attacker-controlled in our CI, but the
	guard is cheap insurance and keeps static analysers happy.
	"""
	dest_resolved = dest.resolve()
	for member in tf.getmembers():
		# Reject absolute paths, parent traversal, and device/symlink members.
		member_name = member.name
		if member_name.startswith("/") or ".." in Path(member_name).parts:
			raise RuntimeError(f"unsafe member in tarball: {member_name!r}")
		if member.issym() or member.islnk() or member.isdev():
			raise RuntimeError(f"unsupported member type in tarball: {member_name!r}")
		target = (dest / member_name).resolve()
		if not str(target).startswith(str(dest_resolved) + "/") and target != dest_resolved:
			raise RuntimeError(f"member escapes destination: {member_name!r}")
		tf.extract(member, dest)  # noqa: S202 — path validated above


def packaged_form(manifest: Path, pkg_stage: Path) -> None:
	"""Extract every `*.crate` under `pkg_stage` and append a `[patch.crates-io]`
	block to the manifest pointing each `reinhardt-*` crate at its extracted dir.
	"""
	extract_dir = pkg_stage / "extracted"
	extract_dir.mkdir(parents=True, exist_ok=True)

	crates: dict[str, Path] = {}
	for crate in sorted(pkg_stage.glob("*.crate")):
		with tarfile.open(crate, "r:gz") as tf:
			_safe_extract(tf, extract_dir)
		stem = crate.stem  # e.g. "reinhardt-web-0.1.0-rc.26"
		extracted = extract_dir / stem
		# Split on the first `-N` boundary — semver versions always start with a digit.
		m = re.match(r"^(?P<name>.+?)-(?P<version>\d.+)$", stem)
		if not m:
			print(f"warn: cannot parse crate stem {stem}", file=sys.stderr)
			continue
		crates[m.group("name")] = extracted

	patch_lines = ["", "[patch.crates-io]"]
	for name, path in sorted(crates.items()):
		if not name.startswith("reinhardt"):
			continue
		patch_lines.append(f'{name} = {{ path = "{path}" }}')

	if len(patch_lines) <= 2:
		print(
			"error: no reinhardt-* crates found in pkg-stage; nothing to patch",
			file=sys.stderr,
		)
		sys.exit(3)

	manifest.write_text(manifest.read_text() + "\n" + "\n".join(patch_lines) + "\n")


def main() -> int:
	ap = argparse.ArgumentParser()
	ap.add_argument("--manifest", required=True, type=Path)
	ap.add_argument("--reinhardt-path", type=Path)
	ap.add_argument("--use-packaged", action="store_true")
	ap.add_argument("--pkg-stage", type=Path)
	args = ap.parse_args()

	if not args.manifest.exists():
		print(f"error: manifest not found: {args.manifest}", file=sys.stderr)
		return 2

	if args.use_packaged:
		if args.pkg_stage is None:
			ap.error("--use-packaged requires --pkg-stage")
		packaged_form(args.manifest, args.pkg_stage)
	else:
		if args.reinhardt_path is None:
			ap.error("workspace form requires --reinhardt-path")
		workspace_form(args.manifest, args.reinhardt_path)

	return 0


if __name__ == "__main__":
	sys.exit(main())
