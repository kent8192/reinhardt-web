#!/usr/bin/env bash
# Stop disposable PostgreSQL + Redis containers started by infra_up.sh.
# Containers were launched with `docker run --rm`, so stopping them also
# removes them and their data.
set -e

docker stop \
	examples-twitter-postgres \
	examples-twitter-redis \
	>/dev/null 2>&1 || true
echo "Infrastructure stopped (containers auto-removed via --rm)"
