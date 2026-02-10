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

CHANGELOGs are generated from commit messages and categorized into sections based on custom `commit_parsers` in `release-plz.toml`. Each commit type maps to a specific CHANGELOG section:

| Commit Type | CHANGELOG Section |
|-------------|-------------------|
| `feat` | Added |
| `fix` | Fixed |
| `perf` | Performance |
| `refactor` | Changed |
| `docs` | Documentation |
| `revert` | Reverted |
| `deprecated` | Deprecated |
| `security` | Security |
| `chore`, `ci`, `build` | Maintenance |
| `test` | Testing |
| `style` | Styling |

**Example CHANGELOG output:**

```markdown
## [0.2.0] - 2026-01-30

### Added
- feat(auth): add OAuth support ([#123](https://github.com/kent8192/reinhardt-web/issues/123))

### Fixed
- fix(orm): resolve connection pool exhaustion under high concurrency ([#124](https://github.com/kent8192/reinhardt-web/issues/124))

### Performance
- perf(query): optimize batch insert with prepared statements

### Changed
- refactor(core): extract query builder into dedicated module

### Security
- security(auth): patch session fixation vulnerability
```

**Key features:**
- All commit types are categorized (no entries fall into "Other" unless unrecognized)
- GitHub issue/PR references (`#123`) are automatically converted to clickable links
- Breaking changes are always included, even from otherwise-skipped commits
- Commits are sorted in chronological order (oldest first)

For detailed guidelines on writing CHANGELOG-friendly commit messages, see [docs/COMMIT_GUIDELINE.md](COMMIT_GUIDELINE.md) § CG: CHANGELOG Generation Guidelines.

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

Configuration file at repository root. Key CHANGELOG-related settings:

```toml
[workspace]
changelog_update = true
pr_branch_prefix = "release-plz-"
pr_labels = ["release", "automated"]
pr_name = "chore: release"
git_release_enable = true
git_tag_enable = true
git_tag_name = "{{ package }}@v{{ version }}"
git_release_type = "auto"
semver_check = false
publish_timeout = "10m"
dependencies_update = false
release_always = false
publish_no_verify = true

# Exclude packages from release
[[package]]
name = "reinhardt-test-support"
release = false
publish = false

[changelog]
protect_breaking_commits = true
sort_commits = "oldest"
commit_parsers = [
  # Skip automated commits, then map types to sections
  { message = "^chore: release", skip = true },
  { message = "^Merge", skip = true },
  { message = "^feat", group = "Added" },
  { message = "^fix", group = "Fixed" },
  # ... (see release-plz.toml for full list)
]
commit_preprocessors = [
  # Convert #123 to clickable GitHub links
  { pattern = '#(\d+)', replace = "[#${1}](https://github.com/kent8192/reinhardt-web/issues/${1})" },
]
```

**CHANGELOG Customization Settings:**

| Setting | Purpose |
|---------|---------|
| `protect_breaking_commits` | Always include breaking change commits, even if their type would be skipped |
| `sort_commits` | Sort commits chronologically (`"oldest"` = oldest first) |
| `commit_parsers` | Map commit types to CHANGELOG sections; skip automated commits |
| `commit_preprocessors` | Transform commit messages (e.g., auto-link GitHub references) |

See `release-plz.toml` at repository root for the complete configuration.

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
