# Version Cascade Policy

## Overview

When a sub-crate's version is updated, the main crate (`reinhardt-web`) version MUST also be updated according to this Version Cascade Policy. This ensures:

- **Traceability**: All dependency changes are tracked in the main crate
- **Semantic Versioning Compliance**: Breaking changes in sub-crates propagate to the main crate
- **CHANGELOG Consistency**: All changes are documented in a centralized location

This document provides the complete implementation guide for Version Cascade Policy.

---

## Version Mapping Rules

### VCR-1: Direct Mapping (Single Sub-Crate Update)

When a single sub-crate is updated, the main crate version change MUST match the sub-crate's version change level:

| Sub-Crate Change | Main Crate Change | Example |
|------------------|-------------------|---------|
| MAJOR (X.0.0) | MAJOR (X.0.0) | `reinhardt-orm` 2.0.0 → `reinhardt-web` 1.0.0 |
| MINOR (0.X.0) | MINOR (0.X.0) | `reinhardt-rest` 0.2.0 → `reinhardt-web` 0.2.0 |
| PATCH (0.0.X) | PATCH (0.0.X) | `reinhardt-core` 0.1.1 → `reinhardt-web` 0.1.1 |

**Rationale**: If a sub-crate has a breaking change (MAJOR), the main crate's API is also affected. Similarly, new features (MINOR) or bug fixes (PATCH) in sub-crates should be reflected in the main crate.

### VCR-2: Priority Mapping (Multiple Sub-Crates Update)

When multiple sub-crates are updated simultaneously, the main crate version change MUST follow the highest priority change:

**Priority Order**: MAJOR > MINOR > PATCH

**Example**:
- `reinhardt-orm` updated from 0.1.0 to 0.2.0 (MINOR)
- `reinhardt-database` updated from 0.1.0 to 0.1.1 (PATCH)
- **Result**: `reinhardt-web` updated from 0.1.0 to 0.2.0 (MINOR, following the higher priority)

### VCR-3: Pre-1.0.0 Exception

For Pre-1.0.0 versions (0.x.x), Semantic Versioning allows breaking changes in MINOR versions:

- Sub-crate MINOR update with breaking changes → Main crate MINOR update
- No exception to VCR-1 and VCR-2 (same mapping rules apply)

**Clarification**: This rule does NOT change the mapping logic—it simply acknowledges that Pre-1.0.0 MINOR updates may contain breaking changes per SemVer 2.0.0 specification.

### VCR-4: Pre-Release Version Exception (alpha/beta/rc)

For pre-release versions (e.g., `0.1.0-alpha.x`), **only increment the pre-release identifier** until the initial stable release:

| Change Type | Sub-Crate Update | Main Crate Update |
|-------------|------------------|-------------------|
| Breaking change | `0.1.0-alpha.1` → `0.1.0-alpha.2` | `0.1.0-alpha.1` → `0.1.0-alpha.2` |
| New feature | `0.1.0-alpha.1` → `0.1.0-alpha.2` | `0.1.0-alpha.1` → `0.1.0-alpha.2` |
| Bug fix | `0.1.0-alpha.1` → `0.1.0-alpha.2` | `0.1.0-alpha.1` → `0.1.0-alpha.2` |

The same rule applies to rc (release candidate) versions:

| Change Type | Sub-Crate Update | Main Crate Update |
|-------------|------------------|-------------------|
| Breaking change | `0.1.0-rc.1` → `0.1.0-rc.2` | `0.1.0-rc.1` → `0.1.0-rc.2` |
| New feature | `0.1.0-rc.1` → `0.1.0-rc.2` | `0.1.0-rc.1` → `0.1.0-rc.2` |
| Bug fix | `0.1.0-rc.1` → `0.1.0-rc.2` | `0.1.0-rc.1` → `0.1.0-rc.2` |

**Key Point**: Do NOT update to `0.2.0-alpha.1` or `0.2.0-rc.1` for breaking changes during pre-release phase. Pre-release versions are inherently unstable, and breaking changes are expected.

