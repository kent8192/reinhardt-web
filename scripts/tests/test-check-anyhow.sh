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

assert_manifest_valid() {
	local name="$1"
	if ! cargo metadata --no-deps --format-version 1 --manifest-path "$FIXTURE/Cargo.toml" >"$FIXTURE/metadata.log" 2>&1; then
		fail "$name (cargo metadata rejected fixture syntax)"
		cat "$FIXTURE/metadata.log" >&2
	fi
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

expect_dependency_rejected() {
	local name="$1"
	local context="$2"
	local dependency="$3"
	expect_rejected "$name" "Cargo.toml:1:remove direct anyhow dependency from $context: $dependency"
}

reset_fixture
expect_clean "clean manifest, source, and README"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
anyhow = "1"
EOF
expect_dependency_rejected "dependency key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
"anyhow" = "1"
EOF
expect_dependency_rejected "quoted dependency key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
"any\u0068ow" = "1"
EOF
assert_manifest_valid "unicode-escaped dependency key"
expect_dependency_rejected "unicode-escaped dependency key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
"any\U00000068ow" = "1"
EOF
assert_manifest_valid "long-unicode-escaped dependency key"
expect_dependency_rejected "long-unicode-escaped dependency key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[dependencies."any\u0068ow"]
version = "1"
EOF
assert_manifest_valid "unicode-escaped dependency subtable"
expect_dependency_rejected "unicode-escaped dependency subtable" "dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[package]
name = "scanner-fixture"
version = "0.1.0"

["depend\u0065ncies"]
anyhow = "1"
EOF
assert_manifest_valid "unicode-escaped dependency table key"
expect_dependency_rejected "unicode-escaped dependency table key" "dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[package]
name = "scanner-fixture"
version = "0.1.0"

['dependencies']
anyhow = "1"
EOF
assert_manifest_valid "literal dependency table key"
expect_dependency_rejected "literal dependency table key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[dependencies.'anyhow']
version = "1"
EOF
assert_manifest_valid "literal dependency subtable key"
expect_dependency_rejected "literal dependency subtable key" "dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[package]
name = "scanner-fixture"
version = "0.1.0"

["dependencies.anyhow"]
version = "1"
EOF
assert_manifest_valid "quoted table component containing dots"
expect_clean "quoted table component containing dots"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
"dependencies.anyhow" = "metadata"

[package]
name = "scanner-fixture"
version = "0.1.0"
EOF
assert_manifest_valid "quoted root key containing dots"
expect_clean "quoted root key containing dots"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[dependencies.anyhow]
version = "1"
EOF
expect_dependency_rejected "dependency subtable" "dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
dependencies.thiserror = "2"
dependencies.anyhow.version = "1"

[package]
name = "scanner-fixture"
version = "0.1.0"
EOF
expect_dependency_rejected "dotted dependency key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
anyhow.version = "1"
EOF
expect_dependency_rejected "dependency-table local dotted key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
"anyhow".version = "1"
EOF
expect_dependency_rejected "quoted dependency-table local dotted key" "dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[dev-dependencies]
anyhow.version = "1"
EOF
expect_dependency_rejected "dev-dependency-table local dotted key" "dev-dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[build-dependencies]
anyhow.version = "1"
EOF
expect_dependency_rejected "build-dependency-table local dotted key" "build-dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[workspace.dependencies]
anyhow.version = "1"
EOF
expect_dependency_rejected "workspace-dependency-table local dotted key" "workspace.dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

[workspace.dependencies]
"any\u0068ow" = "1"
EOF
assert_manifest_valid "unicode-escaped workspace dependency key"
expect_dependency_rejected "unicode-escaped workspace dependency key" "workspace.dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

[workspace.dependencies]
errors = { package = """anyhow""", version = "1" }
EOF
assert_manifest_valid "triple-basic workspace package alias"
expect_dependency_rejected "triple-basic workspace package alias" "workspace.dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

[workspace.dependencies]
errors = { "pack\u0061ge" = "any\u0068ow", version = "1" }
EOF
assert_manifest_valid "unicode-escaped workspace package alias"
expect_dependency_rejected "unicode-escaped workspace package alias" "workspace.dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

[workspace.dependencies.errors]
version = "1"
package = """
anyhow"""
EOF
assert_manifest_valid "spanning triple-basic workspace package alias"
expect_dependency_rejected "spanning triple-basic workspace package alias" "workspace.dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

[workspace.dependencies]
errors = {
  version = "1",
  package = "anyhow",
}
EOF
assert_manifest_valid "physical multiline workspace inline alias"
expect_dependency_rejected "physical multiline workspace inline alias" "workspace.dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
workspace.members = []
workspace.dependencies.errors = { package = "anyhow", version = "1" }
EOF
assert_manifest_valid "root dotted workspace inline alias"
expect_dependency_rejected "root dotted workspace inline alias" "workspace.dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

['workspace.dependencies.anyhow']
version = "1"
EOF
assert_manifest_valid "quoted workspace table component containing dots"
expect_clean "quoted workspace table component containing dots"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
"workspace.dependencies.anyhow" = "metadata"

[workspace]
members = []
EOF
assert_manifest_valid "quoted workspace root key containing dots"
expect_clean "quoted workspace root key containing dots"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[target.'cfg(unix)'.dependencies]
anyhow.version = "1"
EOF
expect_dependency_rejected "target-dependency-table local dotted key" "target.cfg(unix).dependencies" "anyhow"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[package.metadata]
anyhow.version = "application metadata"
EOF
expect_clean "non-dependency table local dotted key"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[package.metadata]
anyhow = "application metadata"
EOF
expect_clean "metadata bare key"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[package.metadata.tool]
package = "anyhow"
EOF
expect_clean "metadata package value"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[package.metadata.tool]
example = "dep:anyhow"
EOF
expect_clean "metadata dependency-token example"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { package = "anyhow", version = "1" }
EOF
expect_dependency_rejected "package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { package = "any\u0068ow", version = "1" }
EOF
assert_manifest_valid "unicode-escaped package alias"
expect_dependency_rejected "unicode-escaped package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { "pack\u0061ge" = "any\u0068ow", version = "1" }
EOF
assert_manifest_valid "unicode-escaped package alias key and value"
expect_dependency_rejected "unicode-escaped package alias key and value" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { package = """anyhow""", version = "1" }
EOF
assert_manifest_valid "triple-basic package alias"
expect_dependency_rejected "triple-basic package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { package = '''anyhow''', version = "1" }
EOF
assert_manifest_valid "triple-literal package alias"
expect_dependency_rejected "triple-literal package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { git = "https://example.com/repository#main", package = "anyhow" }
EOF
expect_dependency_rejected "package alias after double-quoted URL fragment" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { git = 'https://example.com/repository#main', package = "anyhow" }
EOF
expect_dependency_rejected "package alias after single-quoted URL fragment" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = "1" # package = "anyhow"
EOF
expect_clean "package alias text in a real comment"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[package.metadata]
example = "escaped quote: \" and fragment # package = \"anyhow\""
EOF
expect_clean "metadata string with escaped quotes and fragment"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  git = "https://example.com/repository#main",
  package = "anyhow",
}
EOF
expect_dependency_rejected "multiline package alias in dependency table" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
  package = """anyhow""",
}
EOF
assert_manifest_valid "multiline dependency with triple-basic package alias"
expect_dependency_rejected "multiline dependency with triple-basic package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
  package = """
anyhow""",
}
EOF
assert_manifest_valid "physical-line-spanning triple-basic package alias"
expect_dependency_rejected "physical-line-spanning triple-basic package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
  package = '''
anyhow''',
}
EOF
assert_manifest_valid "physical-line-spanning triple-literal package alias"
expect_dependency_rejected "physical-line-spanning triple-literal package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
  package = """any\
    how""",
}
EOF
assert_manifest_valid "continued triple-basic package alias"
expect_dependency_rejected "continued triple-basic package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
EOF
printf '%s\n' '  package = """any\   ' >> "$FIXTURE/Cargo.toml"
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
    how""",
}
EOF
assert_manifest_valid "continued triple-basic package alias with trailing spaces"
expect_dependency_rejected "continued triple-basic package alias with trailing spaces" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
  note = """text"""",
  package = "anyhow",
}
EOF
assert_manifest_valid "four-quote triple-basic ending before package alias"
expect_dependency_rejected "four-quote triple-basic ending before package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  version = "1",
  note = '''text'''',
  package = "anyhow",
}
EOF
assert_manifest_valid "four-quote triple-literal ending before package alias"
expect_dependency_rejected "four-quote triple-literal ending before package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = { version = "1",
  package = "anyhow",
}
EOF
expect_dependency_rejected "multiline package alias after opening-line field" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
dependencies.errors = {
  version = "1",
  package = "anyhow",
}

[package]
name = "scanner-fixture"
version = "0.1.0"
EOF
expect_dependency_rejected "multiline root dotted package alias" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
members = []

[workspace.dependencies.errors]
version = "1"
package = "anyhow"
EOF
assert_manifest_valid "multiline workspace dotted package alias"
expect_dependency_rejected "multiline workspace dotted package alias" "workspace.dependencies" 'errors (package = "anyhow")'

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
target.'cfg(unix)'.dependencies.errors = {
  version = "1",
  package = "anyhow",
}

[package]
name = "scanner-fixture"
version = "0.1.0"
EOF
expect_dependency_rejected "multiline target dotted package alias" "target.cfg(unix).dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[package.metadata]
errors = {
  git = "https://example.com/repository#main",
  package = "anyhow",
}
EOF
expect_clean "multiline metadata package value"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors = {
  git = "https://example.com/repository#main",
  # package = "anyhow",
  version = "1",
}
EOF
expect_clean "multiline dependency package text in a real comment"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = {
  version = "1",
  note = ',package="anyhow",',
}
EOF
expect_clean "multiline literal string with package-like syntax"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = {
  version = "1",
  note = ",package=\"anyhow\",",
}
EOF
expect_clean "multiline basic string with escaped package-like syntax"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = { version = "1", note = ',package="anyhow",' }
EOF
expect_clean "single-line literal string with package-like syntax"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = { version = "1", note = "any\u0068ow" }
EOF
expect_clean "unicode-escaped dependency name in unrelated string"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = { version = "1", note = """anyhow""" }
EOF
expect_clean "triple-basic dependency name in unrelated string"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = {
  version = "1",
  note = """
