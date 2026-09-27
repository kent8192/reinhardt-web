#!/usr/bin/env python3
"""Verify the merged release PR and immutable source used for publishing."""

import argparse
import json
from pathlib import Path
import re
import sys
from tempfile import TemporaryDirectory

from release_state import (
    ReleaseError,
    ancestor,
    audit,
    command,
    git,
    packages,
    version_at,
)


def package_files(repo, package):
    """Ask Cargo for its include/exclude and implicit package-file boundaries."""
    directory = Path(package["manifest"]).parent
    listed = command(
        repo,
        "cargo",
        "package",
        "--list",
        "--offline",
        "--no-verify",
        "--allow-dirty",
        "--manifest-path",
        str(repo / package["manifest"]),
    )
    return {str(directory / name) for name in listed.splitlines()}


def published_package_changed(repo, published_source, source_sha, package):
    directory = str(Path(package["manifest"]).parent)
    changed = set(
        filter(
            None,
            git(
                repo,
                "diff",
                "--name-only",
                "--no-renames",
                "-z",
                published_source,
                source_sha,
                "--",
                directory,
            ).split("\0"),
        )
    )
    if not changed or directory != ".":
        return bool(changed)
    # A workspace root is also a package. Its sibling crates and CI files are
    # not necessarily published; Cargo is authoritative for glob semantics.
    current_files = package_files(repo, package)
    if changed & current_files:
        return True
    # Include the previous file inventory so deleted package files stay guarded.
    with TemporaryDirectory(prefix="release-package-files-", dir="/tmp") as temp:
        checkout = Path(temp) / "source"
        git(
            repo,
            "worktree",
            "add",
            "--quiet",
            "--detach",
            str(checkout),
            published_source,
        )
        try:
            previous_files = package_files(checkout, package)
        finally:
            # This invocation owns the disposable checkout and any generated files.
            git(repo, "worktree", "remove", "--force", str(checkout))
    return bool(changed & previous_files)


def verify(
    repo,
    repository,
    base,
    workflow_sha,
    source_sha,
    pr,
    resume,
    inventory=None,
    state=None,
):
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha) or not re.fullmatch(
        r"[0-9a-f]{40}", workflow_sha
    ):
        raise ReleaseError(
            "Source and workflow revisions must be full 40-character commit SHAs"
        )
    if base != "main" and not re.fullmatch(r"develop/\d+\.\d+\.\d+", base):
        raise ReleaseError("Publishing requires main or develop/X.Y.Z")
    if git(repo, "rev-parse", "HEAD") != source_sha:
        raise ReleaseError("Checked-out source does not match the requested SHA")
    if git(repo, "status", "--porcelain", "--untracked-files=no"):
        raise ReleaseError("Publication source has uncommitted changes")
    _, root_version = version_at(repo, source_sha, "Cargo.toml")
    if (base == "main" and not re.fullmatch(r"\d+\.\d+\.\d+", root_version)) or (
        base != "main"
        and not root_version.startswith(base.removeprefix("develop/") + "-")
    ):
        raise ReleaseError("Source version does not match the selected release line")
    prefix = "release-plz-" if base == "main" else "develop-release-plz-"
    if (
        not pr.get("merged")
        or pr.get("state") != "closed"
        or pr.get("base", {}).get("ref") != base
        or pr.get("base", {}).get("repo", {}).get("full_name") != repository
        or pr.get("head", {}).get("repo", {}).get("full_name") != repository
        or not pr.get("head", {}).get("ref", "").startswith(prefix)
        or "release" not in [label["name"] for label in pr.get("labels", [])]
    ):
        raise ReleaseError(
            "Expected a merged, release-labelled PR from the matching repository and release line"
        )
    merge_sha = pr.get("merge_commit_sha", "")
    if not re.fullmatch(r"[0-9a-f]{40}", merge_sha):
        raise ReleaseError("Release PR has no valid merge commit")
    if not resume:
        if source_sha != workflow_sha or source_sha != merge_sha:
            raise ReleaseError(
                "A normal release must publish its verified release-PR merge SHA"
            )
        return {"mode": "release", "source_sha": source_sha, "release_pr": pr["number"]}
    if not ancestor(repo, merge_sha, source_sha):
        raise ReleaseError("Recovery source must contain the original release merge")
    if not ancestor(repo, source_sha, workflow_sha):
        raise ReleaseError(
            "Recovery source must already be integrated into the selected release branch"
        )
    # Shared dependency and release policy changes belong to a subsequent release.
    if git(
        repo,
        "diff",
        "--name-only",
        merge_sha,
        source_sha,
        "--",
        "Cargo.toml",
        "release-plz.toml",
    ):
        raise ReleaseError(
            "Recovery must preserve the original root manifest and release configuration"
        )
    inventory = inventory if inventory is not None else packages(repo)
    for package in inventory:
        if version_at(repo, merge_sha, package["manifest"]) != (
            package["name"],
            package["version"],
        ):
            raise ReleaseError(
                f"Recovery changed the release version of {package['name']}"
            )
        if version_at(repo, workflow_sha, package["manifest"]) != (
            package["name"],
            package["version"],
        ):
            raise ReleaseError(
                f"The selected release branch has advanced past {package['name']}@{package['version']}"
            )
    original_manifests = git(
        repo, "ls-tree", "-r", "--name-only", merge_sha
    ).splitlines()
    source_manifests = git(
        repo, "ls-tree", "-r", "--name-only", source_sha
    ).splitlines()
    if {p for p in original_manifests if p.endswith("Cargo.toml")} != {
        p for p in source_manifests if p.endswith("Cargo.toml")
    }:
        raise ReleaseError("Recovery must preserve package membership")
    state = state if state is not None else audit(repo, repository, inventory)
    for package in state["packages"]:
        if package["published"]:
            # A prior recovery may already have published this exact repair.
            # Its verified tag is the source boundary on subsequent resumptions.
            published_source = (
                f"refs/tags/{package['tag']}"
                if package["tag"] and package["tag_exists"]
                else merge_sha
            )
            if published_package_changed(repo, published_source, source_sha, package):
                raise ReleaseError(
                    f"Recovery modifies already-published package {package['name']}"
                )
    if state["state"] == "complete":
        raise ReleaseError(
            "The selected release is already complete; no recovery is needed"
        )
    return {
        "mode": "resume-release",
        "source_sha": source_sha,
        "release_pr": pr["number"],
        "state": state,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--base", required=True)
    parser.add_argument("--workflow-sha", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--release-pr", type=int, required=True)
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    try:
        repo = args.repo.resolve()
        pr = json.loads(
            command(
                repo, "gh", "api", f"repos/{args.repository}/pulls/{args.release_pr}"
            )
        )
        report = verify(
            repo,
            args.repository,
            args.base,
            args.workflow_sha,
            args.source_sha,
            pr,
            args.resume,
        )
        print(json.dumps(report, indent=2))
    except (ReleaseError, ValueError, KeyError) as error:
        print(f"::error::{error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