**Pre-release progression**:
1. `0.1.0-alpha.x` → `0.1.0-rc.1` (alpha stabilized, ready for release candidate)
2. `0.1.0-rc.x` → `0.1.0` (rc stabilized, ready for stable release)

---

## CHANGELOG Reference Format

### CRF-1: Standard Format (Sub-Crate Updates Subsection)

The main crate's `CHANGELOG.md` MUST include a **Sub-Crate Updates** subsection under the appropriate version section:

```markdown
## [0.2.0] - 2026-01-24

### Sub-Crate Updates

- `reinhardt-orm` updated to v0.2.0 ([CHANGELOG](crates/reinhardt-orm/CHANGELOG.md#020---2026-01-24))
  - Added support for complex JOIN queries
  - Fixed connection pool leak issue
```

**Rules**:
- **Mandatory Fields**: Crate name, version, CHANGELOG link, brief summary
- **Link Format**: Relative path from repository root + anchor
- **Summary**: 1-3 bullet points highlighting key changes (extracted from sub-crate CHANGELOG)

### CRF-2: CHANGELOG Link Anchor Format

Anchors MUST follow GitHub's auto-generated anchor format:

**Pattern**: `#[version]---YYYY-MM-DD`

**Examples**:
- `#020---2026-01-24` (for version 0.2.0)
- `#100---2026-02-15` (for version 1.0.0)
- `#010-alpha1---2026-01-20` (for version 0.1.0-alpha.1)

**Generation Rule**: Replace `.` with empty string, replace `-` with empty string except for the final separator before date.

### CRF-3: Multiple Sub-Crates Reference

When multiple sub-crates are updated, list them in alphabetical order:

```markdown
## [0.3.0] - 2026-02-01

### Sub-Crate Updates

- `reinhardt-database` updated to v0.2.0 ([CHANGELOG](crates/reinhardt-database/CHANGELOG.md#020---2026-02-01))
  - Migrated to SeaQuery 1.0.0-rc.2
- `reinhardt-orm` updated to v0.3.0 ([CHANGELOG](crates/reinhardt-orm/CHANGELOG.md#030---2026-02-01))
  - BREAKING: Changed `Model` trait signature
  - Added async/await support
- `reinhardt-rest` updated to v0.2.1 ([CHANGELOG](crates/reinhardt-rest/CHANGELOG.md#021---2026-02-01))
  - Fixed JSON serialization bug
```

---

## Commit Strategy

### CS-1: Individual Commits (Sub-Crate → Main Crate)

Each crate version bump MUST be committed individually:

**Order**:
1. Sub-crate commits (in dependency order, leaf-first)
2. Main crate commit (last)

**Example**:
```bash
# 1. Sub-crate commit
git add crates/reinhardt-orm/Cargo.toml crates/reinhardt-orm/CHANGELOG.md
git commit -m "chore(release): bump reinhardt-orm to v0.2.0"

# 2. Main crate commit
git add Cargo.toml CHANGELOG.md
git commit -m "chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-orm)"
```

**Rationale**: Individual commits enable:
- Precise git bisect for troubleshooting
- Clear git log history
- Selective cherry-picking if needed

### CS-2: Main Crate Commit Message Format

Main crate version bump commits MUST include the `cascade:` keyword to indicate Version Cascade:

**Subject Format**:
```
chore(release): bump reinhardt-web to v[version] (cascade: [sub-crate-list])
```

**Body Format**:
```
Version Cascade triggered by:
- [crate-name] v[old] → v[new] ([MAJOR|MINOR|PATCH])

Version Mapping: [sub-crate-change] → [main-crate-change]

Changes:
- [Brief summary extracted from sub-crate CHANGELOG]
```

**Example**:
```
chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-orm)

Version Cascade triggered by:
- reinhardt-orm v0.1.0 → v0.2.0 (MINOR)

Version Mapping: MINOR → MINOR

Changes:
- Added support for complex JOIN queries
- Fixed connection pool leak issue
```

