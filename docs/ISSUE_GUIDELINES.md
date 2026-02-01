# Issue Guidelines

## Purpose

This file defines the issue policy for the Reinhardt project. These rules ensure clear issue tracking, proper labeling, and consistent issue management.

---

## Language Requirements

### LR-1 (MUST): English-Only Content

**ALL issue titles, descriptions, and comments MUST be written in English.**

- Issue titles MUST be in English
- Issue descriptions MUST be in English
- All comments within issues MUST be in English
- Code examples and error messages may use their original language

**Rationale:** English ensures accessibility for all contributors and maintainers worldwide.

---

## Issue Creation Policy

### IC-1 (MUST): Use GitHub Tools

Issues MUST be created using:
- GitHub Web Interface
- GitHub CLI (`gh issue create`)
- GitHub MCP server

**Example (GitHub CLI):**
```bash
gh issue create --title "Bug: Connection pool leak" --body "Description..."
```

### IC-2 (MUST): Search Before Creating

**ALWAYS** search existing issues before creating a new one:
1. Search open and closed issues
2. Check if the issue has already been reported
3. Review related issues for context

**Example:**
```bash
gh issue list --search "connection pool"
gh issue list --state closed --search "leak"
```

### IC-3 (MUST): Use Issue Templates

Issues MUST be created using the appropriate issue template:
- Bug Report (`1-bug.yml`)
- Feature Request (`2-feature.yml`)
- Documentation (`3-documentation.yml`)
- Question (`4-question.yml`)
- Performance Issue (`5-performance.yml`)
- CI/CD Issue (`6-ci_cd.yml`)
- Security Vulnerability (`7-security.yml`)

**Template Selection:**
| Issue Type | Template | Label Applied |
|------------|----------|---------------|
| Bug report | `1-bug.yml` | `bug` |
| New feature | `2-feature.yml` | `enhancement` |
| Documentation | `3-documentation.yml` | `documentation` |
| Question | `4-question.yml` | `question` |
| Performance | `5-performance.yml` | `performance` |
| CI/CD | `6-ci_cd.yml` | `ci-cd` |
| Security | `7-security.yml` | `security`, `critical` |

---

## Issue Title Format

### IT-1 (MUST): Clear and Descriptive

Issue titles MUST be:
- **Specific**: Clearly describe the problem or request
- **Concise**: Maximum 72 characters for readability
- **Uppercase Start**: Begin with uppercase letter
- **Professional**: Use technical language

**Format Options:**

**Option 1: Type Prefix (Recommended)**
```
[Type]: <brief description>
```

**Option 2: Plain Descriptive**
```
<Brief descriptive title>
```

**Examples:**

| Type | Example Title |
|------|---------------|
| Bug | `Bug: Connection pool leak when using async transactions` |
| Feature | `Feature: Add MySQL database backend support` |
| Performance | `Performance: Slow query execution with large datasets` |
| Documentation | `Docs: Missing migration guide for v0.2.0` |
| CI/CD | `CI: TestContainers integration tests failing on macOS` |
| Security | `Security: SQL injection in user query builder` |
| Question | `Question: How to configure custom connection pool size?` |

**Title Quality:**

- ❌ Bad: "Fix bug" (too vague)
- ❌ Bad: "performance issue" (unclear what)
- ❌ Bad: "add feature" (which feature?)
- ✅ Good: "Bug: Connection pool leak when using async transactions"
- ✅ Good: "Performance: Query execution time increases linearly with dataset size"
- ✅ Good: "Feature: Support for MySQL 8.0 window functions"

---

## Issue Labels

### IL-1 (MUST): Apply Type Labels

**ALL issues MUST have at least one type label:**

| Label | Color | Description |
|-------|-------|-------------|
| `bug` | #d73a4a | Confirmed bug or unexpected behavior |
| `enhancement` | #a2eeef | New feature or improvement request |
| `documentation` | #0075ca | Documentation issues or improvements |
| `question` | #d876e3 | Questions about usage or implementation |
| `performance` | #fbca04 | Performance-related issues |
| `ci-cd` | #2cbe4e | CI/CD workflow issues |
| `security` | #ee0701 | Security vulnerabilities or concerns |

### IL-2 (SHOULD): Apply Priority and Scope Labels

**Priority Labels:**

| Label | Color | Description |
|-------|-------|-------------|
| `critical` | #b60205 | Blocks release or major functionality |
| `high` | #d93f0b | Important fix or feature |
| `medium` | #fbca04 | Normal priority |
| `low` | #0e8a16 | Minor fix or enhancement |

**Priority Assignment:**
- **Critical**: Security vulnerabilities, data loss, crashes in production
- **High**: Major functionality broken, significant performance degradation
- **Medium**: Normal bugs, feature requests, documentation improvements
- **Low**: Minor issues, nice-to-have features, cosmetic fixes

**Scope Labels:**

| Label | Color | Description |
|-------|-------|-------------|
| `database` | #ededed | Database layer, schema, migrations |
| `auth` | #ededed | Authentication, authorization, sessions |
| `orm` | #ededed | ORM layer, models, query builder |
| `http` | #ededed | HTTP layer, handlers, middleware |
| `routing` | #ededed | URL routing, path matching |
| `api` | #ededed | REST API, serializers, views |
| `admin` | #ededed | Admin interface, admin panels |
| `graphql` | #ededed | GraphQL schema, resolvers |
| `websockets` | #ededed | WebSocket connections, handlers |
| `i18n` | #ededed | Internationalization, localization |

**Status Labels:**

