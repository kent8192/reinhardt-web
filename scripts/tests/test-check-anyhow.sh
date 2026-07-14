#!/usr/bin/env bash
# Verify the direct dynamic-error dependency scanner against isolated fixtures.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCANNER="$REPO_ROOT/scripts/check-anyhow.sh"
FIXTURE=$(mktemp -d)
trap 'rm -rf "$FIXTURE"' EXIT

FAIL=0

pass() {
	echo "  PASS: $1"
}

fail() {
	echo "  FAIL: $1" >&2
	FAIL=1
}

reset_fixture() {
	rm -rf "$FIXTURE"
	mkdir -p "$FIXTURE/src"
	cat > "$FIXTURE/Cargo.toml" <<'EOF'
[package]
name = "scanner-fixture"
version = "0.1.0"

[dependencies]
thiserror = "2"
EOF
	cat > "$FIXTURE/src/lib.rs" <<'EOF'
pub fn message() -> &'static str {
	"clean"
}
EOF
	cat > "$FIXTURE/README.md" <<'EOF'
# Scanner fixture

This repository uses named error types.
EOF
}

run_scanner() {
	set +e
	bash "$SCANNER" "$FIXTURE" >"$FIXTURE/output.log" 2>&1
	SCANNER_RC=$?
	set -e
}

expect_clean() {
	local name="$1"
	run_scanner
	if [ "$SCANNER_RC" -eq 0 ]; then
		pass "$name"
	else
		fail "$name (expected exit 0, got $SCANNER_RC)"
		cat "$FIXTURE/output.log" >&2
	fi
}

expect_rejected() {
	local name="$1"
	local expected_match="$2"
	run_scanner

	if [ "$SCANNER_RC" -eq 0 ]; then
		fail "$name (expected nonzero exit)"
	elif ! grep -Fqx "$expected_match" "$FIXTURE/output.log"; then
		fail "$name (missing relative line match: $expected_match)"
		cat "$FIXTURE/output.log" >&2
	elif [ "$(grep -Fc 'Remove direct anyhow dependencies and replace owned usage with repository error types.' "$FIXTURE/output.log")" -ne 1 ]; then
		fail "$name (expected one actionable failure summary)"
		cat "$FIXTURE/output.log" >&2
	else
		pass "$name"
	fi
}

reset_fixture
expect_clean "clean manifest, source, and README"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
anyhow = "1"
EOF
expect_rejected "dependency key" 'Cargo.toml:7:anyhow = "1"'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
"anyhow" = "1"
EOF
expect_rejected "quoted dependency key" 'Cargo.toml:7:"anyhow" = "1"'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[dependencies.anyhow]
version = "1"
EOF
expect_rejected "dependency subtable" 'Cargo.toml:8:[dependencies.anyhow]'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[package]
name = "scanner-fixture"
version = "0.1.0"

dependencies.thiserror = "2"
dependencies.anyhow.version = "1"
EOF
expect_rejected "dotted dependency key" 'Cargo.toml:6:dependencies.anyhow.version = "1"'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { package = "anyhow", version = "1" }
EOF
expect_rejected "package alias" 'Cargo.toml:7:errors = { package = "anyhow", version = "1" }'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[features]
dynamic = ["dep:anyhow"]
EOF
expect_rejected "feature token" 'Cargo.toml:9:dynamic = ["dep:anyhow"]'

reset_fixture
cat > "$FIXTURE/src/lib.rs" <<'EOF'
use anyhow::Result;
EOF
expect_rejected "Rust Result import" 'src/lib.rs:1:use anyhow::Result;'

reset_fixture
cat > "$FIXTURE/src/lib.rs" <<'EOF'
pub type Error = anyhow :: Error;
EOF
expect_rejected "Rust path with separator whitespace" 'src/lib.rs:1:pub type Error = anyhow :: Error;'

reset_fixture
cat > "$FIXTURE/src/lib.rs" <<'EOF'
pub fn failure() {
	let _ = anyhow!("failure");
}
EOF
expect_rejected "Rust macro call" 'src/lib.rs:2:	let _ = anyhow!("failure");'

reset_fixture
cat > "$FIXTURE/README.md" <<'EOF'
# Example

```rust
fn run() -> Result<(), anyhow::Error> {
	Ok(())
}
```
EOF
expect_rejected "README code" 'README.md:4:fn run() -> Result<(), anyhow::Error> {'

reset_fixture
cat > "$FIXTURE/README.md" <<'EOF'
# Migration guidance

Applications should not use anyhow for framework boundaries.
EOF
expect_clean "ordinary Markdown prose may name the removed dependency"

reset_fixture
cat > "$FIXTURE/README.md" <<'EOF'
# Example

```rust
use anyhow;
```
EOF
expect_rejected "Markdown crate import" 'README.md:4:use anyhow;'

reset_fixture
cat > "$FIXTURE/README.md" <<'EOF'
# Example

```rust
type Error = anyhow :: Error;
```
EOF
expect_rejected "Markdown path with separator whitespace" 'README.md:4:type Error = anyhow :: Error;'

reset_fixture
cat > "$FIXTURE/README.md" <<'EOF'
# Example

```rust
let error = anyhow !("failure");
```
EOF
expect_rejected "Markdown macro with separator whitespace" 'README.md:4:let error = anyhow !("failure");'

reset_fixture
mkdir -p \
	"$FIXTURE/vendor/example/src" \
	"$FIXTURE/target/debug" \
	"$FIXTURE/nested/vendor/example/src" \
	"$FIXTURE/nested/target/debug" \
	"$FIXTURE/docs/superpowers/specs"
cat > "$FIXTURE/Cargo.lock" <<'EOF'
name = "anyhow"
EOF
cat > "$FIXTURE/CHANGELOG.md" <<'EOF'
- Removed anyhow from owned code.
EOF
cat > "$FIXTURE/vendor/example/Cargo.toml" <<'EOF'
[dependencies]
anyhow = "1"
EOF
cat > "$FIXTURE/vendor/example/src/lib.rs" <<'EOF'
use anyhow::Result;
EOF
cat > "$FIXTURE/target/debug/generated.rs" <<'EOF'
use anyhow::Result;
EOF
cat > "$FIXTURE/nested/vendor/example/Cargo.toml" <<'EOF'
[dependencies]
"anyhow" = "1"
EOF
cat > "$FIXTURE/nested/vendor/example/src/lib.rs" <<'EOF'
pub type Error = anyhow :: Error;
EOF
cat > "$FIXTURE/nested/target/debug/generated.rs" <<'EOF'
let _ = anyhow !("generated");
EOF
cat > "$FIXTURE/docs/superpowers/specs/error-design.md" <<'EOF'
```rust
use anyhow::Result;
```
EOF
expect_clean "lockfile, changelog, nested vendor, nested target, and design artifacts are ignored"

exit "$FAIL"
