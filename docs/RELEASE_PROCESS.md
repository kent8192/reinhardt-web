# Release Process

## Purpose

This document provides comprehensive, step-by-step procedures for releasing Reinhardt crates to crates.io. It covers version selection, CHANGELOG management, verification, publication, and troubleshooting.

---

## Table of Contents

- [Overview](#overview)
- [Version Selection Guidelines](#version-selection-guidelines)
- [Pre-Release Checklist](#pre-release-checklist)
- [Release Workflow](#release-workflow)
- [CHANGELOG Management](#changelog-management)
- [Multi-Crate Releases](#multi-crate-releases)
- [Rollback Procedures](#rollback-procedures)
- [Troubleshooting](#troubleshooting)

---

## Overview

### Reinhardt's Release Strategy

Reinhardt follows a **per-crate versioning and tagging strategy**, inspired by large-scale Rust projects like Tokio:

- **Independent Versioning**: Each crate maintains its own version number
- **Selective Releases**: Only release crates that have changed
- **Clear Traceability**: Git tags enable precise version tracking
- **Semantic Versioning**: Strict adherence to SemVer 2.0.0

**Benefits:**
- Avoid unnecessary dependency updates
- Clear change tracking for each crate
- Efficient CI/CD workflows
- Better user experience (minimal breaking changes)

### Key Principles

1. **Explicit Authorization**: Every release step requires user approval
2. **Dry-Run First**: Always verify with `--dry-run` before actual publication
3. **Commit Before Tag**: Version bumps must be committed before creating tags
4. **Comprehensive Testing**: All tests must pass before release

---

## Version Selection Guidelines

### Semantic Versioning 2.0.0

Reinhardt strictly follows [Semantic Versioning 2.0.0](https://semver.org/):

**Format:** `MAJOR.MINOR.PATCH`

### Version Type Decision Matrix

#### MAJOR Version (X.0.0) - Breaking Changes

**When to use:**
- Breaking API changes (function signature changes, removed methods)
- Incompatible behavior changes
- Removed or renamed public items
- Changed trait bounds or generic constraints

**Examples:**
```rust
// MAJOR version bump required

// Before (v0.1.0)
pub fn connect(url: &str) -> Connection { }

// After (v0.2.0)
pub fn connect(url: &str) -> Result<Connection> { } // ❌ Breaking: return type changed
```

```rust
// MAJOR version bump required

// Before (v0.1.0)
pub struct QueryBuilder { }
impl QueryBuilder {
    pub fn build(self) -> Query { }
}

// After (v0.2.0)
impl QueryBuilder {
    pub fn build(self) -> Result<Query> { } // ❌ Breaking: method signature changed
}
```

#### MINOR Version (0.X.0) - New Features

**When to use:**
- New public functions, methods, or structs
- New optional features (feature flags)
- Backward-compatible enhancements
- Deprecations (with `#[deprecated]` attribute)

**Examples:**
```rust
// MINOR version bump (0.1.0 -> 0.2.0)

// Existing code (v0.1.0)
pub struct Pool { }

// Added in v0.2.0
impl Pool {
    pub fn with_timeout(self, duration: Duration) -> Self { } // ✅ New method, backward compatible
}
```

```rust
// MINOR version bump (0.1.0 -> 0.2.0)

// Added new public function
pub fn validate_email(email: &str) -> Result<()> { } // ✅ New functionality
```

#### PATCH Version (0.0.X) - Bug Fixes

**When to use:**
- Bug fixes (no API changes)
- Performance improvements (no API changes)
- Documentation corrections
- Internal refactoring (no public API impact)

**Examples:**
```rust
// PATCH version bump (0.1.0 -> 0.1.1)

// Before (v0.1.0)
pub fn calculate_hash(data: &[u8]) -> u64 {
    // Bug: incorrect hash calculation
    data.iter().fold(0, |acc, &b| acc + b as u64)
}

// After (v0.1.1)
pub fn calculate_hash(data: &[u8]) -> u64 {
    // Fix: correct hash calculation
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}
```

### Pre-1.0.0 Special Rules

For versions `0.x.x` (pre-1.0.0), per SemVer specification:

- **MINOR version changes (0.X.0) MAY include breaking changes**
- **PATCH version changes (0.0.X) MUST be backward compatible**

**Example:**
```
0.1.0 -> 0.2.0: Breaking changes allowed (API redesign)
0.2.0 -> 0.2.1: Only bug fixes (must be compatible with 0.2.0)
```

**After 1.0.0 Release:**
- Breaking changes REQUIRE MAJOR version bump (1.0.0 -> 2.0.0)
- Strict backward compatibility enforced

---

## Pre-Release Checklist

Before starting the release process, verify:

### Code Quality

- [ ] All tests pass: `cargo test --workspace --all --all-features`
- [ ] Code builds without warnings: `cargo build --workspace --all --all-features`
- [ ] Linting passes: `trunk lint`
- [ ] Code formatting applied: `trunk fmt`
- [ ] Documentation builds: `cargo doc --no-deps --all-features`

### Documentation

- [ ] README.md updated (if public API changed)
- [ ] Crate-level documentation updated (`src/lib.rs`)
- [ ] Code examples tested: `cargo test --doc`
- [ ] CHANGELOG.md prepared with release notes
- [ ] Migration guide written (for breaking changes)

### Dependencies

- [ ] Dependency versions are up-to-date
- [ ] No dev-dependencies on other Reinhardt crates (for functional crates)
- [ ] Cargo.toml metadata complete (description, license, repository)

### Metadata Verification

Run the following to verify crate metadata:

```bash
cargo publish --dry-run -p <crate-name>
```

Common metadata issues:
- Missing `description` field
- Missing `license` field
- Missing `repository` field

---

## Release Workflow

### Step-by-Step Release Process

#### Step 1: Select Version Number

Based on the [Version Selection Guidelines](#version-selection-guidelines):

```bash
# Review changes since last release
git log [crate-name]-v[last-version]..HEAD -- crates/[crate-name]/

# Determine version type
# MAJOR: Breaking changes
# MINOR: New features
# PATCH: Bug fixes only
```

**Example:**
```bash
git log reinhardt-orm-v0.1.0..HEAD -- crates/reinhardt-orm/
# Review commits to determine if changes are breaking, features, or fixes
```

#### Step 2: Update Cargo.toml

Edit `crates/[crate-name]/Cargo.toml`:

```toml
[package]
name = "reinhardt-orm"
version = "0.2.0"  # Update this line
description = "..."
# ... other fields
```

#### Step 3: Update CHANGELOG.md

Edit or create `crates/[crate-name]/CHANGELOG.md`:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-01-15

### Breaking Changes
- `QueryBuilder::build()` now returns `Result<Query>` instead of `Query`
- Removed deprecated method `Model::save_sync()`

### Added
- Support for async connection pooling
- `QueryBuilder::with_timeout()` method
- `Model::bulk_insert()` for batch operations

### Fixed
- Race condition in transaction rollback
- UTC timezone handling in timestamp fields

## [0.1.0] - 2024-12-01

### Added
- Initial release with basic ORM functionality
```

#### Step 4: Run Verification Commands

```bash
# Check that everything compiles
cargo check --workspace --all --all-features

# Run all tests
cargo test --workspace --all --all-features

# Run doc tests
cargo test --doc -p <crate-name>

# Build documentation
cargo doc --no-deps --all-features -p <crate-name>

# Lint and format
trunk fmt
trunk lint
```

#### Step 5: Commit Version Changes

Create a version bump commit following @CLAUDE.commit.md CE-5:

```bash
git add crates/[crate-name]/Cargo.toml
git add crates/[crate-name]/CHANGELOG.md

git commit -m "$(cat <<'EOF'
chore(release): Bump [crate-name] to v[version]

Prepare [crate-name] for publication to crates.io.

Version Changes:
- crates/[crate-name]/Cargo.toml: version [old] -> [new]
- crates/[crate-name]/CHANGELOG.md: Add release notes for v[new]

[Breaking Changes/New Features/Bug Fixes sections as applicable]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

**Example:**
```bash
git commit -m "$(cat <<'EOF'
chore(release): Bump reinhardt-orm to v0.2.0

Prepare reinhardt-orm for publication to crates.io.

Version Changes:
- crates/reinhardt-orm/Cargo.toml: version 0.1.0 -> 0.2.0
- crates/reinhardt-orm/CHANGELOG.md: Add release notes for v0.2.0

Breaking Changes:
- QueryBuilder::build() now returns Result<Query> instead of Query
- Removed deprecated method Model::save_sync()

New Features:
- Add support for async connection pooling
- Implement QueryBuilder::with_timeout() method
- Add Model::bulk_insert() for batch operations

Bug Fixes:
- Fix race condition in transaction rollback
- Correct UTC timezone handling in timestamp fields

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

#### Step 6: Dry-Run Publication

**NEVER skip this step!**

```bash
cargo publish --dry-run -p <crate-name>
```

Review the dry-run output carefully:

- [ ] No warnings or errors
- [ ] File list is correct (no extra test files, no missing essential files)
- [ ] Metadata is complete
- [ ] Size is reasonable (check `target/package/<crate-name>-<version>.crate` size)

**Common Issues:**
- Missing `Cargo.toml` fields (description, license, repository)
- Uncommitted files in working directory
- Path dependencies not published to crates.io

#### Step 7: Wait for User Authorization

**CRITICAL: Do not proceed without explicit user approval**

Present the dry-run results to the user and wait for confirmation:

```
Dry-run completed successfully for [crate-name] v[version].

Package size: X.XX MB
Files included: XX files

Ready to publish. Please confirm:
- Have you reviewed the dry-run output?
- Are all changes documented in CHANGELOG.md?
- Is this the correct version number?

Type "yes" to proceed with publication.
```

#### Step 8: Publish to crates.io

After user confirms:

```bash
cargo publish -p <crate-name>
```

**Expected output:**
```
    Updating crates.io index
   Packaging reinhardt-orm v0.2.0 (/path/to/crates/reinhardt-orm)
   Verifying reinhardt-orm v0.2.0 (/path/to/crates/reinhardt-orm)
   Compiling reinhardt-orm v0.2.0 (/path/to/crates/reinhardt-orm)
    Finished dev [unoptimized + debuginfo] target(s) in X.XXs
   Uploading reinhardt-orm v0.2.0 (/path/to/crates/reinhardt-orm)
```

**Note:** Publication is immediate and **cannot be undone**. Only version yanking is possible after publication.

#### Step 9: Create Git Tag

After successful publication:

```bash
git tag [crate-name]-v[version] -m "Release [crate-name] v[version]"
```

**Example:**
```bash
git tag reinhardt-orm-v0.2.0 -m "Release reinhardt-orm v0.2.0"
```

Verify tag creation:
```bash
git tag -l "[crate-name]-v*"
```

#### Step 10: Push Commits and Tags

```bash
git push origin main  # or your branch name
git push origin --tags
```

Verify on GitHub:
- Commit appears in history
- Tag appears in "Releases" or "Tags" section

---

## CHANGELOG Management

### Format (Keep a Changelog)

Reinhardt uses [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Work in progress features (not yet released)

## [0.2.0] - 2025-01-15

### Breaking Changes
- List breaking changes first for visibility

### Added
- New features

### Changed
- Changes to existing functionality

### Deprecated
- Features marked for removal (use `#[deprecated]` in code)

### Removed
- Features removed in this release

### Fixed
- Bug fixes

### Security
- Security vulnerability fixes

## [0.1.0] - 2024-12-01

### Added
- Initial release
```

### Section Guidelines

#### Breaking Changes
- **Always list first** for maximum visibility
- Explain what changed and how to migrate
- Include code examples if helpful

**Example:**
```markdown
### Breaking Changes
- `QueryBuilder::build()` now returns `Result<Query>` instead of `Query`
  - **Migration:** Wrap calls in `?` operator: `let query = builder.build()?;`
- Removed deprecated method `Model::save_sync()`
  - **Migration:** Use `Model::save().await` instead
```

#### Added
- New public APIs
- New features
- New configuration options

#### Changed
- Changes to existing behavior (non-breaking)
- Performance improvements
- Internal refactoring (user-visible impact)

#### Deprecated
- Features marked with `#[deprecated]` attribute
- Include removal timeline

**Example:**
```markdown
### Deprecated
- `Pool::new()` is deprecated, use `Pool::builder()` instead (will be removed in v0.3.0)
```

#### Removed
- Features removed in this release
- Must correspond to MAJOR version bump (unless pre-1.0.0)

#### Fixed
- Bug fixes
- Include issue numbers if applicable

**Example:**
```markdown
### Fixed
- Fix race condition in transaction rollback (#123)
- Correct UTC timezone handling in timestamp fields (#145)
```

#### Security
- Security vulnerability fixes
- **Always highlight prominently**

---

## Multi-Crate Releases

### When to Release Multiple Crates

Release multiple crates when:
- One crate depends on updated version of another
- Coordinated feature release across crates
- Breaking changes cascade through dependencies

### Dependency Order

**Release in reverse dependency order:**

1. **Leaf crates** (no internal dependencies)
   - Example: `reinhardt-types`, `reinhardt-macros`

2. **Mid-level crates** (depend on leaf crates)
   - Example: `reinhardt-orm` (depends on `reinhardt-types`)

3. **Top-level crates** (depend on mid-level)
   - Example: `reinhardt` (facade, depends on many crates)

**Visualization:**
```
reinhardt (facade) -> reinhardt-orm -> reinhardt-types
                   -> reinhardt-http -> reinhardt-types

Release order: reinhardt-types -> reinhardt-orm, reinhardt-http -> reinhardt
```

### Multi-Crate Release Workflow

#### Step 1: Identify Release Set

```bash
# List crates with changes since last release
git log --name-only --since="2024-12-01" -- crates/ | grep Cargo.toml
```

#### Step 2: Determine Versions

For each crate:
- Review changes
- Select appropriate version (MAJOR/MINOR/PATCH)
- Document in CHANGELOG.md

#### Step 3: Update Dependency Versions

If `crate-b` depends on `crate-a`:

**After releasing `crate-a` v0.2.0:**

Update `crates/crate-b/Cargo.toml`:
```toml
[dependencies]
crate-a = { workspace = true }  # If using workspace dependencies
# OR
crate-a = "0.2.0"  # Explicit version
```

#### Step 4: Release in Order

Follow the standard release workflow for each crate:

1. Release `crate-a` v0.2.0
2. Update `crate-b`'s dependency on `crate-a`
3. Release `crate-b` v0.3.0
4. Continue up the dependency chain

#### Step 5: Verify Dependency Chain

After all releases:

```bash
cargo tree -p <top-level-crate> | grep reinhardt
```

Verify all internal dependencies use correct versions.

---

## Rollback Procedures

### Version Yanking (Preferred)

If a released version has critical issues:

```bash
cargo yank -p <crate-name> --vers <version>
```

**What yank does:**
- Prevents new projects from using the version
- Existing projects with `Cargo.lock` can still use it
- Version remains in crates.io (cannot be deleted)

**When to yank:**
- Critical bugs
- Security vulnerabilities
- Incorrect metadata

**After yanking:**
1. Fix the issue
2. Increment PATCH version
3. Release fixed version

**Example:**
```bash
# Yank broken version
cargo yank -p reinhardt-orm --vers 0.2.0

# Fix the issue in code
# ...

# Release fixed version
cargo publish -p reinhardt-orm  # Now v0.2.1
```

### Un-yanking (If Safe)

If yank was a mistake:

```bash
cargo yank --undo -p <crate-name> --vers <version>
```

### Git Tag Rollback

If publication failed but tag was created:

```bash
# Delete local tag
git tag -d [crate-name]-v[version]

# Delete remote tag (if pushed)
git push origin :refs/tags/[crate-name]-v[version]
```

### Commit Rollback

If version bump was committed but publication failed:

```bash
# Revert the commit
git revert HEAD

# OR reset if not pushed yet
git reset --hard HEAD~1
```

---

## Troubleshooting

### Common Issues and Solutions

#### Issue: `cargo publish` fails with "no such file or directory"

**Cause:** Uncommitted changes or files in `.gitignore`

**Solution:**
```bash
git status  # Check for uncommitted changes
git add <missing-files>
git commit -m "Add missing files"
```

#### Issue: "package.description missing in manifest"

**Cause:** Missing `description` field in `Cargo.toml`

**Solution:**
```toml
[package]
name = "reinhardt-orm"
version = "0.2.0"
description = "ORM layer for Reinhardt framework"  # Add this
```

#### Issue: "version already published"

**Cause:** Attempting to publish an already-published version

**Solution:**
- Increment version number
- Update CHANGELOG.md
- Commit and retry

#### Issue: "path dependency outside of workspace"

**Cause:** Path dependencies to unpublished crates

**Solution:**
- Publish dependency crates first
- Update `Cargo.toml` to use published versions:
  ```toml
  [dependencies]
  reinhardt-types = { workspace = true }  # Use workspace version
  # OR
  reinhardt-types = "0.1.0"  # Explicit version from crates.io
  ```

#### Issue: Tag already exists

**Cause:** Git tag already created (possibly from failed previous attempt)

**Solution:**
```bash
# Delete local tag
git tag -d [crate-name]-v[version]

# Delete remote tag (if pushed)
git push origin :refs/tags/[crate-name]-v[version]

# Recreate tag after fixing
git tag [crate-name]-v[version] -m "Release [crate-name] v[version]"
```

#### Issue: CI/CD failures after release

**Cause:** Tests failing with new version

**Solution:**
1. Yank the problematic version: `cargo yank -p <crate-name> --vers <version>`
2. Fix the issue in code
3. Increment PATCH version
4. Re-release with fix

### Verification Checklist

If unsure about a release, verify:

- [ ] Version number follows SemVer
- [ ] CHANGELOG.md is up-to-date
- [ ] All tests pass locally and in CI
- [ ] Documentation builds without warnings
- [ ] Dry-run succeeds
- [ ] No uncommitted changes
- [ ] Dependencies are published to crates.io

---

## Related Documentation

- **Main Standards**: @CLAUDE.md
- **Commit Guidelines**: @CLAUDE.commit.md
- **Version Policy**: See "Release & Publishing Policy" in CLAUDE.md

---

## Quick Reference

### Complete Release Checklist

```
1. ✅ Select version (MAJOR/MINOR/PATCH)
2. ✅ Update Cargo.toml
3. ✅ Update CHANGELOG.md
4. ✅ Run verification commands
5. ✅ Commit version changes
6. ✅ Run cargo publish --dry-run
7. ✅ Wait for user authorization
8. ✅ Publish: cargo publish -p <crate-name>
9. ✅ Create Git tag
10. ✅ Push commits and tags
```

### Key Commands

```bash
# Verify
cargo check --workspace --all --all-features
cargo test --workspace --all --all-features
cargo test --doc -p <crate-name>

# Publish
cargo publish --dry-run -p <crate-name>
cargo publish -p <crate-name>

# Tagging
git tag [crate-name]-v[version] -m "Release [crate-name] v[version]"
git push origin main
git push origin --tags

# Yank (if needed)
cargo yank -p <crate-name> --vers <version>
```
