# CLAUDE.md

## Purpose

This file contains project-specific instructions for the Reinhardt project. These rules ensure code quality, maintainability, and consistent practices across the Rust codebase.

For detailed standards, see documentation in `docs/` directory.

---

## Project Overview

See @README.md for project details.

**Repository URL**: https://github.com/kent8192/reinhardt-rs

---

## Tech Stack

- **Language**: Rust 2024 Edition
- **Module System**: MUST use 2024 edition (NO `mod.rs`)
- **Database**: SeaQuery v1.0.0-rc1 for SQL operations
- **Testing**: Rust's built-in framework + TestContainers for infrastructure
- **Build**: Cargo workspace with multiple crates

---

## Critical Rules

### Module System
**MUST use `module.rs` + `module/` directory structure (Rust 2024 Edition)**
**NEVER use `mod.rs` files** (deprecated)

See @docs/MODULE_SYSTEM.md for comprehensive module system standards including:
- Basic module patterns (small, medium, large)
- Visibility control with `pub use`
- Anti-patterns to avoid
- Migration guide from old style

### Code Style

**Key Requirements:**
- MINIMIZE `.to_string()` calls - prefer borrowing
- DELETE obsolete code immediately
- NO deletion record comments in code
- NO relative paths beyond `../` (use absolute paths)
- Mark ALL placeholders with `todo!()` or `// TODO:` comment

**Unimplemented Features Notation:**
- `todo!()` - Features that WILL be implemented
- `unimplemented!()` - Features that WILL NOT be implemented (intentionally omitted)
- `// TODO:` - Planning notes
- **DELETE** `todo!()` and `// TODO:` when implemented
- **KEEP** `unimplemented!()` for permanently excluded features
- **NEVER** use alternative notations (`FIXME:`, `Implementation Note:`, etc.)

See @docs/ANTI_PATTERNS.md for comprehensive anti-patterns guide.

### Testing

**Core Principles:**
- NO skeleton tests (all tests MUST have meaningful assertions)
- EVERY test MUST use at least one Reinhardt component
- Unit tests (1 crate): Place in functional crate
- Integration tests (2+ crates): Place in `tests/` crate
- Functional crates MUST NOT include other Reinhardt crates in `dev-dependencies`
- ALL test artifacts MUST be cleaned up
- Global state tests MUST use `#[serial(group_name)]`
- Use strict assertions (`assert_eq!`) instead of loose matching (`contains`)

See @docs/TESTING_STANDARDS.md for comprehensive testing standards including:
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

See @docs/DOCUMENTATION_STANDARDS.md for comprehensive documentation standards.

### Git Workflow

**Commit Policy:**
- **NEVER** commit without explicit user instruction
- **NEVER** push without explicit user instruction
- Split commits by specific intent (NOT feature-level goals)
- Each commit MUST be small enough to explain in one line
- Use `git add -e` for partial file commits
- **NEVER** execute batch commits without user confirmation

See @CLAUDE.commit.md for detailed commit guidelines including:
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
- **MUST** use format: `[crate-name]-v[version]`
  - Examples: `reinhardt-core-v0.2.0`, `reinhardt-orm-v0.1.1`, `reinhardt-v1.0.0`
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

**Publishing Workflow:**
1. Update crate version in `Cargo.toml`
2. Update crate's `CHANGELOG.md`
3. Run all verification commands
4. Commit version changes (see @CLAUDE.commit.md CE-5)
5. Wait for explicit user authorization to proceed
6. Run `cargo publish --dry-run -p <crate-name>`
7. Wait for user confirmation after dry-run
8. Run `cargo publish -p <crate-name>`
9. Create and push tag: `git tag [crate-name]-v[version] -m "Release [crate-name] v[version]"`
10. Push: `git push && git push --tags`

**Why This Approach:**
- **Traceability**: Git tag enables complete restoration of specific crate version state
- **Unambiguous**: Clear identification of which crate at which version (critical for 70+ crates)
- **Efficient**: Release only changed crates, avoid unnecessary dependency updates
- **Automation-friendly**: Compatible with tools like `release-plz`, `cargo-release`

See @docs/RELEASE_PROCESS.md for detailed release procedures.

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
trunk fmt   # Format code
trunk lint  # Lint code
```

**Database Tests:**
```bash
TEST_DATABASE_URL=postgres://postgres@localhost:5432/postgres cargo test
```

**Container Runtime (Podman):**
```bash
# Start Podman machine (macOS/Windows)
podman machine start

# Verify Podman status
podman version
podman ps

# Stop Podman machine
podman machine stop
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
   - `trunk fmt`
   - `trunk lint`

2. **Iterate until all issues resolved**

3. **Review compliance with standards:**
   - [ ] Module system (@docs/MODULE_SYSTEM.md)
   - [ ] Testing standards (@docs/TESTING_STANDARDS.md)
   - [ ] No anti-patterns (@docs/ANTI_PATTERNS.md)
   - [ ] Documentation updated (@docs/DOCUMENTATION_STANDARDS.md)
   - [ ] Git commit policy (@CLAUDE.commit.md)

---

## Additional Instructions

@CLAUDE.local.md - Project-specific local preferences

---

## Quick Reference

### ✅ MUST DO
- Use `module.rs` + `module/` directory (NO `mod.rs`)
- Update docs with code changes (same workflow)
- Clean up ALL test artifacts
- Delete temp files from `/tmp` immediately
- Wait for explicit user instruction before commits
- Mark placeholders with `todo!()` or `// TODO:`
- Use `#[serial(group_name)]` for global state tests
- Split commits by specific intent, not features
- Follow Semantic Versioning 2.0.0 strictly for all crates
- Use `[crate-name]-v[version]` format for Git tags
- Verify with `--dry-run` before publishing to crates.io
- Commit version bump before creating tags
- Update crate's CHANGELOG.md with version changes

### ❌ NEVER DO
- Use `mod.rs` files (deprecated pattern)
- Commit without user instruction
- Leave docs outdated after code changes
- Save files to project directory (use `/tmp`)
- Leave backup files (`.bak`, `.backup`, `.old`, `~`)
- Create skeleton tests (tests without assertions)
- Use loose assertions (`contains`) without justification
- Use glob imports (`use module::*`)
- Create circular dependencies
- Leave unmarked placeholder implementations
- Use alternative TODO notations (`FIXME:`, `NOTE:` for unimplemented features)
- Create batch commits without user confirmation
- Use relative paths beyond `../`
- Publish to crates.io without explicit user authorization
- Create Git tags before committing version changes
- Skip `--dry-run` verification before publishing
- Make breaking changes without MAJOR version bump

### 📚 Detailed Standards

For comprehensive guidelines, see:
- **Module System**: @docs/MODULE_SYSTEM.md
- **Testing**: @docs/TESTING_STANDARDS.md
- **Anti-Patterns**: @docs/ANTI_PATTERNS.md
- **Documentation**: @docs/DOCUMENTATION_STANDARDS.md
- **Git Commits**: @CLAUDE.commit.md
- **Release Process**: @docs/RELEASE_PROCESS.md
- **Project Overview**: @README.md

---

**Note**: This CLAUDE.md focuses on core rules and quick reference. All detailed standards, examples, and comprehensive guides are in the `docs/` directory. Always review CLAUDE.md before starting work, and consult detailed documentation as needed.
