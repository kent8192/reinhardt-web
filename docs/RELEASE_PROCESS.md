# Release Process

## Purpose

This document provides step-by-step procedures for releasing Reinhardt crates to
crates.io using release-plz for automated versioning, CHANGELOG generation, and
publishing.

---

## Table of Contents

- [Overview](#overview)
- [How release-plz Works](#how-release-plz-works)
- [Automated Workflow](#automated-workflow)
- [Manual Intervention](#manual-intervention)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

---

## Overview

### Release Strategy

Reinhardt uses **release-plz** for fully automated release management:

- **Automated Versioning**: Versions are determined from conventional commits
- **Automated CHANGELOGs**: Generated from commit messages
- **Release PRs**: Automatically created when changes are detected
- **Git Tags**: Format `[crate-name]@v[version]`
- **GitHub Releases**: Created automatically upon merge

### Key Principles

1. **Conventional Commits**: Use proper commit message format for version bumps
2. **Automated Process**: release-plz handles version bumps and CHANGELOGs
3. **Review Before Release**: Release PRs allow review before publishing
4. **Per-Crate Releases**: Only changed crates are released

---

## How release-plz Works

### Commit-to-Version Mapping

release-plz uses [Conventional Commits](https://www.conventionalcommits.org/) to determine version bumps:

| Commit Type | Version Bump | Example |
|-------------|--------------|---------|
| `feat:` | MINOR | `feat(auth): add OAuth support` |
| `fix:` | PATCH | `fix(orm): resolve connection leak` |
| `feat!:` or `BREAKING CHANGE:` | MAJOR | `feat!: change API response format` |
| Other types | PATCH | `docs:`, `chore:`, `refactor:`, etc. |

### Automated CHANGELOG Generation

CHANGELOGs are generated from commit messages:

```markdown
## [0.2.0] - 2026-01-30

### Added
- Add OAuth support (#123)

### Fixed
- Resolve connection leak in pool (#124)
```

---

## Automated Workflow

### Step 1: Develop with Conventional Commits

Write commits following [Conventional Commits](https://www.conventionalcommits.org/):

```bash
git commit -m "feat(auth): add JWT token validation"
git commit -m "fix(orm): resolve race condition in connection pool"
git commit -m "feat!: change Model trait signature"
```

### Step 2: Push to Main Branch

```bash
git push origin main
```

### Step 3: release-plz Creates Release PR

When changes are pushed to main, release-plz automatically:

1. Analyzes commits since last release
2. Determines version bumps for affected crates
3. Updates `Cargo.toml` versions
4. Generates/updates CHANGELOG.md files
5. Creates a Release PR

**Release PR includes:**
- Version bumps in `Cargo.toml`
- Updated CHANGELOG.md files
- List of changes for each crate

### Step 4: Review and Merge Release PR

1. Review the Release PR
2. Verify version bumps are correct
3. Check CHANGELOG entries
4. Merge when ready

### Step 5: Automatic Publishing

Upon merge, release-plz:

1. Publishes crates to crates.io (in dependency order)
   - Automatically skips already-published versions
   - Handles workspace dependency ordering correctly
2. Creates Git tags (`[crate-name]@v[version]`)
3. Creates GitHub Releases

**Note**: The `release-plz release` command handles publishing gracefully:
- Already-published crate versions are skipped automatically (no errors on retry)
- Only publishable crates are processed (respects `publish = false` in Cargo.toml)
- Workspace dependencies are published in the correct order

---

## Manual Intervention

### Editing Release PR

You can modify the Release PR before merging:

- **Adjust CHANGELOG entries**: Edit for clarity or add details
- **Modify version bumps**: Change version in `Cargo.toml` if needed
- **Add migration notes**: Include breaking change documentation

### Force Version Bump

To force a specific version, manually edit `Cargo.toml` in the Release PR.

### Skip Release

To skip releasing a crate, add to `release-plz.toml`:

```toml
[[package]]
name = "crate-name"
release = false
```

---

## Configuration

### release-plz.toml

Configuration file at repository root:

```toml
[workspace]
changelog_update = true
pr_branch_prefix = "release-plz/"
pr_labels = ["release", "automated"]
git_release_enable = true
git_tag_enable = true
git_tag_name = "{{ package }}@v{{ version }}"
git_release_type = "auto"
semver_check = false
publish_timeout = "10m"
dependencies_update = false

# Exclude packages from release
[[package]]
name = "reinhardt-test-support"
release = false
publish = false

[changelog]
protect_breaking_commits = true
```

### Non-Published Packages

The following packages are excluded from release:

- `reinhardt-test-support` - Test utilities
- `reinhardt-integration-tests` - Integration tests
- `reinhardt-benchmarks` - Benchmark tests
- `examples-*` - Example projects
- `reinhardt-settings-cli` - Internal CLI tool

---

## Troubleshooting

### Common Issues

**No Release PR Created:**
- Verify commits use conventional commit format
- Check that changes affect publishable crates
- Review release-plz workflow logs

**Wrong Version Bump:**
- Ensure commit messages follow conventions
- Use `feat!:` or `BREAKING CHANGE:` for major bumps
- Edit the Release PR to correct version

**Publish Failed:**
- Check `CARGO_REGISTRY_TOKEN` secret is set
- Verify crate metadata is complete
- Review crates.io for existing version conflicts
- **Already Published**: release-plz automatically skips already-published versions, so retry is safe

**CHANGELOG Not Updated:**
- Ensure `changelog_update = true` in config
- Verify commit messages are properly formatted

### Verification Commands

```bash
# Check release-plz config
cat release-plz.toml

# Verify conventional commits
git log --oneline -10

# Check crates.io version
curl -s "https://crates.io/api/v1/crates/<crate-name>" | jq '.crate.max_version'
```

---

## Related Documentation

- **Main Quick Reference**: @CLAUDE.md
- **Commit Guidelines**: @COMMIT_GUIDELINE.md
- **release-plz Documentation**: https://release-plz.ieni.dev/docs
