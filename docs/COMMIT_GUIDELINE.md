# Git Commit Guidelines

## Purpose

This file defines the git commit policy for the Reinhardt project. These rules ensure clear commit history, proper granularity, and consistent commit message formatting across the development lifecycle.

---

## Specification Reference

This document follows [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/).

Key principles from the specification:
- Commit messages MUST be structured as: `<type>[optional scope]: <description>`
- The types `feat` and `fix` correlate with SemVer MINOR and PATCH respectively
- Breaking changes MUST be indicated with `!` after type/scope or via `BREAKING CHANGE:` footer

---

## Commit Execution Policy

### CE-1 (MUST): Explicit User Authorization

- **NEVER** create commits without explicit user instruction
- **NEVER** push commits without explicit user instruction
- Always wait for user confirmation before committing changes
- Prepare changes and inform the user, but let them decide when to commit

**EXCEPTION: Plan Mode Approval**

When a user approves a plan by accepting Exit Plan Mode, this constitutes explicit authorization for both:
1. Implementation of the planned changes
2. Creation of all commits associated with the implementation

**Automatic Commit Workflow after Plan Mode Approval:**

1. **Success Case**: If implementation completes successfully and all tests pass:
   - Automatically create all commits as planned in the approved plan
   - NO additional user confirmation required for each commit
   - Follow commit granularity rules (CE-2) and commit message format (CM-1, CM-2, CM-3)
   - Commits are created sequentially in the logical order defined in the plan

2. **Failure Case**: If implementation fails or tests fail:
   - **DO NOT** create any commits
   - Report the failure to the user with detailed information
   - Wait for user instruction on how to proceed

**Important Notes:**

- Plan Mode approval does NOT authorize pushing commits to remote
- Pushing still requires explicit user instruction
- The approved plan should clearly outline the planned commits (number, scope, messages)
- If the implementation deviates significantly from the plan, seek user confirmation before committing
- Batch commits are still prohibited - commits are created one at a time, but automatically without confirmation

### CE-2 (MUST): Commit Granularity

- Commits **MUST** be split into developer-friendly, understandable units
- **Each commit should represent a specific intent to achieve a goal, NOT the goal itself**
  - ❌ Bad: Committing an entire "authentication feature" in one commit (goal-level)
  - ✅ Good: Separate commits for each building block:
    - "implement password hashing with bcrypt" (specific intent)
    - "add JWT token generation logic" (specific intent)
    - "create session middleware" (specific intent)
- **Each commit MUST be small enough to be explained in a single line**
  - If you can't describe the commit clearly in one line, it's too large and should be split
  - The description should explain the specific intent, not just the feature name
  - **Note**: This refers to the _scope of changes_, not the commit message length
    - The commit message itself can and should be detailed with body and footer
    - What matters is that the commit's _intent_ can be summarized in one line
- **Avoid monolithic commits at feature-level**
  - ❌ Bad Examples:
    - "feat(auth): Implement authentication feature" (too broad, feature-level, uppercase start)
    - "feat(auth): Add JWT, session, and OAuth support" (multiple intents, uppercase start)
    - "feat(api): Add user CRUD endpoints" (multiple endpoints = multiple intents)
  - ✅ Good Examples:
    - "feat(auth): implement bcrypt password hashing for user authentication"
    - "feat(auth): add JWT token generation with RS256 algorithm"
    - "feat(auth): create session storage middleware"
    - "feat(api): add user creation endpoint with validation"
    - "feat(api): add user retrieval endpoint"
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
chore(release): bump [crate-name] to v[version]

Example:
chore(release): bump reinhardt-core to v0.2.0
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
chore(release): bump reinhardt-orm to v0.2.0

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

### CE-5.1 (MUST): Version Cascade Commits

**When Applicable:**

When a sub-crate's version is updated, the main crate (`reinhardt-web`) version MUST also be updated following the Version Cascade Policy (see [docs/VERSION_CASCADE.md](VERSION_CASCADE.md)).

**Commit Order:**

Version Cascade requires **individual commits** in the following order:

1. **Sub-crate commits** (in dependency order, leaf-first)
2. **Main crate commit** (last, indicating cascade)

**Sub-Crate Commit Format:**

Same as CE-5 standard release commit format:

