# Git Commit Guidelines

## Purpose

This file defines the git commit policy for the Reinhardt project. These rules ensure clear commit history, proper granularity, and consistent commit message formatting across the development lifecycle.

---

## Commit Execution Policy

### CE-1 (MUST): Explicit User Authorization

- **NEVER** create commits without explicit user instruction
- **NEVER** push commits without explicit user instruction
- Always wait for user confirmation before committing changes
- Prepare changes and inform the user, but let them decide when to commit

### CE-2 (MUST): Commit Granularity

- Commits **MUST** be split into developer-friendly, understandable units
- **Each commit should represent a specific intent to achieve a goal, NOT the goal itself**
  - ❌ Bad: Committing an entire "authentication feature" in one commit (goal-level)
  - ✅ Good: Separate commits for each building block:
    - "Implement password hashing with bcrypt" (specific intent)
    - "Add JWT token generation logic" (specific intent)
    - "Create session middleware" (specific intent)
- **Each commit MUST be small enough to be explained in a single line**
  - If you can't describe the commit clearly in one line, it's too large and should be split
  - The description should explain the specific intent, not just the feature name
  - **Note**: This refers to the _scope of changes_, not the commit message length
    - The commit message itself can and should be detailed with body and footer
    - What matters is that the commit's _intent_ can be summarized in one line
- **Avoid monolithic commits at feature-level**
  - ❌ Bad Examples:
    - "feat(auth): Implement authentication feature" (too broad, feature-level)
    - "feat(auth): Add JWT, session, and OAuth support" (multiple intents)
    - "feat(api): Add user CRUD endpoints" (multiple endpoints = multiple intents)
  - ✅ Good Examples:
    - "feat(auth): Implement bcrypt password hashing for user authentication"
    - "feat(auth): Add JWT token generation with RS256 algorithm"
    - "feat(auth): Create session storage middleware"
    - "feat(api): Add user creation endpoint with validation"
    - "feat(api): Add user retrieval endpoint"
- Group files by specific intent at a fine-grained level
- Ensure commits tell a clear story when reviewing history, with each step being a concrete implementation detail

### CE-3 (MUST): Partial File Commits

When a single file contains changes with different purposes, use one of the following methods for line-level commits:

#### Method 1: Editor-based Patch Editing (Recommended)

```bash
git add --patch <file>
```

- Opens patch directly in editor without interactive prompts
- Edit patch file: delete `+` lines to unstage additions, change `-` to ` ` (space) to unstage deletions
- Provides fine-grained control with single editor operation

#### Method 2: Patch File Approach (For Automation)

```bash
# Generate patch
git diff <file> > /tmp/changes.patch

# Edit patch file to keep only desired changes

# Apply to staging area
git apply --cached /tmp/changes.patch
```

- Fully non-interactive and scriptable
- Allows review and modification before staging
- Ideal for automated workflows

**General Guidelines:**

- Stage only related changes together
- Keep each commit focused on one objective

### CE-4 (MUST): Respect .gitignore

- **NEVER** commit files specified in .gitignore
- Verify staged files before committing
- Use `git status` to confirm no ignored files are included

### CE-5 (MUST): Release Commits

**Version Bump Commits:**

When preparing for crate publication to crates.io:

**Subject Line Format:**
```
chore(release): Bump [crate-name] to v[version]

Example:
chore(release): Bump reinhardt-core to v0.2.0
```

**Body Format:**
```
Prepare for crate publication to crates.io.

Version Changes:
- crates/[crate-name]/Cargo.toml: version 0.1.0 -> [new-version]
- crates/[crate-name]/CHANGELOG.md: Add release notes for v[new-version]

Breaking Changes: (if MAJOR version bump)
- List breaking changes here
- API changes that affect backward compatibility

New Features: (if MINOR version bump)
- List new features here
- Enhancements and additions

Bug Fixes: (if PATCH version bump)
- List bug fixes here
- Resolved issues and corrections

[Standard footer with Claude Code attribution]
```

**Requirements:**
- Version bump commits MUST be separate from feature/fix commits
- MUST update both `Cargo.toml` version AND `CHANGELOG.md` in the same commit
- MUST list all significant changes in the commit body
- Breaking changes MUST be clearly identified for MAJOR version bumps
- Git tag MUST be created AFTER commit, not before
- Commit message MUST follow standard format (CM-1, CM-2, CM-3)

**Example Complete Commit:**

```
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
```

**Post-Commit Actions (Performed AFTER Commit):**

After committing the version bump:

1. Wait for explicit user authorization to proceed with publishing
2. Run verification: `cargo publish --dry-run -p [crate-name]`
3. Wait for user confirmation after dry-run results
4. Publish: `cargo publish -p [crate-name]`
5. Create Git tag: `git tag [crate-name]@v[version] -m "Release [crate-name] v[version]"`
6. Push commits and tags: `git push && git push --tags`

**Critical Rules:**
- ❌ NEVER create tag before committing version changes
- ❌ NEVER publish without explicit user authorization
- ❌ NEVER skip `--dry-run` verification
- ✅ ALWAYS commit version bump first, then tag
- ✅ ALWAYS wait for user confirmation between steps
- ✅ ALWAYS update CHANGELOG.md in the same commit as Cargo.toml

---

## Commit Message Structure

### Format

Commit messages consist of three parts:

1. **Subject line**
2. **Body**
3. **Footer**

### CM-1 (MUST): Subject Line Format

```
type(scope): Brief description in English

Example:
feat(shortcuts,dispatch): Implement Django-style shortcuts and event dispatch system
```

**Requirements:**

- **Type**: One of: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `style`
- **Scope**: Module or component name (e.g., `shortcuts`, `orm`, `http`)
  - Multiple scopes separated by commas: `(shortcuts,dispatch)`
- **Description**: Concise summary in English
  - **MUST** be specific, not vague
  - ❌ Bad: "Improve authentication overall" (Too vague)
  - ✅ Good: "Add RS256 algorithm support to JWT token validation"

### CM-2 (MUST): Body Format

```
Brief summary paragraph explaining the changes.

Module/Component Section 1:
- file/path.rs: +XXX lines - Description
  - Sub-detail 1
  - Sub-detail 2
- file/path2.rs: Description

Module/Component Section 2:
- file/path.rs: Changes
- Removed: old_file.rs (reason)

Features:
- Feature 1
- Feature 2
- Feature 3

Additional context or explanation.
```

**Requirements:**

- Write in English
- Organize changes by module or component
- List modified files with line count changes where significant
- Include "Removed:" entries for deleted files with reasons
- Summarize new features in a dedicated "Features:" section
- Provide context for complex changes

### CM-3 (MUST): Footer Format

```

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Requirements:**

- **EXACTLY one blank line** between body and footer
- Footer **MUST** include the Claude Code attribution
- Footer **MUST** include Co-Authored-By line

---

## Commit Message Style Guide

### Style Reference

- **ALWAYS** examine recent commit messages before writing new ones:
  ```bash
  git log --pretty=format:"%s%n%b" -10
  ```
- Match the style, tone, and structure of existing commits
- Maintain consistency across the project history

### Specificity Requirements

#### SR-1 (MUST): Concrete Descriptions

- Be specific about what changed and why
- ❌ Bad: "Improve authentication"
- ✅ Good: "Add RS256 algorithm support to JWT token validation"

#### SR-2 (SHOULD): Context and Impact

- Explain the purpose of changes when not obvious
- Include impact on existing functionality if significant
- Mention related issues or PRs when applicable

---

## Related Documentation

- **Main Quick Reference**: @CLAUDE.md (see Quick Reference section)
- **Main Standards**: @CLAUDE.md
