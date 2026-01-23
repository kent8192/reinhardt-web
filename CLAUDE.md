# CLAUDE.md

## Purpose

This file contains project-specific instructions for the Reinhardt project. These rules ensure code quality, maintainability, and consistent practices across the Rust codebase.

For detailed standards, see documentation in `docs/` directory.

---

## Project Overview

See README.md for project details.

**Repository URL**: https://github.com/kent8192/reinhardt-rs

---

## Tech Stack

- **Language**: Rust 2024 Edition
- **Module System**: MUST use 2024 edition (NO `mod.rs`)
- **Database**: SeaQuery v1.0.0-rc for building SQL queries
- **Testing**: Rust's built-in framework + TestContainers for infrastructure
- **Build**: Cargo workspace with multiple crates

---

## Critical Rules

### Module System
**MUST use `module.rs` + `module/` directory structure (Rust 2024 Edition)**
**NEVER use `mod.rs` files** (deprecated)

See docs/MODULE_SYSTEM.md for comprehensive module system standards including:
- Basic module patterns (small, medium, large)
- Visibility control with `pub use`
- Anti-patterns to avoid
- Migration guide from old style

### Code Style

**Key Requirements:**
- **ALL code comments MUST be written in English** (no exceptions)
- MINIMIZE `.to_string()` calls - prefer borrowing
- DELETE obsolete code immediately
- NO deletion record comments in code
- NO relative paths beyond `../` (use absolute paths)
- Mark ALL placeholders with `todo!()` or `// TODO:` comment
- Document ALL `#[allow(...)]` attributes with explanatory comments (see @docs/ANTI_PATTERNS.md)

**Unimplemented Features Notation:**
- `todo!()` - Features that WILL be implemented
- `unimplemented!()` - Features that WILL NOT be implemented (intentionally omitted)
- `// TODO:` - Planning notes
- **DELETE** `todo!()` and `// TODO:` when implemented
- **KEEP** `unimplemented!()` for permanently excluded features
- **NEVER** use alternative notations (`FIXME:`, `Implementation Note:`, etc.)

See docs/ANTI_PATTERNS.md for comprehensive anti-patterns guide.

### Testing

**Core Principles:**
- NO skeleton tests (all tests MUST have meaningful assertions)
- EVERY test MUST use at least one Reinhardt component
- Unit tests: Test single component behavior, place in functional crate
- Integration tests: Test integration points between components
  - Cross-crate integration: Place in `tests/` crate
  - Within-crate integration: Can place in functional crate
- Functional crates MUST NOT include other Reinhardt crates in `dev-dependencies`
- ALL test artifacts MUST be cleaned up
- Global state tests MUST use `#[serial(group_name)]`
- Use strict assertions (`assert_eq!`) instead of loose matching (`contains`)

See docs/TESTING_STANDARDS.md for comprehensive testing standards including:
- Testing philosophy (TP-1, TP-2)
- Test organization (TO-1, TO-2)
- Test implementation (TI-1 ~ TI-5)
- Infrastructure testing (IT-1)

### File Management

**Critical Rules:**
- **NEVER** save temp files to project directory (use `/tmp`)
- **IMMEDIATELY** delete `/tmp` files when no longer needed
- **IMMEDIATELY** delete backup files (`.bak`, `.backup`, `.old`, `~` suffix)
- NO relative paths beyond one level up (`../..` is forbidden)
- Use absolute paths or single-level relative paths

### Documentation

**Update Requirements:**
- **ALWAYS** update docs when code changes (same workflow)
- Update all relevant: README.md, crate README, docs/, lib.rs
- Planned features go in `lib.rs` header, NOT in README.md
- Test all code examples
- Verify all links are valid
- **ALWAYS** update crate's `CHANGELOG.md` when `Cargo.toml` version changes (same commit)
- **CHANGELOG.md MUST be written in English** (no exceptions)
- **NEVER** document user requests or AI assistant interactions in project documentation
  - Documentation must describe technical reasons, design decisions, and implementation details
  - Avoid phrases like "User requested...", "As requested by...", "User asked..."
  - Focus on the "why" (technical rationale) not the "who asked"

