#!/bin/bash
# Run pre-start cleanup, then delegate to the original entrypoint.
if ! /usr/local/bin/pre-start-cleanup.sh; then
  status=$?
  echo "Warning: pre-start-cleanup.sh failed with exit code ${status}; continuing to start container." >&2
fi
exec /entrypoint.sh "$@"
