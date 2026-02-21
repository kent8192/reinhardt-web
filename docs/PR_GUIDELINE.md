# Pull Request Guidelines

## Purpose

This file defines the pull request (PR) policy for the Reinhardt project. These rules ensure clear communication, proper review process, and consistent PR formatting across the development lifecycle.

---

## Language Requirements

### LR-1 (MUST): English-Only Policy

- **ALL** PR titles MUST be written in English
- **ALL** PR descriptions MUST be written in English
- **ALL** PR comments and discussions MUST be written in English
- This ensures accessibility for international contributors and maintainers

**Rationale:**
- GitHub is an international platform
- English is the lingua franca of software development
- Enables broader collaboration and code review
- Facilitates automated tooling and CI/CD integration

---

## PR Creation Policy

### PC-1 (MUST): Use GitHub MCP or CLI

- **MUST** prefer GitHub MCP tools (`create_pull_request`) for creating pull requests when available
- **Fallback**: Use GitHub CLI (`gh pr create`) when GitHub MCP is not available
- **NEVER** use web browser UI for PR creation when MCP or CLI is available
- MCP and CLI both ensure consistency and can be automated

**Example:**
```bash
gh pr create --title "feat(auth): add JWT token validation" \
  --body "$(cat <<'EOF'
## Summary

- Implement JWT token validation with RS256 algorithm
- Add token expiration checking
- Include unit tests for edge cases

## Test plan

- [x] `cargo test --package reinhardt-auth` passes
- [x] All existing tests pass
- [x] Manual testing with expired tokens

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

### PC-2 (MUST): Branch Naming

- Branch names SHOULD follow the pattern: `<type>/<scope>-<short-description>`
- Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, etc.
- Scope: Module or component name
- Short description: Kebab-case brief summary

**Examples:**
```
feat/auth-jwt-validation
fix/orm-connection-pool-race-condition
refactor/http-middleware-pipeline
docs/api-openapi-spec
test/database-integration-tests
chore/ci-github-actions-update
```

**Exception:** Release branches follow the format `release/<crate>/vX.Y.Z` for compatibility with automated workflows.

### PC-3 (SHOULD): Draft PRs for Work in Progress

- Use draft PRs for incomplete work
- Convert to ready for review when all tests pass
- Draft PRs allow early feedback without formal review requests

**Example:**
```bash
gh pr create --draft --title "feat(auth): add JWT validation (WIP)"
```

### PC-4 (MUST): PR Labels

- **MUST** add appropriate labels to every PR
- Labels help categorize, prioritize, and track PRs
- Use GitHub MCP (`update_pull_request`), GitHub CLI, or web UI to add labels

**Required Labels by PR Type:**

| PR Type | Required Label | Additional Labels |
|---------|---------------|-------------------|
| New feature | `enhancement` | Scope-specific labels |
| Bug fix | `bug` | Severity labels if available |
| Documentation | `documentation` | - |
| Dependency updates | `dependencies` | - |
| Release preparation | `release` | **CRITICAL** - See special notes below |
| Breaking changes | `enhancement` + custom | Consider adding breaking change indicator |

**Common Labels:**

| Label | Usage | When to Apply |
|-------|-------|---------------|
| `enhancement` | New feature or improvement | All feature PRs |
| `bug` | Bug fix | All bug fix PRs |
| `documentation` | Documentation changes | Docs-only or significant doc updates |
| `dependencies` | Dependency updates | Automated or manual dependency version bumps |
| `release` | **Release preparation (SPECIAL)** | **Version bump PRs for crates.io publication** |
| `good first issue` | Beginner-friendly | Simple, well-defined changes |
| `help wanted` | Needs additional input | Complex decisions or blocked PRs |
| `question` | Further information requested | When clarification or discussion is needed |
| `duplicate` | Duplicate PR | When PR duplicates existing work |
| `invalid` | Invalid PR | When PR doesn't meet standards |
| `wontfix` | Will not be merged | When PR is rejected |

**CRITICAL: `release` Label Special Behavior:**

The `release` label has special significance and triggers automated workflows:

1. **GitHub Actions Integration:**
   - PRs with `release` label are automatically processed by release automation
   - Triggers CI/CD pipeline for crates.io publication preparation
   - May trigger additional validation and checks

2. **When to Use:**
   - **ONLY** for PRs that bump crate versions in `Cargo.toml`
   - **ONLY** for PRs that prepare for crates.io publication
   - **NEVER** for regular feature or bug fix PRs

3. **Requirements for `release` Label:**
   - PR title MUST follow format: `chore(release): bump [crate-name] to v[version]`
   - PR MUST include both `Cargo.toml` version update AND `CHANGELOG.md` updates
   - PR MUST be from a branch following pattern: `release/[crate-name]/v[version]`
   - All tests and checks MUST pass before merging

4. **Example Release PR with Label:**
   ```bash
   # Create release branch
   git checkout -b release/reinhardt-core/v0.2.0
   
   # Make version changes
   # ... update Cargo.toml and CHANGELOG.md ...
   
   # Create PR with release label
   gh pr create \
     --title "chore(release): bump reinhardt-core to v0.2.0" \
     --label release \
     --body "$(cat <<'EOF'
   ## Summary
   
   Prepare reinhardt-core for publication to crates.io.
   
   Version Changes:
   - crates/reinhardt-core/Cargo.toml: version 0.1.0 -> 0.2.0
   - crates/reinhardt-core/CHANGELOG.md: Add release notes for v0.2.0
   
   ## Test plan
   
   - [x] All tests pass
   - [x] `cargo publish --dry-run -p reinhardt-core` succeeds
   - [ ] Ready for publication after merge
   
   🤖 Generated with [Claude Code](https://claude.com/claude-code)
   EOF
   )"
   ```

5. **Post-Merge Automation:**
   - After merging PR with `release` label, automated workflows may:
     - Create Git tag automatically
     - Trigger crates.io publication
     - Generate GitHub Release
     - Update documentation
   - **IMPORTANT**: Check repository's `.github/workflows/` for specific automation

6. **DO NOT Use `release` Label For:**
   - ❌ Regular feature additions
   - ❌ Bug fixes
   - ❌ Documentation updates
   - ❌ Refactoring PRs
   - ❌ Any PR that doesn't bump crate version

**Label Application Examples:**

```bash
# Feature PR with label
gh pr create --title "feat(auth): add JWT validation" \
  --label enhancement