See docs/DOCUMENTATION_STANDARDS.md for comprehensive documentation standards.

### Git Workflow

**Commit Policy:**
- **NEVER** commit without explicit user instruction
- **NEVER** push without explicit user instruction
- **EXCEPTION**: Plan Mode approval is considered explicit commit authorization
  - When user approves a plan via Exit Plan Mode, implementation and commits are both authorized
  - Upon successful implementation, all planned commits are created automatically without additional confirmation
  - If implementation fails or tests fail, NO commits are created (report to user instead)
- Split commits by specific intent (NOT feature-level goals)
- Each commit MUST be small enough to explain in one line
- Use `git apply <patchfile name>.patch` for partial file commits
- **NEVER** execute batch commits without user confirmation

**GitHub Integration:**
- **MUST** prefer GitHub MCP tools when available for all GitHub operations
- **Fallback**: Use GitHub CLI (`gh`) when GitHub MCP is not available
- **Priority order**: GitHub MCP > GitHub CLI (`gh`) > raw API
- **NEVER** use raw `curl` or web browser for GitHub operations when MCP or `gh` is available

**GitHub MCP Tool Mapping:**

| Operation | GitHub MCP Tool | gh CLI Fallback |
|-----------|----------------|-----------------|
| Create PR | `create_pull_request` | `gh pr create` |
| View PR | `pull_request_read` (method: get) | `gh pr view` |
| List PRs | `list_pull_requests` | `gh pr list` |
| Create Issue | `issue_write` (method: create) | `gh issue create` |
| View Issue | `issue_read` (method: get) | `gh issue view` |
| List Issues | `list_issues` | `gh issue list` |
| Search Code | `search_code` | `gh api search/code` |
| Get File | `get_file_contents` | `gh api repos/.../contents` |
| Create Branch | `create_branch` | `gh api refs` |
| List Commits | `list_commits` | `gh api commits` |
| List Releases | `list_releases` | `gh release list` |
- **MUST** write all PR titles and descriptions in English
- **MUST** write all issue titles and descriptions in English

See docs/PR_GUIDELINE.md for detailed pull request guidelines including:
- PR creation policy
- PR title and description format
- PR review process

See docs/COMMIT_GUIDELINE.md for detailed commit guidelines including:
- Commit execution policy (CE-1 ~ CE-5)
- Commit message format (CM-1 ~ CM-3)
- Commit message style guide

### Release & Publishing Policy

**Versioning (Semantic Versioning 2.0.0):**
- **MUST** follow Semantic Versioning 2.0.0 strictly for all crates
  - MAJOR version (X.0.0): Breaking changes (API incompatibility)
  - MINOR version (0.X.0): New features (backward compatible)
  - PATCH version (0.0.X): Bug fixes (backward compatible)
- **NEVER** make breaking changes without incrementing MAJOR version
- Each crate maintains its own independent version
- Pre-1.0.0 versions (0.x.x) may have breaking changes in MINOR versions (per SemVer spec)

**Tagging Strategy (Per-Crate Tagging):**
- **MUST** use format: `[crate-name]@v[version]`
  - Examples: `reinhardt-core@v0.2.0`, `reinhardt-orm@v0.1.1`, `reinhardt@v1.0.0`
- **MUST** tag each crate individually when published to crates.io
- Tag message MUST include brief changelog summary
- Tag MUST be created AFTER committing version changes, not before

**Publishing to crates.io:**
- **NEVER** publish without explicit user authorization
- **ALWAYS** use `--dry-run` first for verification
- Verify all checks pass before publishing:
  - `cargo check --workspace --all --all-features`
  - `cargo test --workspace --all --all-features`
  - `cargo publish --dry-run -p <crate-name>`
- Commit version bump and CHANGELOG updates BEFORE creating tag
- Push commits and tags AFTER successful publish

