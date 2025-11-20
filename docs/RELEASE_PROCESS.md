# Release Process

## Purpose

This document provides step-by-step procedures for releasing Reinhardt crates to crates.io, covering version selection, CHANGELOG management, automated publishing, and troubleshooting.

---

## Table of Contents

- [Overview](#overview)
- [Version Selection Guidelines](#version-selection-guidelines)
- [Pre-Release Checklist](#pre-release-checklist)
- [Automated Publishing with CI/CD](#automated-publishing-with-cicd)
- [CHANGELOG Management](#changelog-management)
- [Multi-Crate Releases](#multi-crate-releases)
- [Rollback Procedures](#rollback-procedures)
- [Troubleshooting](#troubleshooting)
- [Quick Reference](#quick-reference)

---

## Overview

### Release Strategy

Reinhardt follows a **per-crate versioning and tagging strategy**:

- **Independent Versioning**: Each crate maintains its own version
- **Selective Releases**: Only release changed crates
- **Semantic Versioning**: Strict adherence to SemVer 2.0.0
- **Git Tags**: Format `[crate-name]@v[version]`

### Key Principles

1. **Explicit Authorization**: Every release requires user approval
2. **Dry-Run First**: Always verify before publication
3. **Commit Before Tag**: Version bumps committed before tagging
4. **Comprehensive Testing**: All tests must pass

### Tools

**cargo-workspaces** handles change detection and publishing:

```bash
cargo install cargo-workspaces --version 0.4.1
```

**Key commands**:
- `cargo ws changed` - Detect changed crates
- `cargo ws publish --dry-run` - Validate
- `cargo ws publish` - Publish

---

## Version Selection Guidelines

### Semantic Versioning 2.0.0

**Format:** `MAJOR.MINOR.PATCH`

#### MAJOR (X.0.0) - Breaking Changes

- Breaking API changes
- Removed/renamed public items
- Changed trait bounds

```rust
// MAJOR bump required
// Before: pub fn connect(url: &str) -> Connection
// After:  pub fn connect(url: &str) -> Result<Connection>
```

#### MINOR (0.X.0) - New Features

- New public APIs
- New optional features
- Backward-compatible enhancements
- Deprecations

```rust
// MINOR bump
impl Pool {
	pub fn with_timeout(self, duration: Duration) -> Self { }  // New method
}
```

#### PATCH (0.0.X) - Bug Fixes

- Bug fixes (no API changes)
- Performance improvements
- Documentation corrections

### Pre-1.0.0 Rules

- **MINOR (0.X.0)**: MAY include breaking changes
- **PATCH (0.0.X)**: MUST be backward compatible

**After 1.0.0**: Breaking changes REQUIRE MAJOR version bump

---

## Pre-Release Checklist

### Code Quality

- [ ] Tests pass: `cargo test --workspace --all --all-features`
- [ ] Build succeeds: `cargo build --workspace --all --all-features`
- [ ] Linting passes: `cargo make clippy-fix`
- [ ] Formatting applied: `cargo make fmt-fix`

### Documentation

- [ ] README.md updated (if API changed)
- [ ] `src/lib.rs` documentation updated
- [ ] CHANGELOG.md prepared
- [ ] Code examples tested: `cargo test --doc`

### Metadata

- [ ] `description`, `license`, `repository` fields present
- [ ] Dependencies up-to-date
- [ ] No dev-dependencies on other Reinhardt crates (functional crates)

**Verify with**:
```bash
cargo publish --dry-run -p <crate-name>
```

---

## Automated Publishing with CI/CD

### Prerequisites

#### GitHub Repository Setup

**1. GitHub Secrets**

Add `CARGO_REGISTRY_TOKEN`:
- Settings → Secrets and variables → Actions
- Name: `CARGO_REGISTRY_TOKEN`
- Value: crates.io API token (from https://crates.io/settings/tokens)

**2. GitHub Labels**

Create `release` label:
- Issues → Labels → New label
- Name: `release`, Color: `#0e8a16`

**3. Branch Protection (Recommended)**

- Require PR reviews
- Require `Publish Dry-Run` status check
- Require branches to be up-to-date

### Publishing Workflow

#### Step 1: Create PR with Version Changes

```bash
# 1. Create feature branch
git checkout -b feature/update-reinhardt-orm

# 2. Update version
vim crates/reinhardt-orm/Cargo.toml  # version = "0.2.0"

# 3. Update CHANGELOG
vim crates/reinhardt-orm/CHANGELOG.md

# 4. Commit
git add crates/reinhardt-orm/Cargo.toml crates/reinhardt-orm/CHANGELOG.md
git commit -m "chore(release): Bump reinhardt-orm to v0.2.0

Prepare reinhardt-orm for publication to crates.io.

Version Changes:
- crates/reinhardt-orm/Cargo.toml: version 0.1.0 -> 0.2.0
- crates/reinhardt-orm/CHANGELOG.md: Add release notes for v0.2.0

New Features:
- Add async connection pooling
- Implement QueryBuilder::with_timeout()

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

# 5. Push and create PR
git push origin feature/update-reinhardt-orm
```

#### Step 2: Add `release` Label

1. Open PR on GitHub
2. Add `release` label
3. `publish-dry-run.yml` workflow starts automatically

**Workflow actions**:
- Detects changed crates (`cargo ws changed`)
- Runs `cargo ws publish --dry-run`
- Reports results in PR checks

#### Step 3: Review Dry-Run Results

**Success indicators**:
- ✅ Green check on "Publish Dry-Run"
- No errors/warnings in logs

**Common issues**:
- Missing metadata fields
- Unpublished dependencies
- Version already published

**Fix and push** - dry-run re-runs automatically.

#### Step 4: Merge PR

Merge after:
- Dry-run passes
- PR approved

#### Step 5: Automatic Publishing

`publish-on-merge.yml` executes:

1. **Detect changes** (`cargo ws changed`)
2. **Publish sequentially**:
   - Dry-run verification
   - Publish to crates.io
   - Create Git tag
   - 30s wait between crates
3. **Create GitHub Releases** (with CHANGELOG notes)
4. **Push tags**

**Example output**:
```
[1/2] Publishing reinhardt-types v0.2.0...
  ✅ Dry-run passed
  ✅ Published to crates.io
  🏷️ Created tag: reinhardt-types@v0.2.0

[2/2] Publishing reinhardt-orm v0.3.0...
  ✅ Dry-run passed
  ✅ Published to crates.io
  🏷️ Created tag: reinhardt-orm@v0.3.0

🎉 All crates published successfully!
```

#### Step 6: Verify Publication

**crates.io**:
- Visit https://crates.io/crates/[crate-name]
- Verify new version

**GitHub Releases**:
- Check tag `[crate-name]@v[version]`
- Verify release notes

**docs.rs**:
- Documentation builds in ~5-10 minutes

### Multi-Crate Releases

Update multiple crates in one PR:

```bash
vim crates/reinhardt-types/Cargo.toml     # version = "0.2.0"
vim crates/reinhardt-types/CHANGELOG.md

vim crates/reinhardt-orm/Cargo.toml       # version = "0.3.0"
vim crates/reinhardt-orm/CHANGELOG.md

git add crates/reinhardt-types crates/reinhardt-orm
git commit -m "chore(release): Bump reinhardt-types v0.2.0 and reinhardt-orm v0.3.0"
```

**Automatic dependency ordering**: Publishes leaf crates first.

### Emergency Manual Publishing

**Via GitHub Actions**:
1. Actions → "Publish on Tag (Manual Only)"
2. Enter tag: `reinhardt-orm@v0.2.0`
3. Run workflow

**Via Command Line** (if CI/CD unavailable):
1. Update `Cargo.toml` and `CHANGELOG.md`
2. Commit changes
3. `cargo publish --dry-run -p <crate-name>`
4. `cargo publish -p <crate-name>`
5. `git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"`
6. `git push origin main && git push origin --tags`

---

## CHANGELOG Management

### Format (Keep a Changelog)

**Structure**:
```markdown
# Changelog

## [Unreleased]

### Added
- Work in progress features

### Changed
- N/A

## [0.2.0] - 2025-01-15

### Breaking Changes
- List breaking changes first

### Added
- New features

### Fixed
- Bug fixes

## [0.1.0] - 2024-12-01

### Added
- Initial release
```

### Critical Requirements

1. **Always include `[Unreleased]` section** (for AWK extraction)
2. **Exact header format**: `## [version] - YYYY-MM-DD`
   - ✅ Correct: `## [0.2.0] - 2025-01-15`
   - ❌ Wrong: `## 0.2.0 - 2025-01-15` (no brackets)
   - ❌ Wrong: `## [v0.2.0] - 2025-01-15` (extra 'v')
3. **Use `###` for subsections**: `### Added`, `### Fixed`

**Extraction logic**:
```bash
awk "/## \[$VERSION\]/,/## \[/" CHANGELOG.md | head -n -1
```

### Section Guidelines

- **Breaking Changes**: List first, include migration guides
- **Added**: New public APIs and features
- **Changed**: Non-breaking changes
- **Deprecated**: Mark with `#[deprecated]`, include removal timeline
- **Removed**: Deleted features (MAJOR version)
- **Fixed**: Bug fixes (include issue numbers)
- **Security**: Vulnerabilities (highlight prominently)

**"N/A" usage**: Only in `[Unreleased]` for empty categories. Released versions should omit empty sections.

---

## Multi-Crate Releases

### When to Release Multiple Crates

- Dependency updates cascade
- Coordinated feature releases
- Breaking changes affect multiple crates

### Dependency Order

**Automatic with cargo-workspaces**: Publishes in correct order.

**Manual order** (if needed):
1. Leaf crates (no internal deps)
2. Mid-level crates
3. Top-level crates (facades)

**Example**:
```
reinhardt-types → reinhardt-orm → reinhardt (facade)
```

### Sub-Crate Structures

**Nested crates** (e.g., `reinhardt-db/crates/orm/`):
- Sub-crates published before parent
- `cargo ws publish` handles automatically

---

## Rollback Procedures

### Version Yanking (Preferred)

```bash
cargo yank <crate-name> --version <version>
```

**What it does**:
- Prevents new projects from using version
- Existing `Cargo.lock` still works
- Cannot delete (remains on crates.io)

**After yanking**:
1. Fix issue
2. Increment PATCH version
3. Release fixed version

### Un-yanking

```bash
cargo yank <crate-name> --version <version> --undo
```

### Git Tag Rollback

```bash
git tag -d [crate-name]@v[version]
git push origin :refs/tags/[crate-name]@v[version]
```

### Commit Rollback

```bash
git revert HEAD  # If pushed
git reset --hard HEAD~1  # If not pushed
```

---

## Troubleshooting

### Common Issues

#### Dry-run Failed - Missing Metadata

**Error**: `missing field 'description' in Cargo.toml`

**Solution**:
```toml
[package]
description = "ORM layer for Reinhardt framework"
license = "MIT OR Apache-2.0"
repository = "https://github.com/kent8192/reinhardt-rs"
```

#### Version Already Published

**Solution**:
```bash
# Check existing versions
curl -s "https://crates.io/api/v1/crates/reinhardt-orm" | jq '.versions[].num'

# Increment version and retry
vim crates/reinhardt-orm/Cargo.toml  # version = "0.2.1"
```

#### Dependency Not Available

**Cause**: Dependency not published or index not updated

**Solution**:
- Workflow retries automatically (3 attempts)
- Manual: Wait 2-3 minutes, trigger manually

#### No Crates Detected

**Cause**: No version changes or already published

**Solution**:
```bash
# Verify changes
git diff main -- crates/*/Cargo.toml

# Check if published
curl -s "https://crates.io/api/v1/crates/reinhardt-orm" | \
  jq '.versions[] | select(.num == "0.2.0")'
```

#### Git Tag Already Exists

**Solution**:
```bash
git push origin :refs/tags/reinhardt-orm@v0.2.0
# Workflow recreates after successful publish
```

### cargo-workspaces Issues

#### "No changed crates detected"

```bash
cargo ws changed  # Check detection

# Force publish
cargo ws publish -p <crate-name> --force

# Or create missing tags
git tag <crate-name>@v<version> -m "Retroactive tag"
```

#### "version mismatch for workspace dependency"

```bash
# Check consistency
grep -A 50 "\[workspace.dependencies\]" Cargo.toml
grep "workspace = true" crates/*/Cargo.toml
```

#### cargo-workspaces not found

```bash
cargo install cargo-workspaces --version 0.4.1
cargo ws --version
```

### Verification Checklist

- [ ] Version follows SemVer
- [ ] CHANGELOG updated
- [ ] Tests pass
- [ ] Documentation builds
- [ ] Dry-run succeeds
- [ ] No uncommitted changes
- [ ] Dependencies published

---

## Quick Reference

### Complete Release Checklist

```
1. ✅ Select version (MAJOR/MINOR/PATCH)
2. ✅ Update Cargo.toml + CHANGELOG.md
3. ✅ Run verification commands
4. ✅ Create PR + add 'release' label
5. ✅ Review dry-run results
6. ✅ Merge PR
7. ✅ Verify publication (crates.io, GitHub Releases, docs.rs)
```

### Key Commands

```bash
# Verification
cargo check --workspace --all --all-features
cargo test --workspace --all --all-features
cargo test --doc -p <crate-name>

# Change detection
cargo ws changed

# Dry-run
cargo ws publish --dry-run -p <crate-name>

# Publish (manual)
cargo ws publish -p <crate-name>

# Tagging (manual)
git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"
git push origin main && git push origin --tags

# Yank
cargo yank <crate-name> --version <version>
cargo yank <crate-name> --version <version> --undo
```

### Workflow Files

- **`.github/workflows/publish-dry-run.yml`**: Pre-merge validation
- **`.github/workflows/publish-on-merge.yml`**: Auto-publish on merge
- **`.github/workflows/publish-on-tag.yml`**: Manual emergency publish

---

## Related Documentation

- **Main Standards**: @CLAUDE.md
- **Commit Guidelines**: @CLAUDE.commit.md
- **Version Policy**: See "Release & Publishing Policy" in CLAUDE.md