# Bug fix PR with label
gh pr create --title "fix(orm): resolve connection leak" \
  --label bug

# Documentation PR with label
gh pr create --title "docs(api): update OpenAPI spec" \
  --label documentation

# Dependency update PR with label
gh pr create --title "chore(deps): bump tokio from 1.0 to 1.1" \
  --label dependencies

# Release PR with label (CRITICAL - special handling)
gh pr create --title "chore(release): bump reinhardt-core to v0.2.0" \
  --label release

# Multiple labels
gh pr create --title "feat(auth): add OAuth support" \
  --label enhancement,help wanted
```

**Adding Labels to Existing PR:**

```bash
# Add single label
gh pr edit <number> --add-label enhancement

# Add multiple labels
gh pr edit <number> --add-label bug,help wanted

# Remove label
gh pr edit <number> --remove-label invalid

# CRITICAL: Add release label (use with caution)
gh pr edit <number> --add-label release
```

**Label Best Practices:**

- Add labels immediately when creating PR
- Update labels as PR status changes
- Use `release` label **ONLY** for version bump PRs (triggers release automation)
- Combine labels to provide more context (e.g., `enhancement` + `help wanted`)
- Don't over-label - typically 1-3 labels per PR is sufficient
- Double-check before adding `release` label - it has special behavior
- If unsure about `release` label, consult with maintainers first

---

## PR Title Format

### TF-1 (MUST): Follow Conventional Commits

PR titles MUST follow the same format as commit messages:

```
<type>[optional scope][optional !]: <description>

Examples:
feat(auth): add JWT token validation with RS256 algorithm
fix(orm): resolve race condition in connection pool
feat(api)!: change response format to JSON:API specification
```

**Requirements:**
- **Type**: One of the defined types (feat, fix, refactor, docs, etc.)
- **Scope**: Module or component name (OPTIONAL but RECOMMENDED)
- **Breaking Change Indicator**: Append `!` for breaking changes
- **Description**: Concise summary in English
  - **MUST** start with lowercase letter
  - **MUST** be specific and descriptive
  - **MUST NOT** end with a period
  - Keep under 72 characters for readability

**See**: @docs/COMMIT_GUIDELINE.md for detailed commit type definitions

---

## PR Description Format

### DF-1 (MUST): Standard Structure

PR descriptions MUST follow this structure:

```markdown
## Summary

- Bullet point 1: Brief description of change
- Bullet point 2: Another change
- Bullet point 3: Additional context

## Test plan

- [ ] Test case 1
- [ ] Test case 2
- [ ] Test case 3
- [ ] Manual testing notes

## Breaking Changes (if applicable)

- Breaking change 1: Migration path
- Breaking change 2: Impact and solution

## Related Issues (if applicable)

Fixes #123
Closes #456
Refs #789

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Requirements:**

1. **Summary Section** (REQUIRED)
   - Use bullet points for clarity
   - List key changes in logical order
   - Be specific about what changed and why
   - Mention new features, bug fixes, or improvements

2. **Test Plan Section** (REQUIRED)
   - List all verification steps
   - Include automated test commands
   - Note manual testing performed
   - Use checkboxes (`- [ ]` or `- [x]`) for tracking

3. **Breaking Changes Section** (REQUIRED for breaking changes)
   - List all API changes that break compatibility
   - Provide migration path for each change
   - Explain impact on existing code

4. **Related Issues Section** (OPTIONAL)
   - Link related issues using GitHub keywords
   - Use `Fixes #123` to auto-close issues
   - Use `Refs #123` for related but not closed issues

5. **Footer** (REQUIRED)
   - Include Claude Code attribution for AI-assisted PRs

### DF-2 (SHOULD): Additional Context