**Version Cascade Policy:**
- When a sub-crate's version changes, the main crate (`reinhardt-web`) version MUST be updated following the version mapping rules:
  - Single sub-crate update: Main crate version change MUST match sub-crate's change level (MAJOR → MAJOR, MINOR → MINOR, PATCH → PATCH)
  - Multiple sub-crates update: Main crate version follows the highest priority change (MAJOR > MINOR > PATCH)
- The main crate's CHANGELOG.md MUST include a "Sub-Crate Updates" subsection with:
  - Sub-crate name, version, and CHANGELOG link (using anchor format: `#[version]---YYYY-MM-DD`)
  - Brief summary (1-3 bullet points) of key changes
- Each crate version bump MUST be committed individually (sub-crates first, main crate last)
- Main crate commit message MUST include `cascade:` keyword indicating Version Cascade
- See [docs/VERSION_CASCADE.md](docs/VERSION_CASCADE.md) for complete implementation guide

**Publishing Workflow:**
1. Update crate version in `Cargo.toml`
2. Update crate's `CHANGELOG.md`
3. Run all verification commands
4. Commit version changes (see docs/COMMIT_GUIDELINE.md CE-5)
5. Wait for explicit user authorization to proceed
6. Run `cargo publish --dry-run -p <crate-name>`
7. Wait for user confirmation after dry-run
8. Run `cargo publish -p <crate-name>`
9. Create and push tag: `git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"`
10. Push: `git push && git push --tags`

**Why This Approach:**
- **Traceability**: Git tag enables complete restoration of specific crate version state
- **Unambiguous**: Clear identification of which crate at which version (critical for 70+ crates)
- **Efficient**: Release only changed crates, avoid unnecessary dependency updates
- **Automation-friendly**: Compatible with tools like `release-plz`, `cargo-release`

See docs/RELEASE_PROCESS.md for detailed release procedures.

### Workflow Best Practices

- Run dry-run for ALL batch operations before actual execution
- Use parallel agents for independent file edits
- NO batch commits (create one at a time with user confirmation)

---

## Common Commands

**Check & Build:**
```bash
cargo check --workspace --all --all-features
cargo build --workspace --all --all-features
```

**Testing:**
```bash
cargo test --workspace --all --all-features
cargo test --doc  # Documentation tests
```

**Code Quality:**
```bash
cargo make fmt-check   # Cheeck format rules of the code
cargo make clippy-check  # Check lint rules of the code
cargo make fmt-fix   # Automatically fix code based on formatting rules
cargo make clippy-fix  # Automatically fix code based on lint rules
```

**Database Tests:**
```bash
# Database tests use TestContainers automatically (no external database needed)
cargo nextest run --package reinhardt-integration-tests
```

**Container Runtime:**
```bash
# Verify Docker status
docker version
docker ps

# Docker daemon should be running automatically on most systems
```

**GitHub Operations:**

When GitHub MCP is available, use MCP tools directly (preferred).
When unavailable, fall back to GitHub CLI.

**GitHub CLI Fallback:**
```bash
# Pull Requests
gh pr create --title "feat: Add feature" --body "Description" --label enhancement
gh pr view [number]
gh pr list --state open
gh pr checks

# Issues
gh issue create --title "Bug report" --body "Description"
gh issue view [number]
gh issue list

# Releases
gh release list
gh release view [tag]
gh release create [tag] --title "Release v1.0.0" --notes "Release notes"

# Repository
gh repo view
gh api repos/{owner}/{repo}/pulls
```

**CRITICAL: This project uses Docker for TestContainers integration, NOT Podman.**

- **MUST** ensure Docker Desktop is installed and running
- **MUST** ensure `DOCKER_HOST` environment variable points to Docker socket:
  - ✅ Correct: `unix:///var/run/docker.sock` or not set
  - ❌ Incorrect: `unix:///.../podman/...` (will cause container startup failures)
- If both Docker and Podman are installed:
  - Use `.testcontainers.properties` to force Docker usage (already configured in project)
  - Ensure `DOCKER_HOST` is not set to Podman socket
