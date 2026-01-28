# Release Process

## Purpose

This document provides step-by-step procedures for releasing Reinhardt crates to
crates.io, covering version selection, CHANGELOG management, automated
publishing, and troubleshooting.

---

## Table of Contents

- [Overview](#overview)
- [Version Selection Guidelines](#version-selection-guidelines)
- [Pre-Release Checklist](#pre-release-checklist)
- [Automated Publishing with CI/CD](#automated-publishing-with-cicd)
- [CHANGELOG Management](#changelog-management)
- [Version Cascade Releases](#version-cascade-releases)
- [Multi-Crate Releases (Independent Updates)](#multi-crate-releases-independent-updates)
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

### Pre-Release Version Rules (alpha/beta/rc)

Until the initial stable release (e.g., `0.1.0`), **increment only the pre-release identifier** regardless of change type:

| Change Type | Version Update | Example |
|-------------|----------------|---------|
| Breaking change | Pre-release increment | `0.1.0-alpha.1` → `0.1.0-alpha.2` |
| New feature | Pre-release increment | `0.1.0-alpha.1` → `0.1.0-alpha.2` |
| Bug fix | Pre-release increment | `0.1.0-alpha.1` → `0.1.0-alpha.2` |

The same rule applies to rc (release candidate) versions:

| Change Type | Version Update | Example |
|-------------|----------------|---------|
| Breaking change | Pre-release increment | `0.1.0-rc.1` → `0.1.0-rc.2` |
| New feature | Pre-release increment | `0.1.0-rc.1` → `0.1.0-rc.2` |
| Bug fix | Pre-release increment | `0.1.0-rc.1` → `0.1.0-rc.2` |

**Rationale**: Pre-release versions are inherently unstable. Breaking changes are expected during alpha/beta/rc phases. Incrementing MINOR/MAJOR versions during pre-release would prematurely consume version numbers before stabilization.

**Pre-release progression**:
1. `0.1.0-alpha.x` → `0.1.0-rc.1` (alpha stabilized, ready for release candidate)
2. `0.1.0-rc.x` → `0.1.0` (rc stabilized, ready for stable release)

**Examples**:
- `0.1.0-alpha.5` → `0.1.0-rc.1` (transition from alpha to rc)
- `0.1.0-rc.3` → `0.1.0` (transition from rc to stable)

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

### Version Cascade

- [ ] Main crate (`reinhardt-web`) version updated if sub-crate version changed
- [ ] Main crate CHANGELOG.md references sub-crate changes

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

**1. Authentication Method**

**Recommended: Trusted Publishing (OIDC)** ⭐⭐⭐⭐⭐

- **No long-lived tokens**: Uses short-lived OIDC tokens (15-30 minutes)
- **No GitHub Secrets**: Authentication via OpenID Connect
- **Best security**: Industry best practice
- **Requires**: Initial publication with API token first

**Alternative: API Token** ⭐⭐⭐☆☆

- **For initial publication only**: Required before Trusted Publishing can be enabled
- **90-day expiration recommended**: Minimize security risk
- **Migrate to Trusted Publishing**: After first successful publication

**For Initial Publication (API Token)**:

1. Generate token at https://crates.io/settings/tokens
   - Name: `GitHub Actions - reinhardt-rs`
   - Expiration: **90 days recommended**
   - Scope: `publish-update` only
2. Add to GitHub Secrets:
   - Settings → Secrets and variables → Actions
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: (generated token)

**For Trusted Publishing (After Initial Publication)**:

See [Trusted Publishing Setup](#trusted-publishing-setup) section below.

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

### Trusted Publishing Setup

**Prerequisites**:

- At least one version published to crates.io (using API token)
- Crate ownership on crates.io

**Benefits**:

- ⭐⭐⭐⭐⭐ Security: Short-lived tokens (15-30 minutes)
- No GitHub Secrets management
- No token expiration concerns
- Industry best practice (PyPI, npm, RubyGems)

#### Step 1: Configure Trusted Publishers (All 40 Crates)

For **each published crate**, configure Trusted Publisher on crates.io:

1. Go to `https://crates.io/crates/[crate-name]/settings`
2. Navigate to "Trusted Publishing" section
3. Click "Add Trusted Publisher"
4. Fill in:
   - **Provider**: GitHub Actions
   - **Owner**: `kent8192`
   - **Repository**: `reinhardt-rs`
   - **Workflow**: `publish-on-merge.yml`
   - **Environment**: `release`

**Efficiency Tips**:

- Start with core crates: `reinhardt-macros`, `reinhardt-core`, `reinhardt-orm`
- Can use GitHub MCP tools, GitHub CLI, or API for batch configuration
- Only published crates need configuration (unpublished crates skip this)

#### Step 2: Create GitHub Environment

1. Repository Settings → Environments → "New environment"
2. **Name**: `release` (must match workflow configuration)
3. **Protection rules** (recommended):
   - Required reviewers: Add yourself
   - Wait timer: 0 minutes

#### Step 3: Verify Workflow Configuration

The workflow is already configured for Trusted Publishing:

```yaml
environment: release  # Enables OIDC
permissions:
  id-token: write  # Required for OIDC
```

#### Step 4: Test with Next Release

1. Create PR with version bump
2. Add `release` label
3. Merge PR → workflow uses OIDC automatically
4. Verify successful publication

#### Step 5: Clean Up (After Successful Test)

1. **Delete GitHub Secret**: Remove `CARGO_REGISTRY_TOKEN` from repository secrets
2. **Enable "Trusted Publishing Only"** mode on crates.io (optional):
   - Crate settings → "Trusted Publishing" → Enable "Require trusted publishing"
   - Prevents API token publishing (maximum security)

**Migration Summary**:

```
Before: API Token (90-day expiration) ⭐⭐⭐☆☆
After:  Trusted Publishing (15-30min tokens) ⭐⭐⭐⭐⭐
```

**References**:

- Official Guide: https://crates.io/docs/trusted-publishing
- RFC #3691: https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html
- GitHub Action: https://github.com/rust-lang/crates-io-auth-action

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

**"N/A" usage**: Only in `[Unreleased]` for empty categories. Released versions
should omit empty sections.

---

## Version Cascade Releases

### What is Version Cascade?

When a sub-crate's version is updated, the main crate (`reinhardt-web`) version **MUST** also be updated according to the Version Cascade Policy. This ensures:

- **Traceability**: All dependency changes are tracked in the main crate
- **SemVer Compliance**: Breaking changes in sub-crates propagate to the main crate
- **CHANGELOG Consistency**: All changes are documented centrally

### When Version Cascade Applies

Version Cascade applies when:

1. **Any sub-crate version is bumped** (MAJOR, MINOR, or PATCH)
2. **Multiple sub-crates are updated simultaneously**
3. **Sub-crate changes affect the main crate's API surface**

### Version Mapping Rules

#### Single Sub-Crate Update

Main crate version change MUST match sub-crate's change level:

| Sub-Crate Change | Main Crate Change | Example |
|------------------|-------------------|---------|
| MAJOR (X.0.0) | MAJOR (X.0.0) | `reinhardt-orm` 2.0.0 → `reinhardt-web` 1.0.0 |
| MINOR (0.X.0) | MINOR (0.X.0) | `reinhardt-rest` 0.2.0 → `reinhardt-web` 0.2.0 |
| PATCH (0.0.X) | PATCH (0.0.X) | `reinhardt-core` 0.1.1 → `reinhardt-web` 0.1.1 |

#### Multiple Sub-Crates Update

Main crate version follows the **highest priority** change:

**Priority**: MAJOR > MINOR > PATCH

**Example**:
- `reinhardt-orm` 0.1.0 → 0.2.0 (MINOR)
- `reinhardt-database` 0.1.0 → 0.1.1 (PATCH)
- **Result**: `reinhardt-web` 0.1.0 → 0.2.0 (MINOR wins)

### Quick Workflow

**3-Step Process**:

1. **Update Sub-Crate(s)**
   ```bash
   # Update Cargo.toml and CHANGELOG.md
   git add crates/[crate-name]/Cargo.toml crates/[crate-name]/CHANGELOG.md
   git commit -m "chore(release): bump [crate-name] to v[version]"
   ```

2. **Apply Version Cascade to Main Crate**
   ```bash
   # Update Cargo.toml and CHANGELOG.md (add Sub-Crate Updates section)
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-web to v[version] (cascade: [crate-name])"
   ```

3. **Create Atomic PR**
   ```bash
   git push origin [branch-name]
   gh pr create --title "chore(release): version cascade for [crate-name] v[version]" --label release
   ```

### CHANGELOG Reference Format

Main crate's `CHANGELOG.md` MUST include a **Sub-Crate Updates** subsection:

```markdown
## [0.2.0] - 2026-01-24

### Sub-Crate Updates

- `reinhardt-orm` updated to v0.2.0 ([CHANGELOG](crates/reinhardt-orm/CHANGELOG.md#020---2026-01-24))
  - Added support for complex JOIN queries
  - Fixed connection pool leak issue
```

**Key Elements**:
- Crate name with backticks
- Version number
- Link to sub-crate CHANGELOG with anchor: `#[version]---YYYY-MM-DD`
- Brief summary (1-3 bullet points)

### Complete Example

**Scenario**: `reinhardt-orm` adds new features (0.1.0 → 0.2.0)

**Steps**:

1. **Update Sub-Crate**:
   ```bash
   cd crates/reinhardt-orm
   # Edit Cargo.toml: version = "0.2.0"
   # Edit CHANGELOG.md: Add [0.2.0] section
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-orm to v0.2.0"
   ```

2. **Update Main Crate (Version Cascade)**:
   ```bash
   cd ../..  # Back to repository root
   # Edit Cargo.toml: version = "0.2.0"
   # Edit CHANGELOG.md: Add [0.2.0] section with Sub-Crate Updates
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-orm)"
   ```

   **Commit Message Body**:
   ```
   Version Cascade triggered by:
   - reinhardt-orm v0.1.0 → v0.2.0 (MINOR)

   Version Mapping: MINOR → MINOR

   Changes:
   - Added support for complex JOIN queries
   - Fixed connection pool leak issue
   ```

3. **Create PR**:
   ```bash
   git push origin feature/update-orm
   gh pr create --title "chore(release): version cascade for reinhardt-orm v0.2.0" \
                --label release \
                --body "See commits for details"
   ```

### For Detailed Implementation Guide

For comprehensive rules, edge cases, and automation considerations, see:
[docs/VERSION_CASCADE.md](VERSION_CASCADE.md)

---

## Multi-Crate Releases (Independent Updates)

**Note**: This section covers **independent multi-crate updates** where crates are updated in parallel without triggering Version Cascade. For dependency-driven updates, see [Version Cascade Releases](#version-cascade-releases) above.

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

**Dry-run Failed**: Add missing metadata fields (`description`, `license`,
`repository`) to `Cargo.toml`

**Version Already Published**: Increment version and retry

**No Crates Detected**: Verify version changes in `Cargo.toml` files

**cargo-workspaces Issues**: Run `cargo ws changed` to check detection; use
`--force` if needed

### Trusted Publishing Issues

**"please provide a non-empty token" Error**:

- **If using API Token**: Verify `CARGO_REGISTRY_TOKEN` secret is set correctly
- **If migrating to Trusted Publishing**: Ensure environment name matches (`release`)

**"OIDC token not found" Error**:

1. Verify `environment: release` in workflow
2. Verify `id-token: write` permission
3. Check GitHub Environment exists (Settings → Environments)

**"Trusted publisher not configured" Error**:

- Configure Trusted Publisher on crates.io for the specific crate
- Verify repository, workflow, and environment names match exactly

**Permission Denied**:

- Ensure you have ownership of the crate on crates.io
- Verify Trusted Publisher configuration uses correct GitHub username/org

**Environment Protection Rules Blocking**:

- Approve the deployment in GitHub Actions UI
- Adjust protection rules if needed (Settings → Environments → release)

### Verification Checklist

- [ ] Version follows SemVer
- [ ] CHANGELOG updated
- [ ] Tests pass
- [ ] Dry-run succeeds
- [ ] Dependencies published

---

## Related Documentation

- **Main Quick Reference**: @CLAUDE.md (see Quick Reference section)
- **Main Standards**: @CLAUDE.md
- **Commit Guidelines**: @COMMIT_GUIDELINE.md
- **Version Policy**: See "Release & Publishing Policy" in CLAUDE.md