| Label | Color | Description |
|-------|-------|-------------|
| `good first issue` | #7057ff | Suitable for new contributors |
| `help wanted` | #008672 | Community contributions welcome |
| `duplicate` | #cfd3d7 | Duplicate of another issue |
| `invalid` | #e4e669 | Not a valid issue |
| `wontfix` | #ffffff | Will not be fixed (intentional) |
| `needs more info` | #fef2c0 | Awaiting additional information |

**Label Combinations:**
- Minimum: One type label (`bug`, `enhancement`, etc.)
- Recommended: Type + Priority + Scope
- Example: `bug`, `high`, `database` for a critical database bug

---

## Issue Lifecycle

### LC-1 (MUST): Triage Process

**New Issues:**

1. **Automatic Labeling**: Issue template applies type label
2. **Maintainer Review**: Triage within 48 hours
3. **Label Enhancement**: Add priority and scope labels
4. **Assignment**: Assign to maintainer or contributor

**Issue States:**

| State | Description | Labels |
|-------|-------------|--------|
| Open | New issue awaiting triage | Type label only |
| Triaged | Reviewed and labeled | Type + Priority + Scope |
| In Progress | Being actively worked | Add `in-progress` via project board |
| Blocked | Awaiting dependency | Add `blocked` via project board |
| Closed | Resolved | - |

### LC-2 (MUST): Issue Hygiene

**Issue Closure:**

- **Fixed**: Close with comment describing fix and referencing PR/commit
- **Duplicate**: Close with reference to original issue
- **Wontfix**: Close with explanation of why it won't be fixed
- **Invalid**: Close with explanation

**Comment Requirements:**
- Provide context for status changes
- Reference related issues or PRs
- Explain closure reasoning

**Stale Issues:**
- Issues inactive for 90 days marked as stale
- 30-day grace period for response
- Closed after grace period if no response

---

## Issue Types and Templates

### Bug Report (`1-bug.yml`)

**Use When:**
- Unexpected behavior or error
- Crash or panic
- Incorrect output or result

**Required Information:**
- Rust version
- Operating system
- Minimal reproduction code
- Expected vs actual behavior
- Error messages or stack traces

**Label Applied:** `bug`

### Feature Request (`2-feature.yml`)

**Use When:**
- Requesting new functionality
- Suggesting API improvements
- Proposing new features

**Required Information:**
- Problem statement (why is this needed?)
- Proposed solution
- Alternative approaches considered
- Impact on existing functionality

**Label Applied:** `enhancement`

### Documentation (`3-documentation.yml`)

**Use When:**
- Documentation is missing or unclear
- Examples are needed
- API documentation is incomplete

**Required Information:**
- Which documentation is affected?
- What is missing or unclear?
- Suggested improvement

**Label Applied:** `documentation`

### Question (`4-question.yml`)

**Use When:**
- Asking how to use a feature
- Clarifying API behavior
- General usage questions

**Note:** For usage questions, consider GitHub Discussions first.

**Label Applied:** `question`

### Performance Issue (`5-performance.yml`)

**Use When:**
- Slow query execution
- High memory usage
- Performance regression

**Required Information:**
- Performance problem description
- Steps to reproduce
- Expected vs actual performance
- Benchmark results (if available)
- Environment details

**Label Applied:** `performance`

### CI/CD Issue (`6-ci_cd.yml`)

**Use When:**
- CI workflow failures
- CD deployment issues
- Test infrastructure problems

**Required Information:**
- Affected workflow name
- Issue description
- Error logs
- Environment details

**Label Applied:** `ci-cd`

---

## Security Issues

### SEC-1 (MUST): Private Disclosure

**Security vulnerabilities MUST be reported privately:**

1. **DO NOT** create public issues for security vulnerabilities
2. **DO** use GitHub Security Advisories for private reporting
3. **DO** include reproduction steps and impact assessment

**How to Report:**

Via GitHub Security Advisories (Recommended):
```
https://github.com/kent8192/reinhardt-web/security/advisories
```

**What to Include:**
- Vulnerability description
- Affected versions
- Steps to reproduce
- Proof of concept (if applicable)
- Impact assessment
- Proposed mitigation

**Response Timeline:**
- **48 hours**: Initial confirmation and acknowledgment
- **7 days**: Assessment and severity classification
- **30 days**: Patch release for critical/high severity
- **90 days**: Coordinated disclosure timeline

**After Disclosure:**
- Issue will be created as PRIVATE
- Applied labels: `security`, `critical` (automatically)
- Maintainers will work on patch privately
- Public disclosure after fix is released

---

## Quick Reference

### ✅ MUST DO

- Write ALL issue content in English (no exceptions)
- Search existing issues before creating new ones
- Use appropriate issue templates for ALL issues
- Apply at least one type label to every issue
- Report security vulnerabilities privately via GitHub Security Advisories
- Provide minimal reproduction code for bug reports
- Include environment details (Rust version, OS)
- Be specific in issue titles (max 72 characters)

### ❌ NEVER DO

- Create public issues for security vulnerabilities
- Create duplicate issues without searching first
- Skip issue templates when creating issues
- Use non-English in issue titles or descriptions
- Create issues without appropriate labels
- Apply `release` label to issues (only for PRs)
- Submit bug reports without reproduction steps
- Leave issues inactive without response

---

## Related Documentation

- **Pull Request Guidelines**: docs/PR_GUIDELINE.md
- **Commit Guidelines**: docs/COMMIT_GUIDELINE.md
- **Contributing Guide**: CONTRIBUTING.md
- **Security Policy**: SECURITY.md
- **Code of Conduct**: CODE_OF_CONDUCT.md
- **Label Definitions**: .github/labels.yml

---

**Note**: This document focuses on issue creation and management. For pull request guidelines, see docs/PR_GUIDELINE.md.
