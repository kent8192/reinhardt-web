#!/usr/bin/env bash
# Emit a shell-evaluable DOCKER_HOST fallback for local Docker-compatible sockets.
#
# The repository-level .testcontainers.properties defaults to Docker Desktop's
# /var/run/docker.sock. OrbStack exposes a Docker-compatible socket under the
# user's home directory, and TestContainers honors DOCKER_HOST ahead of the
# properties file, so example tasks can opt in without changing global project
# behavior.
set -euo pipefail

if [ -n "${DOCKER_HOST:-}" ]; then
	exit 0
fi

if [ -S /var/run/docker.sock ]; then
	exit 0
fi

ORBSTACK_SOCKET="${HOME}/.orbstack/run/docker.sock"
if [ -S "$ORBSTACK_SOCKET" ]; then
	printf 'export DOCKER_HOST=%q\n' "unix://${ORBSTACK_SOCKET}"
fi
