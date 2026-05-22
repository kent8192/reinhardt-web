#!/usr/bin/env bash
# Start disposable PostgreSQL + Redis containers via `docker run --rm`.
#
# Connection parameters (user / password / database / port / redis URL)
# are parsed from the settings TOML matching the active `REINHARDT_ENV`
# profile (defaults to `local`). Reading the same profile that the
# example's `src/config/settings.rs` resolves keeps the provisioned
# container credentials in sync with what `runserver` later connects
# with — mirrors the pattern from `reinhardt-cloud/dashboard/scripts/infra_up.sh`.
#
# Container names are namespaced by the example crate so concurrent
# `cargo make runserver` invocations across examples don't collide on
# `--name`. Ports default to 5432 / 6379, so running two examples
# side-by-side still requires distinct ports in `settings/local.toml`.
set -euo pipefail

PG_NAME="examples-tutorial-basis-postgres"
RD_NAME="examples-tutorial-basis-redis"

# Resolve the crate root from this script's location so the task works
# whether invoked through cargo-make or directly via
# `bash examples-tutorial-basis/scripts/infra_up.sh`.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROFILE="${REINHARDT_ENV:-local}"
CONFIG="$CRATE_DIR/settings/${PROFILE}.toml"

if [ ! -f "$CONFIG" ]; then
	echo "Error: settings file for profile '${PROFILE}' not found at: $CONFIG" >&2
	if [ -f "$CRATE_DIR/settings/${PROFILE}.example.toml" ]; then
		echo "  Run: cp $CRATE_DIR/settings/${PROFILE}.example.toml $CONFIG" >&2
		echo "       and fill in any required secrets before retrying." >&2
	else
		echo "  No example template exists for profile '${PROFILE}'." >&2
		echo "  Either create $CONFIG manually or unset REINHARDT_ENV to use 'local'." >&2
	fi
	exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
	echo "Error: python3 (>=3.11, for tomllib) is required to parse $CONFIG" >&2
	exit 1
fi

SETTINGS=$(python3 "$SCRIPT_DIR/parse_local_toml.py" "$CONFIG") || exit $?
eval "$SETTINGS"

# Drop any stale containers from a previous aborted run so --name is free.
docker rm -f "$PG_NAME" "$RD_NAME" >/dev/null 2>&1 || true

echo "Using settings profile '${PROFILE}' (from $CONFIG)"
echo "Starting PostgreSQL ($PG_NAME) on ${PG_HOST}:${PG_PORT} as ${PG_USER}/${PG_DB}..."
docker run --rm -d \
	--name "$PG_NAME" \
	-p "${PG_PORT}:5432" \
	-e POSTGRES_USER="$PG_USER" \
	-e POSTGRES_PASSWORD="$PG_PASS" \
	-e POSTGRES_DB="$PG_DB" \
	postgres:17 >/dev/null

echo "Starting Redis ($RD_NAME) on ${RD_HOST}:${RD_PORT}..."
docker run --rm -d \
	--name "$RD_NAME" \
	-p "${RD_PORT}:6379" \
	redis:7-alpine >/dev/null

echo "Waiting for PostgreSQL..."
for _ in $(seq 1 30); do
	if docker exec "$PG_NAME" pg_isready -U "$PG_USER" -d "$PG_DB" >/dev/null 2>&1; then
		echo "  PostgreSQL ready"
		break
	fi
	sleep 1
done

echo "Waiting for Redis..."
for _ in $(seq 1 30); do
	if docker exec "$RD_NAME" redis-cli ping 2>/dev/null | grep -q PONG; then
		echo "  Redis ready"
		break
	fi
	sleep 1
done

echo "Infrastructure ready. Run 'cargo make infra-down' to stop."
