#!/usr/bin/env python3
"""Constrain incompatible dependencies only in a disposable release baseline checkout."""

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
        constraints = {}
        if "aws-config" in dependencies:
            # Workaround for smithy-lang/smithy-rs#4853 (reinhardt-web#6318).
            # Remove when published baseline dependencies compile without the pin.
            # Ideal implementation: use the release-tag manifests without edits.
            constraints["aws-smithy-types"] = "=1.6.3"
        if dependencies.get("evcxr", {}).get("req") == "^0.22.0":
            # Compatibility constraints for the published evcxr 0.22 lexer and
            # rust-analyzer graph, tracked in reinhardt-web#6341 and #6342.
            # Upstream: rust-lang/rust#163012, rust-lang/rust-analyzer#23394.
            # Remove when release baselines resolve a compatible graph unaided.
            # Ideal implementation: use the release-tag manifests without edits.
            constraints.update({
                "unicode-ident": "=1.0.24",
                "salsa": "=0.28.2",
                "salsa-macro-rules": "=0.28.2",
            })
        if not constraints:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        manifest.relative_to(root)
        content = manifest.read_text()
        for name, requirement in constraints.items():
            if name in dependencies:
                if dependencies[name]["req"] != requirement:
                    raise ValueError(f"Unexpected {name} compatibility constraint in {manifest}")
                continue
            content += f'\n[dependencies.{name}]\nversion = "{requirement}"\n'
            if name == "salsa":
                content += 'default-features = false\n'
        edits.append((manifest, content))
    for manifest, content in edits:
        manifest.write_text(content)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