- **NEVER** use Podman for integration tests in this project

**Troubleshooting Container Errors:**

If you encounter "Cannot connect to Docker daemon" or "IncompleteMessage" errors:

```bash
# 1. Check Docker is running
docker ps

# 2. Check DOCKER_HOST environment variable
echo $DOCKER_HOST

# 3. If DOCKER_HOST points to Podman, unset it
unset DOCKER_HOST

# 4. Verify .testcontainers.properties exists in project root
cat .testcontainers.properties
```

---

## Database Operations

**Layer Selection:**
- **Basic CRUD**: Use `reinhardt-orm` for table-level operations
- **Low-Level**: Use `reinhardt-database` for schema management, raw queries, DB-specific operations

---

## Review Process

Before submitting code:

1. **Run all commands:**
   - `cargo check --workspace --all --all-features`
   - `cargo build --workspace --all --all-features`
   - `cargo test --workspace --all --all-features`
   - `cargo make fmt-check`
   - `cargo make clippy-check`

2. **Iterate until all issues resolved**

3. **Review compliance with standards:**
   - [ ] Module system (@docs/MODULE_SYSTEM.md)
   - [ ] Testing standards (@docs/TESTING_STANDARDS.md)
   - [ ] No anti-patterns (@docs/ANTI_PATTERNS.md)
   - [ ] Documentation updated (@docs/DOCUMENTATION_STANDARDS.md)
   - [ ] Git commit policy (@docs/COMMIT_GUIDELINE.md)
   - [ ] PR guidelines (@docs/PR_GUIDELINE.md)

---

## Additional Instructions

@CLAUDE.local.md - Project-specific local preferences

**Note**: When editing files in `examples/` directory, also refer to examples/CLAUDE.md for examples-specific coding standards and dependency rules.

---

## Quick Reference

### ✅ MUST DO
- Write ALL code comments in English (no exceptions)
- Use `module.rs` + `module/` directory (NO `mod.rs`)
- Update docs with code changes (same workflow)
- Clean up ALL test artifacts
- Delete temp files from `/tmp` immediately
- Wait for explicit user instruction before commits
- Understand that Plan Mode approval authorizes both implementation and commits
- Mark placeholders with `todo!()` or `// TODO:`
- Use `#[serial(group_name)]` for global state tests
- Split commits by specific intent, not features
- Follow Conventional Commits v1.0.0 format: `<type>[scope]: <description>`
- Start commit description with lowercase letter (e.g., `feat: add feature`)
- Use `!` notation for breaking changes (e.g., `feat!:` or `feat(scope)!:`)
- Follow Semantic Versioning 2.0.0 strictly for all crates
- Use `[crate-name]@v[version]` format for Git tags
- Verify with `--dry-run` before publishing to crates.io
- Commit version bump before creating tags
- Update crate's CHANGELOG.md with version changes
- Write CHANGELOG.md in English (no exceptions)
- Update main crate (`reinhardt-web`) version when any sub-crate version changes
- Apply Version Cascade Policy: version mapping (MAJOR → MAJOR, MINOR → MINOR, PATCH → PATCH) for single sub-crate updates
- For multiple sub-crates updates, follow highest priority: MAJOR > MINOR > PATCH
- Commit each crate version bump individually (sub-crates first, main crate last)
- Include `cascade:` keyword in main crate commit message for Version Cascade
- Use standardized CHANGELOG reference format: `#[version]---YYYY-MM-DD` for sub-crate links
- Add "Sub-Crate Updates" subsection in main crate CHANGELOG.md with brief summary
- Prefer GitHub MCP tools when available; fall back to `gh` CLI otherwise
- Write all PR titles and descriptions in English
- Write all issue titles and descriptions in English
- Add appropriate labels to every PR (`enhancement`, `bug`, `documentation`, etc.)
- Use `release` label ONLY for version bump PRs (triggers automation)
- Use `rstest` for ALL test cases (no plain `#[test]`)
- Use `reinhardt-test` fixtures for test setup/teardown
- Create specialized fixtures wrapping generic `reinhardt-test` fixtures for test data injection
- Use SeaQuery (not raw SQL) for SQL construction in tests
- Wrap generic types in backticks in doc comments: `` `Result<T>` ``, NOT `Result<T>`
- Wrap macro attributes in backticks: `` `#[inject]` ``, NOT `#[inject]`
- Wrap URLs in angle brackets or backticks: `<https://...>` or `` `https://...` ``
- Specify language for code blocks: ` ```rust `, NOT ` ``` `
- Wrap bracket patterns in backticks: `` `array[0]` ``, NOT `array[0]`
- Use backticks (not intra-doc links) for feature-gated types: `` `FeatureType` ``, NOT `` [`FeatureType`] ``
- Use Mermaid diagrams (via `aquamarine`) for architecture documentation instead of ASCII art
- Ensure `.stderr` files in trybuild tests contain only single error type (no warning/error mixing)