package = "anyhow"
{ package = "anyhow" } # string content
[dependencies."anyhow"]
""",
}
EOF
assert_manifest_valid "physical-line-spanning unrelated triple-basic string"
expect_clean "physical-line-spanning unrelated triple-basic string"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
serde = {
  version = "1",
  note = '''
package = "anyhow"
{ package = "anyhow" } # string content
''',
}
EOF
assert_manifest_valid "physical-line-spanning unrelated triple-literal string"
expect_clean "physical-line-spanning unrelated triple-literal string"

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'

[dependencies.errors]
package = "anyhow"
version = "1"
EOF
expect_dependency_rejected "package alias dependency subtable" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
errors.package = "anyhow"
errors.version = "1"
EOF
assert_manifest_valid "package alias dotted dependency entry"
expect_dependency_rejected "package alias dotted dependency entry" "dependencies" 'errors (package = "anyhow")'

reset_fixture
cat >> "$FIXTURE/Cargo.toml" <<'EOF'
anyhow = { version = "1", optional = true }

[features]
dynamic = ["dep:anyhow"]
EOF
assert_manifest_valid "feature token"
expect_dependency_rejected "feature token" "dependencies" "anyhow"

reset_fixture
cat > "$FIXTURE/Cargo.toml" <<'EOF'
dependencies.anyhow = { version = "1", optional = true }
features.dynamic = ["dep:anyhow"]

[package]
name = "scanner-fixture"
version = "0.1.0"
EOF
assert_manifest_valid "dotted feature token"
expect_dependency_rejected "dotted feature token" "dependencies" "anyhow"

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
	"$FIXTURE/nested/.git" \
	"$FIXTURE/docs/superpowers/specs"
cat > "$FIXTURE/Cargo.lock" <<'EOF'
name = "anyhow"
EOF
cat > "$FIXTURE/CHANGELOG.md" <<'EOF'
- Removed `anyhow::Error` from owned code.
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
cat > "$FIXTURE/nested/vendor/Cargo.toml" <<'EOF'
[dependencies]
anyhow = "1"
EOF
cat > "$FIXTURE/nested/target/debug/generated.rs" <<'EOF'
let _ = anyhow !("generated");
EOF
cat > "$FIXTURE/nested/target/generated.rs" <<'EOF'
use anyhow::Result;
EOF
cat > "$FIXTURE/nested/.git/Cargo.toml" <<'EOF'
[dependencies]
anyhow = "1"
EOF
cat > "$FIXTURE/docs/superpowers/specs/error-design.md" <<'EOF'
```rust
use anyhow::Result;
```
EOF
cat > "$FIXTURE/docs/superpowers/error.md" <<'EOF'
```rust
use anyhow::Result;
```
EOF
expect_clean "lockfile, changelog, nested generated trees, and design artifacts are ignored"

exit "$FAIL"