**For Multiple Sub-Crates**:
```
chore(release): bump reinhardt-web to v0.3.0 (cascade: reinhardt-database, reinhardt-orm, reinhardt-rest)

Version Cascade triggered by:
- reinhardt-database v0.1.0 → v0.2.0 (MINOR)
- reinhardt-orm v0.2.0 → v0.3.0 (MINOR)
- reinhardt-rest v0.2.0 → v0.2.1 (PATCH)

Version Mapping: MINOR (highest priority) → MINOR

Changes:
- reinhardt-database: Migrated to SeaQuery 1.0.0-rc.2
- reinhardt-orm: BREAKING - Changed Model trait signature, added async/await support
- reinhardt-rest: Fixed JSON serialization bug
```

### CS-3: Atomic PR (All Commits in Single PR)

All version bump commits (sub-crate + main crate) MUST be included in a single PR:

**PR Title Format**:
```
chore(release): version cascade for [sub-crate-list] v[version]
```

**PR Description Template**:
```markdown
## Version Cascade Summary

This PR implements Version Cascade Policy following sub-crate updates:

### Updated Crates

- [ ] `reinhardt-orm` v0.1.0 → v0.2.0
- [ ] `reinhardt-web` v0.1.0 → v0.2.0 (cascade)

### Version Mapping

- Sub-crate change: MINOR (0.1.0 → 0.2.0)
- Main crate change: MINOR (0.1.0 → 0.2.0)
- Mapping rule: VCR-1 (Direct Mapping)

### Commit Structure

1. `chore(release): bump reinhardt-orm to v0.2.0` - Sub-crate version bump
2. `chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-orm)` - Main crate version cascade

### CHANGELOG Updates

- [x] `crates/reinhardt-orm/CHANGELOG.md` updated
- [x] `CHANGELOG.md` updated with Sub-Crate Updates section

### Related Issues

- Fixes #XXX (if applicable)

### Checklist

- [x] All version bumps committed individually
- [x] Main crate commit includes `cascade:` keyword
- [x] CHANGELOG.md includes Sub-Crate Updates section
- [x] CHANGELOG links use correct anchor format
- [x] Version mapping follows VCR-1/VCR-2/VCR-3
- [x] All tests pass (`cargo test --workspace --all --all-features`)
```

**Rationale**: Atomic PRs ensure:
- All related changes are reviewed together
- No partial version cascades are merged
- Easy rollback if issues are found

---

## Workflow Examples

### Example 1: Single Sub-Crate MINOR Update

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

3. **Create PR**:
   ```bash
   git push origin docs/update-version-cascade-policy
   gh pr create --title "chore(release): version cascade for reinhardt-orm v0.2.0" \
                --label release
   ```

### Example 2: Multiple Sub-Crates (MINOR + PATCH)

**Scenario**:
- `reinhardt-database` adds features (0.1.0 → 0.2.0, MINOR)
- `reinhardt-rest` fixes bugs (0.2.0 → 0.2.1, PATCH)

**Version Mapping**: MINOR (higher priority) → MINOR

**Steps**:

1. **Update Sub-Crates** (dependency order, leaf-first):
   ```bash
   # 1. reinhardt-database (no dependency on reinhardt-rest)
   git add crates/reinhardt-database/Cargo.toml crates/reinhardt-database/CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-database to v0.2.0"

   # 2. reinhardt-rest (depends on reinhardt-database)
   git add crates/reinhardt-rest/Cargo.toml crates/reinhardt-rest/CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-rest to v0.2.1"
   ```

2. **Update Main Crate**:
   ```bash
   # Version: 0.1.0 → 0.2.0 (MINOR, following higher priority)
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-database, reinhardt-rest)"
   ```