Include additional sections when relevant:

- **Migration Guide**: For breaking changes with complex migration
- **Performance Impact**: For performance-related changes
- **Security Considerations**: For security-related changes
- **Documentation**: Links to updated documentation
- **Screenshots**: For UI changes (use relative paths or URLs)

---

## PR Review Process

### RP-1 (MUST): Pre-Review Checklist

Before requesting review, ensure:

- [ ] All CI checks pass
- [ ] All tests pass locally
- [ ] Code follows project style guidelines
- [ ] Documentation is updated
- [ ] Commit history is clean and logical
- [ ] PR description is complete and accurate

**Commands to run:**
```bash
cargo check --workspace --all --all-features
cargo test --workspace --all --all-features
cargo make fmt-check
cargo make clippy-check
```

### RP-2 (SHOULD): Self-Review

- Review your own PR before requesting review from others
- Check for:
  - Unnecessary debug code or comments
  - Proper error handling
  - Test coverage
  - Documentation completeness
  - Code clarity and readability

### RP-3 (MUST): Address Review Comments

- Respond to all review comments
- Mark conversations as resolved when addressed
- Request re-review after making changes
- Be respectful and constructive in discussions

### RP-4 (SHOULD): Keep PRs Small

- Aim for PRs under 400 lines of changes
- Split large features into multiple PRs
- Each PR should have a single, clear purpose
- Smaller PRs are easier to review and less risky to merge

**For batch issue handling**: See docs/ISSUE_HANDLING.md for work unit principles (WU-1 ~ WU-3) on how to scope PRs when addressing multiple issues.

---

## PR Merge Policy

### MP-1 (MUST): Merge Requirements

A PR can only be merged when:

- All CI checks pass
- All conversations are resolved
- At least one approval from a maintainer (if required by repo settings)
- No merge conflicts with base branch
- All commits follow commit guidelines (@docs/COMMIT_GUIDELINE.md)

### MP-2 (MUST): Merge Strategy

**Squash and Merge** (Default):
- Combine all PR commits into a single commit
- Use PR title as commit message
- Include PR description in commit body
- Use for feature branches with multiple interim commits

**Rebase and Merge**:
- Preserve individual commits
- Use when commits are already well-structured
- Each commit MUST follow commit guidelines
- Prefer for PRs with clean, logical commit history

**Merge Commit** (Avoid):
- Creates additional merge commit
- Only use for merging long-lived branches
- Generally avoid for feature branches

### MP-3 (SHOULD): Delete Branch After Merge

- Delete feature branches after successful merge
- Keeps repository clean
- Use GitHub's automatic branch deletion feature

---

## Special Cases

### Release PRs

For release preparation PRs (version bumps):

**Title Format:**
```
chore(release): bump [crate-name] to v[version]

Example:
chore(release): bump reinhardt-core to v0.2.0
```

**Description Format:**
```markdown
## Summary

Prepare for crate publication to crates.io.

Version Changes:
- crates/[crate-name]/Cargo.toml: version [old-version] -> [new-version]
- crates/[crate-name]/CHANGELOG.md: Add release notes for v[new-version]

## Breaking Changes (if MAJOR version bump)

- List breaking changes here
- API changes that affect backward compatibility

## New Features (if MINOR version bump)

- List new features here
- Enhancements and additions

## Bug Fixes (if PATCH version bump)

- List bug fixes here
- Resolved issues and corrections

## Test plan

- [x] `cargo check -p [crate-name] --all-features`
- [x] `cargo test -p [crate-name] --all-features`
- [x] `cargo publish --dry-run -p [crate-name]`
- [ ] Ready for publish after PR merge

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**See**: @docs/RELEASE_PROCESS.md for detailed release procedures

### Documentation-Only PRs

For documentation changes:

**Title Format:**
```
docs(<scope>): <description>

Example:
docs(api): update OpenAPI specification for v0.2.0
docs(readme): add installation instructions
```

**Description:**
- List all documentation files changed
- Note what information was added/updated/removed
- Include links to rendered documentation if available

---

## Quick Reference

### ✅ MUST DO
- Write all PR content in English
- Use GitHub MCP (`create_pull_request`) or `gh pr create` for creating PRs
- Follow Conventional Commits format for titles
- Include Summary and Test plan sections
- Run all checks before requesting review
- Address all review comments
- Ensure all CI checks pass before merge

### ❌ NEVER DO
- Write PR titles or descriptions in non-English languages
- Create PRs without proper description
- Skip test plan section
- Merge with failing CI checks
- Leave unresolved review comments
- Force push after review has started (unless explicitly requested)

---

## Related Documentation

- **Main Quick Reference**: @CLAUDE.md (see Quick Reference section)
- **Issue Handling Principles**: docs/ISSUE_HANDLING.md
- **Commit Guidelines**: @docs/COMMIT_GUIDELINE.md
- **Release Process**: @docs/RELEASE_PROCESS.md
- **GitHub MCP Tools**: Available when GitHub MCP server is configured
- **GitHub CLI Documentation (fallback)**: https://cli.github.com/manual/
