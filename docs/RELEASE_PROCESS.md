# Release Process

## Purpose

This document provides comprehensive, step-by-step procedures for releasing Reinhardt crates to crates.io. It covers version selection, CHANGELOG management, verification, publication, and troubleshooting.

---

## Table of Contents

- [Overview](#overview)
- [Version Selection Guidelines](#version-selection-guidelines)
- [Pre-Release Checklist](#pre-release-checklist)
- [Manual Publishing Process (Without CI/CD)](#manual-publishing-process-without-cicd)
- [Automated Publishing with CI/CD](#automated-publishing-with-cicd)
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
- [ ] Linting passes: `cargo make clippy-fix`
- [ ] Code formatting applied: `cargo make fmt-fix`
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

## Manual Publishing Process (Without CI/CD)

This section is preserved as a reference for cases where CI/CD automation is unavailable or when you need to perform completely manual publishing.

**Recommended**: Normally, use the procedures in the "Automated Publishing with CI/CD" section.

### Step-by-Step Release Process

#### Step 1: Select Version Number

Based on the [Version Selection Guidelines](#version-selection-guidelines):

```bash
# Review changes since last release
git log [crate-name]@v[last-version]..HEAD -- crates/[crate-name]/

# Determine version type
# MAJOR: Breaking changes
# MINOR: New features
# PATCH: Bug fixes only
```

**Example:**
```bash
git log reinhardt-orm@v0.1.0..HEAD -- crates/reinhardt-orm/
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
cargo make fmt-fix
cargo make clippy-fix
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
git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"
```

**Example:**
```bash
git tag reinhardt-orm@v0.2.0 -m "Release reinhardt-orm v0.2.0"
```

Verify tag creation:
```bash
git tag -l "[crate-name]@v*"
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

## Automated Publishing with CI/CD

The Reinhardt project automatically publishes crates to crates.io when a pull request with the `release` label is merged to the main branch. This integrated workflow combines change detection, publishing, tagging, and GitHub Release creation into a single automated process.

### Prerequisites

#### GitHub Repository Setup

Before using the automated publishing system, configure the following:

**1. GitHub Secrets**

Add `CARGO_REGISTRY_TOKEN` to repository secrets:
- Navigate to Settings → Secrets and variables → Actions
- Click "New repository secret"
- Name: `CARGO_REGISTRY_TOKEN`
- Value: Your crates.io API token (obtain from https://crates.io/settings/tokens)

**2. GitHub Labels**

Create a `release` label:
- Navigate to Issues → Labels → New label
- Name: `release`
- Description: `Trigger automatic crates.io publishing`
- Color: `#0e8a16` (green)

**3. Branch Protection Rules (Recommended)**

Configure main branch protection:
- Navigate to Settings → Branches → Branch protection rules → Add rule
- Branch name pattern: `main`
- Enable:
  - ✅ Require pull request reviews before merging
  - ✅ Require status checks to pass before merging
  - Select `Publish Dry-Run` (this check appears after first workflow run)
  - ✅ Require branches to be up to date before merging

### Publishing Workflow

The automated publishing process consists of two main stages:

1. **Pre-merge validation** (`publish-dry-run.yml`): Validates publishability before merge
2. **Post-merge publishing** (`publish-on-merge.yml`): Publishes to crates.io and creates releases

#### Step 1: Develop and Create PR

Create a feature branch and update version information:

```bash
# 1. Create feature branch
git checkout -b feature/update-reinhardt-orm

# 2. Update Cargo.toml version
vim crates/reinhardt-orm/Cargo.toml  # version = "0.2.0"

# 3. Update CHANGELOG.md
vim crates/reinhardt-orm/CHANGELOG.md  # Add release notes

# 4. Commit changes
git add crates/reinhardt-orm/Cargo.toml crates/reinhardt-orm/CHANGELOG.md
git commit -m "chore(release): Bump reinhardt-orm to v0.2.0

Prepare reinhardt-orm for publication to crates.io.

Version Changes:
- crates/reinhardt-orm/Cargo.toml: version 0.1.0 -> 0.2.0
- crates/reinhardt-orm/CHANGELOG.md: Add release notes for v0.2.0

New Features:
- Add support for async connection pooling
- Implement QueryBuilder::with_timeout() method

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

# 5. Push and create PR
git push origin feature/update-reinhardt-orm
```

#### Step 2: Add `release` Label to PR

**Critical**: The `release` label triggers both dry-run validation and automatic publishing.

1. Open the PR on GitHub
2. Click "Labels" in the right sidebar
3. Select `release` label
4. Verify the green `release` label appears

**What happens:**
- `publish-dry-run.yml` workflow automatically starts
- Detects changed crates using `cargo ws changed`
- Runs `cargo ws publish --dry-run` for each changed crate
- Reports results in PR checks

#### Step 3: Review Dry-Run Results

Check the GitHub Actions tab for dry-run results:

**Success indicators:**
- ✅ Green check mark on "Publish Dry-Run" workflow
- Summary shows: "📋 Dry-Run Publish Check" with validated crates list
- No errors or warnings in workflow logs

**Common issues:**
- ❌ Missing Cargo.toml fields (description, license, repository)
- ❌ Unpublished dependencies
- ❌ Version already published to crates.io

**Fix issues before merging:**
```bash
# Fix metadata issues
vim crates/reinhardt-orm/Cargo.toml  # Add missing fields

# Commit and push fixes
git add crates/reinhardt-orm/Cargo.toml
git commit -m "fix(release): Add missing Cargo.toml metadata"
git push origin feature/update-reinhardt-orm

# Dry-run automatically re-runs on new push
```

#### Step 4: Merge PR

Once dry-run passes and PR is approved:

1. Click "Merge pull request" on GitHub
2. Confirm merge

**Important**: Only merge after dry-run succeeds.

#### Step 5: Automatic Publishing Process

After merge, `publish-on-merge.yml` workflow automatically executes:

**Phase 1: Change Detection**
- Detects changed crates using `cargo ws changed --include-merged-tags`
- Extracts crate name and version from Cargo.toml
- Checks if version already published to crates.io
- Skips already-published crates

**Phase 2: Sequential Publishing**
- Publishes each changed crate to crates.io
- Runs dry-run before each publish as final verification
- Uses exponential backoff retry (3 attempts: 10s, 20s, 40s)
- Waits 30 seconds between crates (for crates.io index propagation)
- Creates local Git tags after successful publish

**Phase 3: GitHub Releases**
- Extracts release notes from each crate's CHANGELOG.md
- Creates GitHub Release for each published crate
- Links to crates.io and docs.rs

**Phase 4: Tag Push**
- Pushes all created tags to remote repository
- Tag format: `[crate-name]@v[version]`

**Example execution:**
```
[1/2] Publishing reinhardt-types v0.2.0...
  Running dry-run...
  ✅ Dry-run passed for reinhardt-types v0.2.0
  Publishing...
  ✅ Successfully published reinhardt-types v0.2.0
  🏷️ Created tag: reinhardt-types@v0.2.0
  ⏳ Waiting 30 seconds before next crate...

[2/2] Publishing reinhardt-orm v0.3.0...
  Running dry-run...
  ✅ Dry-run passed for reinhardt-orm v0.3.0
  Publishing...
  ✅ Successfully published reinhardt-orm v0.3.0
  🏷️ Created tag: reinhardt-orm@v0.3.0

Creating GitHub Releases...
✅ Created GitHub Release for reinhardt-types@v0.2.0
✅ Created GitHub Release for reinhardt-orm@v0.3.0

Pushing tags...
✅ Tag reinhardt-types@v0.2.0 pushed successfully
✅ Tag reinhardt-orm@v0.3.0 pushed successfully

🎉 All crates published successfully!
```

#### Step 6: Verify Publication

Check the following to confirm successful publication:

**crates.io:**
- Visit https://crates.io/crates/[crate-name]
- Verify new version appears in version dropdown
- Check publication timestamp

**GitHub Releases:**
- Navigate to repository's "Releases" page
- Verify new release with tag `[crate-name]@v[version]`
- Check release notes extracted from CHANGELOG.md

**docs.rs:**
- Visit https://docs.rs/[crate-name]/[version]
- Documentation builds automatically within ~5-10 minutes
- Check "Docs" badge shows "passing"

**GitHub Actions:**
- Workflow summary shows "🎉 Release Successful"
- Lists all published crates with links
- No errors in workflow logs

### Multi-Crate Release Process

The workflow automatically handles multiple crate releases in a single PR:

```bash
# Update multiple crates
vim crates/reinhardt-types/Cargo.toml     # version = "0.2.0"
vim crates/reinhardt-types/CHANGELOG.md

vim crates/reinhardt-orm/Cargo.toml       # version = "0.3.0"
vim crates/reinhardt-orm/CHANGELOG.md

# Commit all changes
git add crates/reinhardt-types crates/reinhardt-orm
git commit -m "chore(release): Bump reinhardt-types v0.2.0 and reinhardt-orm v0.3.0

..."

# Create PR + add `release` label + merge
```

**Automatic dependency ordering:**
- `cargo ws changed` detects all modified crates
- Publishes in dependency order (leaf crates first)
- 30-second intervals ensure crates.io index updates
- Retry logic handles transient failures

**Example with dependencies:**
```
reinhardt-types (no internal deps) → published first
    ↓
reinhardt-orm (depends on reinhardt-types) → published after 30s
```

### Troubleshooting

#### Issue: Dry-run Check Not Executing

**Cause**: PR does not have `release` label

**Solution**:
1. Add `release` label to PR
2. Workflow automatically triggers
3. Alternative: Manually trigger from GitHub Actions tab

#### Issue: Dry-run Failed - Missing Metadata

**Cause**: Cargo.toml missing required fields

**Error example:**
```
error: missing field `description` in Cargo.toml
```

**Solution**:
```toml
[package]
name = "reinhardt-orm"
version = "0.2.0"
description = "ORM layer for Reinhardt framework"  # Add this
license = "MIT OR Apache-2.0"  # Add this
repository = "https://github.com/kent8192/reinhardt-rs"  # Add this
```

Commit and push - dry-run re-runs automatically.

#### Issue: Publishing Failed - Version Already Exists

**Cause**: Version already published to crates.io

**Solution**:
```bash
# Check existing versions
curl -s "https://crates.io/api/v1/crates/reinhardt-orm" | jq '.versions[].num'

# Increment version in Cargo.toml
vim crates/reinhardt-orm/Cargo.toml  # version = "0.2.1"

# Update CHANGELOG.md
vim crates/reinhardt-orm/CHANGELOG.md

# Commit, push, wait for dry-run, merge
```

#### Issue: Publishing Failed - Dependency Not Available

**Cause**: Dependency crate not yet published or crates.io index not updated

**Solution 1**: Wait and retry
- Workflow automatically retries 3 times with exponential backoff
- Usually resolves after 30-second interval

**Solution 2**: Manual intervention (if auto-retry fails)
```bash
# Wait 2-3 minutes for crates.io index to fully update
# Manually trigger publish-on-tag workflow from GitHub Actions:
# Actions → Publish on Tag (Manual Only) → Run workflow
# Input: reinhardt-orm@v0.2.0
```

#### Issue: No Crates Detected for Publishing

**Cause**: No version changes detected or all versions already published

**Solution**:
```bash
# Verify Cargo.toml version was actually changed
git diff main -- crates/*/Cargo.toml

# Check if version already published
curl -s "https://crates.io/api/v1/crates/reinhardt-orm" | \
  jq '.versions[] | select(.num == "0.2.0")'

# If needed, update version and create new PR
```

#### Issue: CHANGELOG Extraction Failed

**Cause**: CHANGELOG.md format doesn't match expected pattern

**Expected format:**
```markdown
## [0.2.0] - 2025-01-15

### Added
- Feature description

### Fixed
- Bug fix description
```

**Solution**:
Follow [Keep a Changelog](https://keepachangelog.com/) format exactly.

#### Issue: Git Tag Already Exists

**Cause**: Previous failed publish attempt created tag

**Solution**:
```bash
# Delete remote tag
git push origin :refs/tags/reinhardt-orm@v0.2.0

# Workflow will recreate tag after successful publish
```

### Emergency Manual Publishing

If automated publishing fails completely, use the manual workflow:

#### Via GitHub Actions (Recommended)

1. Navigate to Actions → "Publish on Tag (Manual Only)"
2. Click "Run workflow"
3. Enter tag name: `reinhardt-orm@v0.2.0`
4. Click "Run workflow"
5. Monitor execution in Actions tab

**Note**: The tag must already exist. Create it first if needed:
```bash
git tag reinhardt-orm@v0.2.0 -m "Release reinhardt-orm v0.2.0"
git push origin reinhardt-orm@v0.2.0
```

#### Via Command Line

If GitHub Actions is unavailable:

```bash
# Follow steps in "Manual Publishing Process" section above
# Then create and push tag:
git tag reinhardt-orm@v0.2.0 -m "Release reinhardt-orm v0.2.0"
git push origin --tags
```

### Workflow Files Reference

**`.github/workflows/publish-dry-run.yml`**
- Triggers: PR with `release` label (on open, sync, reopen, labeled)
- Purpose: Pre-merge validation of publishability
- Actions: Detects changed crates, runs `--dry-run` for each

**`.github/workflows/publish-on-merge.yml`**
- Triggers: PR merge to main (only if PR has `release` label)
- Purpose: Publish to crates.io, create tags and releases
- Actions: Detect changes → publish → create tags → create releases → push tags

**`.github/workflows/publish-on-tag.yml`**
- Triggers: Manual dispatch only
- Purpose: Emergency manual publishing
- Actions: Validate tag → verify version → publish → create release

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
git tag -d [crate-name]@v[version]

# Delete remote tag (if pushed)
git push origin :refs/tags/[crate-name]@v[version]
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
  reinhardt-types = "0.1.0-alpha.1"  # Explicit version from crates.io
  ```

#### Issue: Tag already exists

**Cause:** Git tag already created (possibly from failed previous attempt)

**Solution:**
```bash
# Delete local tag
git tag -d [crate-name]@v[version]

# Delete remote tag (if pushed)
git push origin :refs/tags/[crate-name]@v[version]

# Recreate tag after fixing
git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"
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
git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"
git push origin main
git push origin --tags

# Yank (if needed)
cargo yank -p <crate-name> --vers <version>
```
