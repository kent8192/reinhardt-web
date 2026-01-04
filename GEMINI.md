# GEMINI.md

## Purpose

This file contains project-specific instructions for the Gemini CLI agent working on the Reinhardt project. These rules are mandatory to ensure code quality, maintainability, and consistent practices across the Rust codebase.

For detailed standards, refer to the documentation in the `docs/` directory.

---

## Project Overview

See README.md for project details. Use the `read_file` tool to review it.

**Repository URL**: https://github.com/kent8192/reinhardt-rs

---

## Tech Stack

- **Language**: Rust 2024 Edition
- **Module System**: MUST use 2024 edition (`module.name/` directory, NO `mod.rs`)
- **Database**: SeaQuery v1.0.0-rc1 for SQL operations
- **Testing**: Rust's built-in framework + TestContainers for infrastructure
- **Build**: Cargo workspace with multiple crates

---

## Critical Rules for Gemini CLI

### Module System
**MUST use `module.rs` + `module/` directory structure (Rust 2024 Edition)**
**NEVER use `mod.rs` files** (deprecated)

To understand the module system, use `read_file` on `docs/MODULE_SYSTEM.md`. It contains comprehensive standards including:
- Basic module patterns (small, medium, large)
- Visibility control with `pub use`
- Anti-patterns to avoid
- Migration guide from the old `mod.rs` style

### Code Style & Modification

**Core Principles:**
- Use `read_file` to analyze existing code before making changes.
- Use `replace` for targeted changes, ensuring sufficient context is provided.
- Use `write_file` for creating new files or significant rewrites.

**Key Requirements:**
- MINIMIZE `.to_string()` calls; prefer borrowing (`&str`).
- DELETE obsolete code immediately. Do not comment it out.
- NO deletion record comments in the code.
- NO relative paths beyond one level up (`../`). Use absolute paths from the project root.
- Mark ALL placeholders with `todo!()` or a `// TODO:` comment.

**Unimplemented Features Notation:**
- `todo!()`: For features that WILL be implemented.
- `unimplemented!()`: For features that are intentionally NOT implemented.
- `// TODO:`: For planning notes and temporary placeholders.
- **DELETE** `todo!()` and `// TODO:` comments once the feature is implemented.
- **KEEP** `unimplemented!()` macros to clearly mark permanently excluded features.
- **NEVER** use alternative notations like `FIXME:`, `NOTE:`, or `Implementation Note:`.

For a comprehensive guide on what to avoid, use `read_file` on `docs/ANTI_PATTERNS.md`.

### Testing

**Core Principles:**
- NO skeleton tests. All tests MUST have meaningful assertions.
- EVERY test MUST use at least one Reinhardt component.
- Unit tests should test a single component's behavior and be located in the same functional crate.
- Integration tests should test integration points between components.
  - Cross-crate integration tests belong in the `tests/` crate.
  - Within-crate integration tests can be placed in the functional crate.
- Functional crates MUST NOT include other Reinhardt crates in `dev-dependencies`.
- ALL test artifacts (e.g., temporary files) MUST be cleaned up.
- Tests that modify global state MUST use the `#[serial(group_name)]` attribute from the `serial_test` crate.
- Use strict assertions (`assert_eq!`) instead of loose matching (`.contains()`) where possible.

For detailed standards, use `read_file` on `docs/TESTING_STANDARDS.md`.

### File Management

**Critical Rules:**
- **NEVER** save temporary files to the project directory. Use the designated temporary directory (e.g., `/tmp` or the one provided by the environment).
- **IMMEDIATELY** delete temporary files when they are no longer needed.
- **IMMEDIATELY** delete backup files (`.bak`, `.backup`, `.old`, `~` suffix).
- NO relative paths navigating more than one level up (`../../` is forbidden). Use absolute paths from the project root or single-level `../` relative paths.

### Documentation

**Update Requirements:**
- **ALWAYS** update documentation in the same workflow as code changes.
- Relevant files include `README.md`, crate-level `README.md`, `docs/*.md`, and `lib.rs` doc comments.
- Document planned features in the `lib.rs` header, NOT in the main `README.md`.
- Test all code examples using `cargo test --doc`.
- Verify all links are valid.
- **NEVER** document user requests or AI assistant interactions in project documentation.
  - Documentation must describe technical reasons, design decisions, and implementation details.
  - Avoid phrases like "User requested...", "As requested by...", or "The user asked for...".
  - Focus on the "why" (technical rationale), not the "who" (the requester).

For detailed standards, use `read_file` on `docs/DOCUMENTATION_STANDARDS.md`.

### Git Workflow

**Commit Policy:**
- **NEVER** commit without explicit user instruction.
- **NEVER** push without explicit user instruction.
- A commit MUST be small enough to be explained in a single line.
- Use `git apply <patchfile_name>.patch` for partial file commits if necessary.
- **NEVER** execute batch commits without user confirmation. Propose one commit at a time.

