#!/usr/bin/env bash
# Emit a shell-evaluable `export DATABASE_URL=...` line for the active profile.
#
# Why this exists: the management CLI (`manage migrate` / `makemigrations` /
# `runserver`) resolves its database connection via
# `DatabaseConnection::database_url_from(ctx.settings, $DATABASE_URL)`. For an
# example crate `ctx.settings` is always `None` — the framework does not wire
# the project's own `get_settings()` into the command context (tracked in
# kent8192/reinhardt-web#5042; the management runtime would otherwise read
# `settings/*.toml` directly). The CLI therefore falls back to the `DATABASE_URL` environment
# variable, which nothing sets, producing:
#   "No database URL available. Set DATABASE_URL environment variable."
#
# cargo-make runs each task in its own process, so exporting from
# `infra_up.sh` (a separate `infra-up` dependency task) would not reach the
# `cargo run` in the `migrate` task. Instead, the cargo-running tasks
# `eval "$(bash scripts/db_url.sh)"` in their own shell, so the URL is set in
# the same process that launches `cargo run`. Credentials come from the same
# settings TOML the container is provisioned from, keeping the connection
# string in lockstep with the active settings profile.
#
# Usage (from the crate root, inside a cargo-make task):
#   eval "$(bash scripts/db_url.sh)" && cargo run --bin manage migrate
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROFILE="${REINHARDT_ENV:-local}"
CONFIG="$CRATE_DIR/settings/${PROFILE}.toml"

if [ ! -f "$CONFIG" ]; then
	echo "Error: settings file for profile '${PROFILE}' not found at: $CONFIG" >&2
	exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
	echo "Error: python3 (>=3.11, for tomllib) is required to parse $CONFIG" >&2
	exit 1
fi

# parse_local_toml.py emits shell-quoted DB_* / PG_* / RD_* assignments.
SETTINGS=$(python3 "$SCRIPT_DIR/parse_local_toml.py" "$CONFIG") || exit $?
eval "$SETTINGS"

case "$DB_ENGINE" in
	sqlite)
		if [ "$DB_NAME" = ":memory:" ]; then
			DATABASE_URL="sqlite::memory:"
		elif [[ "$DB_NAME" = /* ]]; then
			DATABASE_URL="sqlite:///${DB_NAME}"
		else
			DATABASE_URL="sqlite:///${CRATE_DIR}/${DB_NAME}"
		fi
		;;
	postgresql|postgres)
		# Reuse python3's URL-encoding so passwords containing URL-reserved
		# characters (`@`, `:`, `/`, ...) survive being embedded in the
		# DATABASE_URL authority.
		ENCODED_USER=$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$PG_USER")
		ENCODED_PASS=$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$PG_PASS")
		DATABASE_URL="postgresql://${ENCODED_USER}:${ENCODED_PASS}@${PG_HOST}:${PG_PORT}/${PG_DB}"
		;;
	*)
		echo "Error: unsupported database engine '$DB_ENGINE'" >&2
		exit 1
		;;
esac

# Shell-quote the final value so the `eval` in the caller is safe.
printf 'export DATABASE_URL=%q\n' "$DATABASE_URL"