3. **Commit Message Body**:
   ```
   Version Cascade triggered by:
   - reinhardt-database v0.1.0 → v0.2.0 (MINOR)
   - reinhardt-rest v0.2.0 → v0.2.1 (PATCH)

   Version Mapping: MINOR (highest priority) → MINOR

   Changes:
   - reinhardt-database: Migrated to SeaQuery 1.0.0-rc.2
   - reinhardt-rest: Fixed JSON serialization bug
   ```

### Example 3: MAJOR Breaking Change

**Scenario**: `reinhardt-orm` introduces breaking API changes (0.2.0 → 1.0.0)

**Version Mapping**: MAJOR → MAJOR (reinhardt-web 0.2.0 → 1.0.0)

**Steps**:

1. **Update Sub-Crate**:
   ```bash
   git add crates/reinhardt-orm/Cargo.toml crates/reinhardt-orm/CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-orm to v1.0.0"
   ```

2. **Update Main Crate**:
   ```bash
   # MAJOR version bump: 0.2.0 → 1.0.0
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore(release): bump reinhardt-web to v1.0.0 (cascade: reinhardt-orm)"
   ```

3. **CHANGELOG.md (Main Crate)**:
   ```markdown
   ## [1.0.0] - 2026-03-01

   ### Sub-Crate Updates

   - `reinhardt-orm` updated to v1.0.0 ([CHANGELOG](crates/reinhardt-orm/CHANGELOG.md#100---2026-03-01))
     - **BREAKING**: Changed `Model` trait signature
     - **BREAKING**: Removed deprecated `connect()` function
     - Added async/await support for all operations
   ```

---

## Edge Cases

### Edge Case 1: Workspace Version Sharing

**Scenario**: Multiple crates share the same version via `version.workspace = true`

**Rule**: Update the workspace version in root `Cargo.toml`, then update main crate `CHANGELOG.md` to reference ALL affected sub-crates.

**Example**:
```toml
# Root Cargo.toml
[workspace.package]
version = "0.2.0"  # Shared version
```

**CHANGELOG.md (Main Crate)**:
```markdown
## [0.2.0] - 2026-01-24

### Sub-Crate Updates

- `reinhardt-core`, `reinhardt-orm`, `reinhardt-database` updated to v0.2.0 (workspace version)
  - See individual crate CHANGELOGs for details
```

### Edge Case 2: Indirect Dependency Update

**Scenario**: Sub-crate A updates, which requires sub-crate B to update (indirect dependency)

**Rule**: List both crates in the main crate's Sub-Crate Updates, but only mention the direct trigger in commit message.

**Example**:
```
chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-database)

Version Cascade triggered by:
- reinhardt-database v0.1.0 → v0.2.0 (MINOR)
  - Triggered reinhardt-orm update v0.1.0 → v0.1.1 (dependency compatibility)

Version Mapping: MINOR → MINOR
```

### Edge Case 3: Optional Dependency Update

**Scenario**: Optional sub-crate feature is updated

**Rule**: Version Cascade still applies if the sub-crate is part of the workspace. Use `(optional)` notation in CHANGELOG.

**Example**:
```markdown
## [0.2.0] - 2026-01-24

### Sub-Crate Updates

- `reinhardt-cli` (optional) updated to v0.2.0 ([CHANGELOG](crates/reinhardt-cli/CHANGELOG.md#020---2026-01-24))
  - Added new command-line options
```

### Edge Case 4: Metadata-Only Change (No Version Bump)

**Scenario**: Sub-crate's `Cargo.toml` metadata (description, keywords) is updated without version bump

**Rule**: NO Version Cascade required. Metadata-only changes do NOT trigger version updates.

**Exception**: If documentation or README is significantly updated, consider a PATCH version bump.

---

## Automation Considerations

### Current State: Manual Process

Version Cascade Policy is currently enforced through manual review and adherence to this document.

**Manual Steps**:
1. Developer updates sub-crate version and CHANGELOG
2. Developer calculates main crate version change using VCR-1/VCR-2/VCR-3
3. Developer updates main crate `Cargo.toml` and `CHANGELOG.md`
4. Developer creates commits following CS-1/CS-2/CS-3
5. Reviewer verifies version mapping and CHANGELOG format