```
chore(release): bump [sub-crate-name] to v[version]

Prepare [sub-crate-name] for publication to crates.io.

Version Changes:
- crates/[sub-crate-name]/Cargo.toml: version [old] -> [new]
- crates/[sub-crate-name]/CHANGELOG.md: Add release notes for v[new]

[List changes as per CE-5]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Main Crate Commit Format (with `cascade:` keyword):**

**Subject Line:**
```
chore(release): bump reinhardt-web to v[version] (cascade: [sub-crate-list])

Examples:
chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-orm)
chore(release): bump reinhardt-web to v0.3.0 (cascade: reinhardt-database, reinhardt-orm, reinhardt-rest)
```

**Body Format:**
```
Version Cascade triggered by:
- [crate-name] v[old] → v[new] ([MAJOR|MINOR|PATCH])
- [crate-name-2] v[old] → v[new] ([MAJOR|MINOR|PATCH])  # If multiple

Version Mapping: [change-level] → [change-level]

Changes:
- [crate-name]: Brief summary of key changes
- [crate-name-2]: Brief summary of key changes  # If multiple

Version Changes:
- Cargo.toml: version [old] -> [new]
- CHANGELOG.md: Add Sub-Crate Updates section for v[new]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Complete Example (Single Sub-Crate Update):**

```
chore(release): bump reinhardt-web to v0.2.0 (cascade: reinhardt-orm)

Version Cascade triggered by:
- reinhardt-orm v0.1.0 → v0.2.0 (MINOR)

Version Mapping: MINOR → MINOR

Changes:
- reinhardt-orm: Added support for complex JOIN queries, fixed connection pool leak

Version Changes:
- Cargo.toml: version 0.1.0 -> 0.2.0
- CHANGELOG.md: Add Sub-Crate Updates section for v0.2.0

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Complete Example (Multiple Sub-Crates Update):**

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

Version Changes:
- Cargo.toml: version 0.2.0 -> 0.3.0
- CHANGELOG.md: Add Sub-Crate Updates section for v0.3.0

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Requirements:**

- ✅ MUST commit each crate version bump individually (sub-crates first, main crate last)
- ✅ MUST include `cascade:` keyword in main crate commit subject
- ✅ MUST list all triggering sub-crates in subject (alphabetical order if multiple)
- ✅ MUST specify version mapping in commit body (e.g., "MINOR → MINOR")
- ✅ MUST include brief summary of sub-crate changes in commit body
- ✅ MUST update main crate's CHANGELOG.md with "Sub-Crate Updates" subsection
- ✅ MUST create all Version Cascade commits in a single PR (atomic PR)
- ✅ MUST use correct CHANGELOG anchor format: `#[version]---YYYY-MM-DD`

**Prohibited Actions:**

- ❌ NEVER batch multiple crate version bumps into a single commit
- ❌ NEVER omit `cascade:` keyword in main crate commit subject
- ❌ NEVER skip version mapping information in commit body
- ❌ NEVER use incorrect version level (e.g., MAJOR sub-crate → PATCH main crate)
- ❌ NEVER create separate PRs for sub-crate and main crate commits
- ❌ NEVER use non-standard CHANGELOG anchor format

**For Detailed Implementation Guide:**

See [docs/VERSION_CASCADE.md](VERSION_CASCADE.md) for:
- Version mapping rules (VCR-1, VCR-2, VCR-3)
- CHANGELOG reference format (CRF-1, CRF-2, CRF-3)
- Complete workflow examples
- Edge case handling

---

## Commit Message Structure

### Format

Commit messages consist of three parts:

1. **Subject line**
2. **Body**
3. **Footer**

### CM-1 (MUST): Subject Line Format

```
<type>[optional scope][optional !]: <description>

Examples:
feat(auth): add password validation with bcrypt
fix(orm): resolve race condition in connection pool
feat(api)!: change response format to JSON:API specification
```

**Requirements:**

- **Type**: One of the defined commit types (see Commit Types section below)
- **Scope**: Module or component name (e.g., `auth`, `orm`, `http`)
  - Multiple scopes separated by commas: `(shortcuts,dispatch)`
  - Scope is OPTIONAL but RECOMMENDED for clarity
- **Breaking Change Indicator**: Append `!` after type/scope to indicate breaking changes
  - Example: `feat!:` or `feat(api)!:`
  - This is the PREFERRED method for indicating breaking changes
