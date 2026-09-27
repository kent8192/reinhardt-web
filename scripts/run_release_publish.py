#!/usr/bin/env python3
"""Publish with bounded retries for transient registry failures only."""

import argparse
from collections import deque
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
import re
import signal
import subprocess
import sys
import time


def retry_delay(log, attempt, now=None):
    # A permanent packaging error takes precedence over transient log messages.
    if re.search(
        r"failed to select a version|no matching package|failed to parse manifest|"
        r"failed to verify|permission denied|status(?: code)?[ :]+(?:401|403)\b",
        log,
        re.I,
    ):
        return None
    if re.search(
        r"(?:status(?: code)?|HTTP[/\d.]*)[ :]+429\b|too many requests", log, re.I
    ):
        header = re.search(r"retry-after:\s*(\d+)", log, re.I)
        if header:
            return min(int(header[1]) + 5, 3600)
        deadline = re.search(
            r"(?:try again after|retry-after:)\s*([^\n]+?GMT)", log, re.I
        )
        if deadline:
            try:
                current = now or datetime.now(timezone.utc)
                seconds = (parsedate_to_datetime(deadline[1]) - current).total_seconds()
                return min(max(int(seconds) + 5, 5), 3600)
            except (ValueError, TypeError):
                pass
        return 120 * attempt
    if re.search(
        r"(?:status(?: code)?|HTTP[/\d.]*)[ :]+5\d\d\b|"
        r"connection (?:reset|timed out)|operation timed out|could not resolve host|"
        r"failed to lookup address|connection closed before message completed",
        log,
        re.I,
    ):
        return 30 * attempt
    return None


def run_attempt(args, repo):
    lines = deque(maxlen=500)
    with subprocess.Popen(
        args, cwd=repo, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
    ) as process:
        try:
            for line in process.stdout:
                print(line, end="", flush=True)
                lines.append(line)
            code = process.wait()
        except BaseException:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise
    return code, "".join(lines)


def publish(args, repo, runner=run_attempt, sleep=time.sleep):
    for attempt in range(1, 4):
        print(f"Publish attempt {attempt}/3", flush=True)
        code, log = runner(args, repo)
        if code == 0:
            return 0
        delay = retry_delay(log, attempt)
        if delay is None or attempt == 3:
            print(
                "::error::Publishing failed; no further automatic retry.",
                file=sys.stderr,
            )
            return code
        print(f"Transient publish failure; retrying in {delay}s.", flush=True)
        sleep(delay)
    return 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--dry-run", action="store_true")
    options = parser.parse_args()
    repo = options.repo.resolve()
    args = [
        "release-plz",
        "release",
        "--manifest-path",
        str(repo / "Cargo.toml"),
        "--config",
        str(repo / "release-plz.toml"),
        "--repo-url",
        f"https://github.com/{options.repository}",
    ]
    if options.dry_run:
        args.append("--dry-run")

    def stop(_signal, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, stop)
    try:
        return publish(args, repo)
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())
