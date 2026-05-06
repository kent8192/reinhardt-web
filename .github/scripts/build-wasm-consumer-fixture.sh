#!/usr/bin/env bash
# Generate a wasm SPA-shaped consumer using reinhardt-admin templates and
# exercise the four macro re-exports gated by Issue #4161.
#
# Usage:
#   build-wasm-consumer-fixture.sh                # workspace-path form (CI alpha)
#   build-wasm-consumer-fixture.sh --use-packaged # publish-form (CI beta)
#
# Tracks: kent8192/reinhardt-web#4161
set -euo pipefail

USE_PACKAGED=0
if [[ "${1:-}" == "--use-packaged" ]]; then
	USE_PACKAGED=1
fi

: "${GITHUB_WORKSPACE:?GITHUB_WORKSPACE must be set (path to reinhardt-web checkout)}"
WORK="${RUNNER_TEMP:-/tmp}/wasm-consumer-fixture"
PKG_STAGE="${PKG_STAGE:-/tmp/pkg-stage}"

rm -rf "$WORK"
mkdir -p "$WORK"

echo "::group::1) Scaffold project via reinhardt-admin startproject --with-pages"
cargo run --quiet \
	--manifest-path "$GITHUB_WORKSPACE/Cargo.toml" \
	-p reinhardt-admin-cli -- \
	startproject verifier --with-pages --directory "$WORK"
echo "::endgroup::"

cd "$WORK/verifier"

echo "::group::2) Scaffold demo app via reinhardt-admin startapp --with-pages"
cargo run --quiet \
	--manifest-path "$GITHUB_WORKSPACE/Cargo.toml" \
	-p reinhardt-admin-cli -- startapp demo --with-pages
echo "::endgroup::"

echo "::group::3) Rewrite Cargo.toml to point at PR HEAD (or packaged tarballs)"
if [[ $USE_PACKAGED -eq 1 ]]; then
	# Pass --reinhardt-path so the umbrella `reinhardt-web` (excluded from
	# `cargo package` in publish-check.yml) is patched to the local workspace.
	# Sub-crates remain patched to their freshly-built tarballs, exercising the
	# publish-form regression surface for #4161.
	python3 "$GITHUB_WORKSPACE/.github/scripts/patch-fixture-cargo-toml.py" \
		--manifest Cargo.toml \
		--use-packaged \
		--pkg-stage "$PKG_STAGE" \
		--reinhardt-path "$GITHUB_WORKSPACE"
else
	python3 "$GITHUB_WORKSPACE/.github/scripts/patch-fixture-cargo-toml.py" \
		--manifest Cargo.toml \
		--reinhardt-path "$GITHUB_WORKSPACE"
fi
echo "::endgroup::"

echo "::group::4) Apply augment patch (#[url_patterns mode=unified|ws])"
# `git apply` needs to be inside a git repo for context line resolution.
git init --quiet
git add -A
git commit --quiet -m "snapshot before augment" \
	--author "ci-fixture <ci@local>" \
	-c user.name="ci-fixture" -c user.email="ci@local"
git apply "$GITHUB_WORKSPACE/.github/fixtures/wasm-consumer-augment.patch"
echo "::endgroup::"

echo "::group::5) cargo check --target wasm32-unknown-unknown --lib (the gate)"
cargo check --target wasm32-unknown-unknown --lib
echo "::endgroup::"

echo "wasm-consumer-fixture: PASS"
