#!/usr/bin/env python3
"""Constrain AWS dependencies only in a disposable release baseline checkout."""

import json
from pathlib import Path
import subprocess
import sys


def prepare(root):
    root = root.resolve()
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--no-deps", "--format-version", "1",
        "--manifest-path", str(root / "Cargo.toml"),
    ], cwd=root, text=True))
    edits = []
    for package in metadata["packages"]:
        dependencies = {dep["name"]: dep for dep in package["dependencies"]}
        if "aws-config" not in dependencies:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        manifest.relative_to(root)
        if "aws-smithy-types" in dependencies:
            if dependencies["aws-smithy-types"]["req"] != "=1.6.3":
                raise ValueError(f"Unexpected AWS compatibility constraint in {manifest}")
            continue
        # Workaround for smithy-lang/smithy-rs#4853 (reinhardt-web#6318).
        # Remove when published baseline dependencies compile without the pin.
        # Ideal implementation: use the release-tag manifests without edits.
        edits.append((manifest, manifest.read_text() +
                      '\n[dependencies.aws-smithy-types]\nversion = "=1.6.3"\n'))
    for manifest, content in edits:
        manifest.write_text(content)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