### ❌ NEVER DO
- Use `mod.rs` files (deprecated pattern)
- Commit without user instruction (except Plan Mode approval)
- Leave docs outdated after code changes
- Document user requests or AI interactions in project documentation
- Create PRs without appropriate labels
- Use `release` label for non-version-bump PRs (triggers unintended automation)
- Save files to project directory (use `/tmp`)
- Leave backup files (`.bak`, `.backup`, `.old`, `~`)
- Create skeleton tests (tests without assertions)
- Use loose assertions (`contains`) without justification
- Use glob imports (`use module::*`)
- Create circular dependencies
- Leave unmarked placeholder implementations
- Use `#[allow(...)]` without explanatory comments
- Use alternative TODO notations (`FIXME:`, `NOTE:` for unimplemented features)
- Create batch commits without user confirmation
- Use relative paths beyond `../`
- Publish to crates.io without explicit user authorization
- Create Git tags before committing version changes
- Skip `--dry-run` verification before publishing
- Update sub-crate version without updating main crate version
- Use inappropriate version level in Version Cascade (e.g., MAJOR sub-crate → PATCH main crate)
- Batch multiple crate version bumps into single commit (must commit individually)
- Omit `cascade:` keyword in main crate version bump commit message
- Use non-standard CHANGELOG anchor format for sub-crate links
- Skip "Sub-Crate Updates" subsection in main crate CHANGELOG.md
- Change `Cargo.toml` version without updating corresponding CHANGELOG.md
- Make breaking changes without MAJOR version bump
- Start commit description with uppercase letter
- End commit description with a period
- Omit `!` or `BREAKING CHANGE:` for API-breaking changes
- Use plain `#[test]` instead of `#[rstest]`
- Write raw SQL strings in tests (use SeaQuery instead)
- Duplicate infrastructure setup code (use `reinhardt-test` fixtures)
- Write generic types without backticks in doc comments (causes HTML tag warnings)
- Write macro attributes without backticks in doc comments (causes unresolved link warnings)
- Write bare URLs in doc comments (causes bare URL warnings)
- Use intra-doc links for feature-gated items (causes unresolved link warnings)
- Create new ASCII art diagrams in doc comments (use Mermaid instead)
- Mix warnings and errors in trybuild `.stderr` files

### 📚 Detailed Standards

For comprehensive guidelines, see:
- **Module System**: docs/MODULE_SYSTEM.md
- **Testing**: docs/TESTING_STANDARDS.md
- **Anti-Patterns**: docs/ANTI_PATTERNS.md
- **Documentation**: docs/DOCUMENTATION_STANDARDS.md
- **Git Commits**: docs/COMMIT_GUIDELINE.md
- **Pull Requests**: docs/PR_GUIDELINE.md
- **Release Process**: docs/RELEASE_PROCESS.md
- **Project Overview**: README.md

---

**Note**: This CLAUDE.md focuses on core rules and quick reference. All detailed standards, examples, and comprehensive guides are in the `docs/` directory. Always review CLAUDE.md before starting work, and consult detailed documentation as needed.