**GitHub Integration:**
- **MUST** use the GitHub CLI (`gh`) for all GitHub operations.
- Use `gh pr create` for creating pull requests.
- Use `gh issue create` for creating issues.
- **NEVER** use raw `curl` or other methods when `gh` provides a native command.

For detailed commit guidelines, see `docs/COMMIT_GUIDELINE.md`.

### Release & Publishing Policy

**Versioning (Semantic Versioning 2.0.0):**
- **MUST** follow Semantic Versioning 2.0.0 strictly for all crates.
- Each crate maintains its own independent version.

**Tagging Strategy (Per-Crate Tagging):**
- **MUST** use the format: `[crate-name]@v[version]`.
- Examples: `reinhardt-core@v0.2.0`, `reinhardt-utils@v0.1.1`.
- Tag MUST be created AFTER committing version changes.

**Publishing to crates.io:**
- **NEVER** publish without explicit user authorization.
- **ALWAYS** use `--dry-run` for verification before publishing.
- Follow the exact workflow below.

**Publishing Workflow (to be executed step-by-step with `run_shell_command`):**
1.  Update crate version in `Cargo.toml`.
2.  Update the crate's `CHANGELOG.md`.
3.  Run all verification commands (see "Review Process" section).
4.  Commit version changes (see `docs/COMMIT_GUIDELINE.md` CE-5).
5.  **Await explicit user authorization to proceed.**
6.  Run `cargo publish --dry-run -p <crate-name>`.
7.  **Await user confirmation after reviewing the dry-run output.**
8.  Run `cargo publish -p <crate-name>`.
9.  Create and push tag: `git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"`.
10. Push changes: `git push && git push --tags`.

For detailed procedures, use `read_file` on `docs/RELEASE_PROCESS.md`.

---

## Common Commands (for `run_shell_command`)

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
cargo make fmt-check   # Check format rules
cargo make clippy-check  # Check lint rules
cargo make fmt-fix     # Automatically fix formatting
cargo make clippy-fix    # Automatically fix lints
```

**Database Tests:**
```bash
# Database tests use TestContainers automatically (no external database needed)
cargo nextest run --package reinhardt-integration-tests
```

**Container Runtime:**
This project uses **Docker** for TestContainers integration. Podman is not supported.
- Ensure Docker Desktop is installed and running.
- Use `docker ps` to verify.
- The `.testcontainers.properties` file is configured to force Docker usage.
- If you encounter container errors, check that `DOCKER_HOST` is not set to a Podman socket.

**GitHub Operations (using `gh`):**
```bash
# Pull Requests
gh pr create --title "feat: Add feature" --body "Description"
gh pr list
gh pr view [number]
gh pr checks

# Issues
gh issue create --title "Bug report" --body "Description"
gh issue list
gh issue view [number]

# Releases
gh release create [tag] --title "Release v1.0.0" --notes "Release notes"
gh release list
```

---

## Review Process

Before finalizing your work, you MUST:

1.  **Run all checks using `run_shell_command`:**
    *   `cargo check --workspace --all --all-features`
    *   `cargo build --workspace --all --all-features`
    *   `cargo test --workspace --all --all-features`
    *   `cargo make fmt-check`
    *   `cargo make clippy-check`

2.  **Iterate and fix all issues until all commands pass.**

3.  **Confirm compliance with all standards in this document.**

---

## Quick Reference for Gemini CLI

### ✅ MUST DO
- Use `module.rs` + `module/` directory structure.
- Update docs with code changes in the same workflow.
- Clean up ALL test artifacts.
- Delete temporary files immediately.
- Await explicit user instruction before committing.
- Mark placeholders with `todo!()` or `// TODO:`.
- Use `#[serial(group_name)]` for tests modifying global state.
- Split commits by specific intent.
- Follow Semantic Versioning 2.0.0 for all crates.
- Use `[crate-name]@v[version]` format for Git tags.
- Verify with `cargo publish --dry-run` before publishing.
- Use the GitHub CLI (`gh`) for all GitHub operations.

### ❌ NEVER DO
- Use `mod.rs` files.
- Commit or push without explicit user instruction.
- Leave documentation outdated after code changes.
- Document user requests in project documentation.
- Save temporary files in the project directory.
- Leave backup files (`.bak`, `.old`, etc.).
- Create skeleton tests (tests without assertions).
- Use loose assertions like `.contains()` without strong justification.
- Use glob imports (`use module::*`).
- Create circular dependencies.
- Leave placeholder implementations without `todo!()` or `// TODO:`.
- Use alternative notations like `FIXME:` or `NOTE:`.
- Create batch commits without user confirmation.
- Use relative paths beyond `../`.
- Publish to crates.io without explicit user authorization.
- Create Git tags before committing the version bump.
- Make breaking changes without a MAJOR version bump.