- **Description**: Concise summary in English
  - **MUST** start with lowercase letter
  - **MUST** be specific, not vague
  - **MUST NOT** end with a period
  - ❌ Bad: "Improve authentication overall" (Too vague, starts with uppercase)
  - ❌ Bad: "add feature." (Ends with period)
  - ✅ Good: "add RS256 algorithm support to JWT token validation"

### Commit Types

**Required Types (correlate with SemVer):**

| Type | Description | SemVer |
|------|-------------|--------|
| `feat` | A new feature | MINOR |
| `fix` | A bug fix | PATCH |

**Recommended Types:**

| Type | Description |
|------|-------------|
| `build` | Changes affecting build system or external dependencies |
| `chore` | Maintenance tasks (no production code change) |
| `ci` | CI configuration changes |
| `docs` | Documentation only changes |
| `perf` | Performance improvements |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `revert` | Reverts a previous commit |
| `style` | Code style changes (formatting, whitespace) |
| `test` | Adding or modifying tests |

### BREAKING CHANGE

Breaking changes introduce API incompatibility and correlate with SemVer MAJOR version bump.

**Indicating Breaking Changes:**

1. **Preferred: Using `!` notation** (concise and visible)
   ```
   feat!: remove deprecated authentication endpoints
   feat(api)!: change response format to JSON:API specification
   ```

2. **Alternative: Using footer** (allows detailed explanation)
   ```
   feat(auth): migrate to OAuth 2.0

   BREAKING CHANGE: legacy session-based authentication is no longer supported.
   Users must migrate to OAuth 2.0 tokens before upgrading.
   ```

3. **Combined: Both `!` and footer** (for maximum clarity)
   ```
   refactor(db)!: change connection pool implementation

   BREAKING CHANGE: `ConnectionPool::new()` now requires a `Config` parameter.
   Previous: `ConnectionPool::new(url)`
   New: `ConnectionPool::new(url, Config::default())`
   ```

**Requirements:**

- Breaking changes MUST be indicated using `!` or `BREAKING CHANGE:` footer
- When using `!`, additional `BREAKING CHANGE:` footer is OPTIONAL
- `BREAKING CHANGE` MUST be uppercase in footer
- `BREAKING-CHANGE` is synonymous with `BREAKING CHANGE`

### Revert Commits

When reverting a previous commit, use the `revert` type with references to the original commit(s).

**Format:**

```
revert: <original commit subject>

Refs: <commit SHA(s)>
```

**Example:**

```
revert: add experimental caching layer

This reverts the experimental caching implementation that caused
memory leaks under high load conditions.

Refs: a1b2c3d, e4f5g6h
```

**Requirements:**

- Subject SHOULD match the original commit's subject
- Body SHOULD explain why the revert is necessary
- Footer MUST include `Refs:` with the original commit SHA(s)

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

Footers follow the [git trailer convention](https://git-scm.com/docs/git-interpret-trailers).

**Footer Syntax:**

```
<token>: <value>
```

or

```
<token> #<value>
```

**Standard Footers:**

| Token | Description | Example |
|-------|-------------|---------|
| `BREAKING CHANGE` | Indicates breaking API change | `BREAKING CHANGE: remove deprecated method` |
| `Co-Authored-By` | Credit to co-authors | `Co-Authored-By: Name <email>` |
| `Refs` | Reference to issues/commits | `Refs: #123, #456` |
| `Closes` | Closes related issues | `Closes #123` |
| `Fixes` | Fixes related issues | `Fixes #789` |
| `Reviewed-by` | Reviewer credit | `Reviewed-by: Name <email>` |

**Required Footer for Claude Code:**

```

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Requirements:**

- **EXACTLY one blank line** between body and footer section
- Footer tokens MUST use `-` in place of whitespace (except `BREAKING CHANGE`)
- Footer **MUST** include the Claude Code attribution when AI-assisted
- Footer **MUST** include Co-Authored-By line when AI-assisted

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
- ❌ Bad: "improve authentication" (too vague)
- ✅ Good: "add RS256 algorithm support to JWT token validation"

#### SR-2 (SHOULD): Context and Impact

- Explain the purpose of changes when not obvious
- Include impact on existing functionality if significant
- Mention related issues or PRs when applicable

---

## Related Documentation

- **Main Quick Reference**: @CLAUDE.md (see Quick Reference section)
- **Main Standards**: @CLAUDE.md
