#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <package> [features]" >&2
  echo "set NO_DEFAULT_FEATURES=1 to pass --no-default-features" >&2
  exit 2
fi

package="$1"
features="${2:-}"
blocked_regex='(^| )(sqlx|tokio|refinery|testcontainers|native-tls|mio|hyper-util) v'

args=(tree --target wasm32-unknown-unknown -p "$package")
if [ "${NO_DEFAULT_FEATURES:-0}" = "1" ]; then
  args+=(--no-default-features)
fi
if [ -n "$features" ]; then
  args+=(--features "$features")
fi

tree_output="$(cargo "${args[@]}")"
if printf '%s\n' "$tree_output" | rg "$blocked_regex"; then
  echo "native-only dependency leaked into wasm graph for package $package" >&2
  exit 1
fi
