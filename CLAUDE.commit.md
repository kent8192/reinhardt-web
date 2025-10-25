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
git add -e <file>
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

### CE-5 (MUST): No Batch Commits

- **NEVER** execute multiple commits in a batch operation
- **ALWAYS** create commits one at a time, with user confirmation between each
- After creating a commit, wait for user instruction before proceeding with the next commit
- ❌ Bad: Creating 5 commits in sequence without user interaction
- ✅ Good: Create one commit, inform user, wait for next instruction, then proceed
- This ensures user has control and visibility over commit history

---

## Commit Message Structure

### Format

Commit messages consist of three parts:

1. **Subject line** (タイトル行)
2. **Body** (本文)
3. **Footer** (フッター)

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

## Quick Reference

**Critical Rules Summary:**

### Execution

- ❌ NO commits without explicit user instruction
- ❌ NO pushing without explicit user instruction
- ❌ NO committing .gitignore files
- ❌ NO batch commits (multiple commits without user confirmation)
- ❌ NO monolithic feature-level commits (e.g., "Implement authentication feature")
- ❌ NO combining multiple intents in one commit (e.g., "Add JWT and OAuth support")
- ✅ SPLIT commits by specific intent, not feature-level goals
- ✅ KEEP each commit small enough to explain in one line
- ✅ FOCUS on concrete implementation details, not broad feature names
- ✅ USE `git add -e` or patch files for partial file commits
- ✅ CREATE one commit at a time with user confirmation between each

### Message Format

- ❌ NO vague descriptions
- ❌ NO missing footer attribution
- ✅ USE `type(scope): description` format
- ✅ WRITE in English (body and subject)
- ✅ INCLUDE file changes with line counts
- ✅ ADD Features section for new capabilities
- ✅ MAINTAIN one blank line before footer

### Style

- ✅ EXAMINE recent commits for style reference
- ✅ BE SPECIFIC about changes
- ✅ ORGANIZE by module/component
- ✅ LIST removed files with reasons
