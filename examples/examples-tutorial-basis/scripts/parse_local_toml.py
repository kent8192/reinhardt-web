#!/usr/bin/env python3
"""Parse an example crate's settings TOML and emit shell-evaluable KEY=VALUE lines.

Used by `scripts/infra_up.sh` to keep the example crate's runtime
configuration in a single source of truth. Despite the historical name,
this script accepts any settings profile (`local.toml`, `ci.toml`, ...);
the caller picks the file based on `REINHARDT_ENV`.

The settings shape mirrors the example crates' top-level `[database]`
section (see README "With Database" / `reinhardt_conf::settings::DatabaseConfig`).
Redis URL is read from a top-level `redis_url = "..."` key with a sane
default; examples that don't actually use Redis still get an idle
container spun up so the infra footprint is identical across examples.

Usage:
    parse_local_toml.py <path_to_settings.toml>

Stdout (one KEY=value per line, suitable for `eval`):
    PG_HOST=localhost
    PG_PORT=5432
    PG_DB=reinhardt
    PG_USER=reinhardt
    PG_PASS=reinhardt
    RD_HOST=localhost
    RD_PORT=6379

Values are shell-quoted via `shlex.quote` so `eval "$SETTINGS"` is safe
even if a password or hostname contains whitespace, quotes, or shell
metacharacters.

Exit codes:
    0  success
    1  parse / validation error (details on stderr)
    2  invalid CLI usage
"""

from __future__ import annotations

import shlex
import sys
import urllib.parse


def _q(value: object) -> str:
	# Shell-quote every emitted value so `eval "$SETTINGS"` in infra_up.sh
	# is safe against passwords / hostnames that contain whitespace, quotes,
	# or shell metacharacters.
	return shlex.quote(str(value))


def _load_toml(path: str) -> dict:
	try:
		import tomllib
	except ImportError:
		sys.stderr.write("Error: requires Python 3.11+ for tomllib\n")
		raise SystemExit(1)

	try:
		with open(path, "rb") as f:
			return tomllib.load(f)
	except FileNotFoundError:
		sys.stderr.write(f"Error: {path} not found\n")
		raise SystemExit(1)


def main(argv: list[str]) -> int:
	if len(argv) != 2:
		sys.stderr.write("Usage: parse_local_toml.py <settings.toml>\n")
		return 2

	data = _load_toml(argv[1])

	try:
		db = data["database"]
	except KeyError:
		sys.stderr.write(
			f"Error: top-level [database] missing from {argv[1]}\n"
		)
		return 1

	redis_url = data.get("redis_url", "redis://localhost:6379/0")
	parsed = urllib.parse.urlparse(redis_url)

	print(f"PG_HOST={_q(db.get('host', 'localhost'))}")
	print(f"PG_PORT={_q(db.get('port', 5432))}")
	print(f"PG_DB={_q(db.get('name', 'reinhardt'))}")
	print(f"PG_USER={_q(db.get('user', 'reinhardt'))}")
	print(f"PG_PASS={_q(db.get('password', 'reinhardt'))}")
	print(f"RD_HOST={_q(parsed.hostname or 'localhost')}")
	print(f"RD_PORT={_q(parsed.port or 6379)}")
	return 0


if __name__ == "__main__":
	sys.exit(main(sys.argv))