### Future: CI Check (Phase 3)

**Proposed CI Check** (`version-cascade-check.yml`):

```yaml
name: Version Cascade Check

on:
  pull_request:
    paths:
      - 'crates/*/Cargo.toml'
      - 'Cargo.toml'

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for version comparison

      - name: Detect Sub-Crate Version Changes
        id: detect
        run: |
          # Compare Cargo.toml versions between base and head
          # Output: changed_crates, version_changes, mapping_level

      - name: Verify Main Crate Version Cascade
        run: |
          # Check if main crate version matches expected mapping
          # Fail if Version Cascade Policy is violated

      - name: Validate CHANGELOG Format
        run: |
          # Verify Sub-Crate Updates section exists
          # Verify CHANGELOG links use correct anchor format
          # Verify all changed sub-crates are referenced
```

**Benefits**:
- Automatic detection of Version Cascade violations
- Enforce CHANGELOG format consistency
- Reduce manual review burden

### Future: Automated Version Bump Script

**Proposed Tool**: `scripts/version-cascade.sh`

```bash
#!/usr/bin/env bash
# Usage: ./scripts/version-cascade.sh <sub-crate-name> <new-version>
# Example: ./scripts/version-cascade.sh reinhardt-orm 0.2.0

SUB_CRATE=$1
NEW_VERSION=$2

# 1. Detect version change level (MAJOR/MINOR/PATCH)
# 2. Update sub-crate Cargo.toml and CHANGELOG.md
# 3. Calculate main crate version using VCR-1/VCR-2
# 4. Update main crate Cargo.toml and CHANGELOG.md
# 5. Create individual commits following CS-1/CS-2
```

**Benefits**:
- Eliminate manual version calculation
- Ensure consistent commit message format
- Reduce human error in version mapping

---

## Quick Reference

### Version Mapping Cheat Sheet

| Scenario | Sub-Crate | Main Crate | Rule |
|----------|-----------|------------|------|
| Single MAJOR | 1.0.0 | 1.0.0 | VCR-1 |
| Single MINOR | 0.2.0 | 0.2.0 | VCR-1 |
| Single PATCH | 0.1.1 | 0.1.1 | VCR-1 |
| MINOR + PATCH | 0.2.0, 0.1.1 | 0.2.0 | VCR-2 (MINOR wins) |
| MAJOR + MINOR | 1.0.0, 0.2.0 | 1.0.0 | VCR-2 (MAJOR wins) |

### CHANGELOG Anchor Examples

| Version | Anchor |
|---------|--------|
| 0.2.0 | `#020---2026-01-24` |
| 1.0.0 | `#100---2026-02-15` |
| 0.1.0-alpha.1 | `#010-alpha1---2026-01-20` |
| 2.3.4 | `#234---2026-03-10` |

### Commit Message Template

**Sub-Crate**:
```
chore(release): bump [crate-name] to v[version]
```

**Main Crate**:
```
chore(release): bump reinhardt-web to v[version] (cascade: [crate-list])

Version Cascade triggered by:
- [crate-name] v[old] → v[new] ([MAJOR|MINOR|PATCH])

Version Mapping: [change-level] → [change-level]

Changes:
- [Brief summary]
```

---

## Related Documentation

- [CLAUDE.md](../CLAUDE.md) - Project rules and quick reference
- [docs/RELEASE_PROCESS.md](RELEASE_PROCESS.md) - Complete release workflow
- [docs/COMMIT_GUIDELINE.md](COMMIT_GUIDELINE.md) - Commit message standards
- [docs/DOCUMENTATION_STANDARDS.md](DOCUMENTATION_STANDARDS.md) - CHANGELOG formatting rules

---

## Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2026-01-24 | 1.0.0 | Initial Version Cascade Policy documentation |
