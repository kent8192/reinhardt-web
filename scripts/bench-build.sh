#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/bench-build.sh [--dry-run] [--runs N] [--warmup N] [--output PATH] [SCENARIO...]

Scenarios:
  cold-check-all-features
  cold-build-standard
  cold-test-build
  incremental-leaf-check
  incremental-core-check
  incremental-db-macro-check
  incremental-page-macro-check
  incremental-pages-wasm-check
  incremental-pages-wasm-build
  incremental-server-build
  incremental-hot-reload-client-legacy-both-build
  incremental-hot-reload-server-legacy-both-build
  incremental-leaf-build

The script records reproducible build-loop timings for Reinhardt Pages and
workspace build-performance work. It requires hyperfine unless --dry-run is set.
USAGE
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

runs=3
warmup=1
dry_run=0
output=""
declare -a requested=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run)
      dry_run=1
      shift
      ;;
    --runs)
      runs="${2:?--runs requires a value}"
      shift 2
      ;;
    --warmup)
      warmup="${2:?--warmup requires a value}"
      shift 2
      ;;
    --output)
      output="${2:?--output requires a value}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      requested+=("$1")
      shift
      ;;
  esac
done

if [ "${#requested[@]}" -eq 0 ]; then
  requested=(
    cold-check-all-features
    cold-build-standard
    cold-test-build
    incremental-leaf-check
    incremental-core-check
    incremental-db-macro-check
    incremental-page-macro-check
    incremental-pages-wasm-check
    incremental-pages-wasm-build
    incremental-server-build
    incremental-hot-reload-client-legacy-both-build
    incremental-hot-reload-server-legacy-both-build
    incremental-leaf-build
  )
fi

if [ -z "$output" ]; then
  stamp="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
  output="docs/build-perf/baseline-${stamp}.json"
fi

scenario_command() {
  case "$1" in
    cold-check-all-features)
      printf '%s\n' 'cargo clean && cargo check --workspace --all-features'
      ;;
    cold-build-standard)
      printf '%s\n' 'cargo clean && cargo build --workspace'
      ;;
    cold-test-build)
      printf '%s\n' 'cargo clean && cargo nextest run --workspace --all-features --no-run'
      ;;
    incremental-leaf-check)
      printf '%s\n' 'touch crates/reinhardt-throttling/src/lib.rs && cargo check -p reinhardt-throttling'
      ;;
    incremental-core-check)
      printf '%s\n' 'touch crates/reinhardt-core/src/lib.rs && cargo check --workspace'
      ;;
    incremental-db-macro-check)
      printf '%s\n' 'touch crates/reinhardt-db-macros/src/lib.rs && cargo check --workspace'
      ;;
    incremental-page-macro-check)
      printf '%s\n' 'touch crates/reinhardt-pages/macros/src/lib.rs && cargo check --workspace'
      ;;
    incremental-pages-wasm-check)
      printf '%s\n' 'touch crates/reinhardt-pages/src/component.rs && cargo check -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full'
      ;;
    incremental-pages-wasm-build)
      printf '%s\n' 'touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full'
      ;;
    incremental-server-build)
      printf '%s\n' 'touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server'
      ;;
    incremental-hot-reload-client-legacy-both-build)
      printf '%s\n' 'touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full && cargo build -p reinhardt-server'
      ;;
    incremental-hot-reload-server-legacy-both-build)
      printf '%s\n' 'touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full'
      ;;
    incremental-leaf-build)
      printf '%s\n' 'touch crates/reinhardt-throttling/src/lib.rs && cargo build -p reinhardt-throttling'
      ;;
    *)
      echo "unknown scenario: $1" >&2
      exit 2
      ;;
  esac
}

declare -a hyperfine_args=()
for scenario in "${requested[@]}"; do
  command="$(scenario_command "$scenario")"
  if [ "$dry_run" -eq 1 ]; then
    printf '%s: %s\n' "$scenario" "$command"
  else
    hyperfine_args+=(--command-name "$scenario" "$command")
  fi
done

if [ "$dry_run" -eq 1 ]; then
  exit 0
fi

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "hyperfine is required. Install it before collecting build benchmarks." >&2
  exit 127
fi

mkdir -p "$(dirname "$output")"
hyperfine --warmup "$warmup" --runs "$runs" --export-json "$output" "${hyperfine_args[@]}"

{
  printf 'rustc: %s\n' "$(rustc --version)"
  printf 'cargo: %s\n' "$(cargo --version)"
  printf 'host: %s\n' "$(uname -a)"
  printf 'CARGO_INCREMENTAL: %s\n' "${CARGO_INCREMENTAL:-<unset>}"
  printf 'RUSTC_WRAPPER: %s\n' "${RUSTC_WRAPPER:-<unset>}"
} > "${output%.json}.env.txt"

echo "Wrote $output"
