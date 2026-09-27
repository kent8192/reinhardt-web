#!/usr/bin/env python3
"""Reconcile exact workspace versions with crates.io, Git tags, and releases."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen


class ReleaseError(RuntimeError):
    """A release prerequisite could not be established."""


def command(repo, *args):
    result = subprocess.run(args, cwd=repo, text=True, capture_output=True)
    if result.returncode:
        raise ReleaseError(
            result.stderr.strip() or f"{args[0]} exited {result.returncode}"
        )
    return result.stdout.strip()


def git(repo, *args):
    return command(repo, "git", *args)


def ancestor(repo, older, newer):
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", older, newer],
        cwd=repo,
        text=True,
        capture_output=True,
    )
    if result.returncode not in (0, 1):
        raise ReleaseError(result.stderr.strip())
    return result.returncode == 0


def packages(repo):
    repo = repo.resolve()
    metadata = json.loads(
        command(repo, "cargo", "metadata", "--no-deps", "--format-version", "1")
    )
    config = tomllib.loads((repo / "release-plz.toml").read_text())
    defaults = config.get("workspace", {})
    overrides = {item["name"]: item for item in config.get("package", [])}
    result = []
    for package in metadata["packages"]:
        if (
            package["id"] not in metadata["workspace_members"]
            or package.get("publish") == []
        ):
            continue
        options = defaults | overrides.get(package["name"], {})
        if not options.get("release", True):
            continue
        if package.get("publish") not in (None, ["crates-io"]):
            raise ReleaseError(
                f"Unsupported registry for {package['name']}; expected crates.io"
            )
        name, version = package["name"], package["version"]
        tag = options.get("git_tag_name", "{{ package }}@v{{ version }}")
        tag = tag.replace("{{ package }}", name).replace("{{ version }}", version)
        if "{{" in tag or not re.fullmatch(r"[A-Za-z0-9_.@+-]+", tag):
            raise ReleaseError(f"Unsupported release tag template: {tag}")
        result.append(
            {
                "name": name,
                "version": version,
                "manifest": str(
                    Path(package["manifest_path"]).resolve().relative_to(repo)
                ),
                "publish": options.get("publish", True),
                "tag": tag if options.get("git_tag_enable", True) else None,
                "git_release": options.get("git_release_enable", True),
            }
        )
    if not result:
        raise ReleaseError("No release-enabled packages found")
    return sorted(result, key=lambda item: item["name"])


def previous_release_packages(repo):
    """Do not mistake packages added after a completed facade release for a partial release."""
    inventory = packages(repo)
    root = next(item for item in inventory if item["manifest"] == "Cargo.toml")
    tag = root["tag"]
    if not tag or not git(repo, "tag", "--list", tag):
        return inventory
    manifests = set(
        git(repo, "ls-tree", "-r", "--name-only", f"refs/tags/{tag}").splitlines()
    )
    return [item for item in inventory if item["manifest"] in manifests]


def registry_version(name, version, opener=urlopen):
    name = name.lower()
    if len(name) == 1:
        path = f"1/{name}"
    elif len(name) == 2:
        path = f"2/{name}"
    elif len(name) == 3:
        path = f"3/{name[0]}/{name}"
    else:
        path = f"{name[:2]}/{name[2:4]}/{name}"
    request = Request(
        f"https://index.crates.io/{path}",
        headers={
            "User-Agent": "reinhardt-release-state (github.com/kent8192/reinhardt-web)"
        },
    )
    try:
        with opener(request, timeout=30) as response:
            entries = [
                json.loads(line)
                for line in response.read().decode().splitlines()
                if line
            ]
    except HTTPError as error:
        if error.code == 404:
            return False
        raise ReleaseError(
            f"Registry lookup for {name} failed: HTTP {error.code}"
        ) from error
    except (OSError, ValueError) as error:
        raise ReleaseError(f"Registry lookup for {name} failed: {error}") from error
    if not entries or any(
        entry.get("name", "").lower() != name or "vers" not in entry
        for entry in entries
    ):
        raise ReleaseError(f"Malformed registry index for {name}")
    for entry in entries:
        if entry["vers"] == version:
            if entry.get("yanked"):
                raise ReleaseError(
                    f"{name}@{version} is yanked; it cannot be republished"
                )
            return True
    return False


def github_release(repo, repository, tag):
    result = subprocess.run(
        ["gh", "api", f"repos/{repository}/releases/tags/{quote(tag, safe='')}"],
        cwd=repo,
        text=True,
        capture_output=True,
    )
    if result.returncode:
        if "HTTP 404" in result.stderr:
            return False
        raise ReleaseError(f"GitHub release lookup failed: {result.stderr.strip()}")
    release = json.loads(result.stdout)
    return release.get("tag_name") == tag and release.get("draft") is False


def version_at(repo, revision, manifest):
    data = tomllib.loads(git(repo, "show", f"{revision}:{manifest}"))["package"]
    version = data["version"]
    if isinstance(version, dict) and version.get("workspace"):
        root = tomllib.loads(git(repo, "show", f"{revision}:Cargo.toml"))
        version = root["workspace"]["package"]["version"]
    return data["name"], version


def audit(
    repo, repository, inventory=None, registry=registry_version, release=github_release
):
    inventory = inventory if inventory is not None else packages(repo)
    with ThreadPoolExecutor(max_workers=4) as executor:
        published = list(
            executor.map(
                lambda item: (
                    registry(item["name"], item["version"]) if item["publish"] else None
                ),
                inventory,
            )
        )
    rows = []
    for package, is_published in zip(inventory, published):
        tag = package["tag"]
        tag_exists = not tag
        if tag:
            tag_exists = bool(git(repo, "tag", "--list", tag))
            if tag_exists:
                commit = git(
                    repo, "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}"
                )
                if not ancestor(repo, commit, "HEAD"):
                    raise ReleaseError(
                        f"Release tag {tag} is not an ancestor of the selected source"
                    )
                if version_at(repo, commit, package["manifest"]) != (
                    package["name"],
                    package["version"],
                ):
                    raise ReleaseError(
                        f"Release tag {tag} contains a different package/version"
                    )
        release_exists = not package["git_release"]
        if package["git_release"]:
            if not tag:
                raise ReleaseError(
                    f"GitHub release without a tag is unsupported: {package['name']}"
                )
            release_exists = tag_exists and release(repo, repository, tag)
        rows.append(
            package
            | {
                "published": is_published,
                "tag_exists": tag_exists,
                "release_exists": release_exists,
            }
        )
    unpublished = [
        f"{row['name']}@{row['version']}" for row in rows if row["published"] is False
    ]
    missing_tags = [row["tag"] for row in rows if not row["tag_exists"]]
    missing_releases = [row["tag"] for row in rows if not row["release_exists"]]
    complete = not (unpublished or missing_tags or missing_releases)
    return {
        "state": "complete" if complete else "incomplete",
        "source_sha": git(repo, "rev-parse", "HEAD"),
        "unpublished": unpublished,
        "missing_tags": missing_tags,
        "missing_releases": missing_releases,
        "packages": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--repository", required=True)
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument(
        "--before-release-pr",
        action="store_true",
        help="Exclude new packages introduced after the completed facade release",
    )
    args = parser.parse_args()
    try:
        repo = args.repo.resolve()
        inventory = previous_release_packages(repo) if args.before_release_pr else None
        report = audit(repo, args.repository, inventory)
        print(json.dumps(report, indent=2))
        if args.require_complete and report["state"] != "complete":
            raise ReleaseError(
                "Release is incomplete; hold the next Release PR and use resume-release with "
                "the merged release PR number and a verified recovery SHA. "
                f"Unpublished: {', '.join(report['unpublished']) or 'none'}; "
                f"missing tags: {', '.join(report['missing_tags']) or 'none'}; "
                f"missing GitHub releases: {', '.join(report['missing_releases']) or 'none'}"
            )
    except ReleaseError as error:
        print(f"::error::{error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
