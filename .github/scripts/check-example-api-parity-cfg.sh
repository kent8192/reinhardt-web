#!/usr/bin/env bash
set -euo pipefail

paths=(
  "examples/examples-tutorial-basis/src/apps"
  "examples/examples-tutorial-basis/src/config"
  "examples/examples-tutorial-basis/src/shared"
  "examples/examples-tutorial-rest/src/apps"
  "examples/examples-tutorial-rest/src/config"
)

existing_paths=()
for path in "${paths[@]}"; do
  if [ -e "$path" ]; then
    existing_paths+=("$path")
  fi
done

if [ "${#existing_paths[@]}" -eq 0 ]; then
  echo "no accepted example paths found" >&2
  exit 1
fi

pattern='#\[cfg\((server|client|native|wasm|target_arch|not\(target_arch|all\(target_family|not\(all\(target_family)'
matches="$(rg -n "$pattern" "${existing_paths[@]}" -g '*.rs' || true)"
filtered="$(printf '%s\n' "$matches" | rg -v '#\[cfg\(test\)\]' || true)"

if [ -n "$filtered" ]; then
  printf '%s\n' "$filtered" >&2
  echo "target-specific cfg remains in accepted example app/config/shared paths" >&2
  exit 1
fi
