# GitHub Interaction Guidelines

## Purpose

This file defines the policy for coding agents to participate in GitHub discussions on existing pull requests and issues in the Reinhardt project. These rules ensure appropriate authorization, consistent formatting, and useful technical context when commenting on PRs and Issues.

---

## Language Requirements

### LR-1 (MUST): English-Only Comments

- **ALL** comments on PRs and Issues MUST be written in English
- Code references, file paths, and technical terms should use their original form
- This ensures accessibility for international contributors and maintainers

**Rationale:**
- Consistent with LR-1 in PR_GUIDELINE.md and ISSUE_GUIDELINES.md
- GitHub is an international platform
- English is the lingua franca of software development

---

## Posting Policy

### PP-1 (MUST): Posting Authorization Flow

The agent MUST follow this authorization model before posting any comment:

| Authorization Source | Action |
|---------------------|--------|
| Explicit user instruction | Post directly |
| Plan Mode approval | Post directly |
| Self-initiated (no instruction) | MUST preview and get user confirmation |

**Scope:** COMMIT_GUIDELINE.md CE-1 authorizes certain Git and creation operations.
Comment, reply, and review permission is separate. An explicit request to address
and reply to feedback covers the requested replies; do not ask again. A plan
covers only the interactions it actually approves.

**Self-Initiated Comment Flow:**

1. Draft the comment content
2. Present the full preview to the user
3. Wait for explicit approval
4. Post only after confirmation

**Important Notes:**
- Commit/push permission alone does not authorize comments
- "Post directly" still means using proper tools (PP-3), not bypassing quality standards
- Check the current task and prior authorization before asking. If permission is
  still missing, prepare the exact comment and target before requesting it.

The following diagram summarizes the comment authorization decision flow:

```mermaid
flowchart TD
    A[Want to post GitHub comment] --> B{Explicit user instruction?}
    B -->|Yes| C[Post comment with attribution footer]
    B -->|No| D{Plan Mode approved?}
    D -->|Yes| C
    D -->|No| E[Self-initiated comment]
    E --> F[Draft comment]
    F --> G[Preview to user]
    G --> H{User approved?}
    H -->|Yes| C
    H -->|No| I[Discard or modify draft]
```

### PP-2 (MUST): Content Preview Before Posting

Before posting any self-initiated comment:

1. Show the complete comment text to the user
2. Identify the target (PR number, Issue number, review thread)
3. Explain why this comment would be helpful
4. Wait for explicit approval or modification request

**Example Preview Format:**
```
Target: PR #42 - Review comment reply
Thread: src/auth/jwt.rs line 15

---
[Comment content here]
---

Shall I post this comment?
```

### PP-3 (MUST): GitHub Tool Selection

Prefer a callable GitHub MCP capability that supports the operation. If it is
unavailable, lacks the needed operation or pagination, or returns an error,
use `gh` immediately; do not retry the failed integration. An explicit task
instruction to use `gh` is sufficient to select it directly. Use `gh api` when
higher-level commands do not expose the required endpoint.

Use authenticated tools or `gh` for GitHub operations, never raw `curl` or a
browser. Verify actual tool names and schemas rather than assuming a historical
MCP method is installed. For multiline content, write a task-owned temporary
file and use `--body-file` or a structured tool argument. Remove the temporary
file after posting.

```bash
gh pr comment <number> --body-file /tmp/review-reply.md
gh issue comment <number> --body-file /tmp/issue-comment.md
gh pr review <number> --comment --body-file /tmp/review.md
```

---

## PR Review Response

### RR-1 (MUST): Responding to Review Comments

When responding to PR review comments:

1. Address the specific concern raised by the reviewer
2. Reference the exact code location being discussed
3. Provide technical justification for implementation decisions
4. Offer alternatives when the reviewer suggests changes

**Authorization:** Responding to review comments typically requires explicit user instruction, unless the response was part of an approved plan.

### RR-2 (SHOULD): Response Content Structure

Use this template for PR review responses:

```markdown
**Re: [Reviewer's concern summary]**

[Direct answer to the concern]

[Technical justification or explanation]

[Code reference if applicable]:
`path/to/file.rs:L42` - [Description of relevant code]

[Action taken or proposed]:
- [What was changed, or what will be changed]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Guidelines:**
- Be concise — answer the concern directly
- Include code references with repository-relative paths
- If changes were made in response, reference the commit
- If proposing alternatives, list trade-offs

### RR-3 (MUST): Code Reference Format

When referencing code in GitHub comments:

- **MUST** use repository-relative paths: `crates/reinhardt-core/src/lib.rs`
- **MUST** include line numbers when referring to specific code: `crates/reinhardt-core/src/lib.rs:L42`
- **MUST** use markdown code blocks with language specifiers for code snippets
- **NEVER** use absolute local paths (`/Users/...`, `/home/...`)

**Examples:**
```markdown
✅ Good: See `crates/reinhardt-orm/src/query/builder.rs:L150`
✅ Good: The implementation in `crates/reinhardt-core/src/model.rs:L42-L58`
❌ Bad: See `/Users/kent8192/Projects/reinhardt/crates/reinhardt-orm/src/query/builder.rs`
❌ Bad: Check line 150 (no file reference)
```

---

## PR Implementation Context

### PIC-1 (SHOULD): Providing Change Context for Reviewers

When providing implementation context on PRs, include:

```markdown
## Implementation Context

**Approach:** [Brief description of the approach taken]

**Key Changes:**
- `path/to/file.rs` - [What was changed and why]
- `path/to/other.rs` - [What was changed and why]

**Design Decisions:**
- [Decision 1]: [Rationale]
- [Decision 2]: [Rationale]

**Testing:**
- [What was tested and how]
- [Edge cases covered]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

### PIC-2 (SHOULD): Impact Analysis Comments

When changes affect multiple crates or modules, provide impact analysis:

```markdown
## Impact Analysis

**Changed Crates:**
| Crate | Change Type | Impact |
|-------|------------|--------|
| `reinhardt-core` | API addition | Non-breaking |
| `reinhardt-orm` | Behavior change | Breaking (see migration) |

**Cross-Crate Dependencies:**
- `reinhardt-orm` depends on `reinhardt-core` — changes are forward-compatible
- `reinhardt-database` is not affected

**Migration Required:** [Yes/No]
- [Migration steps if applicable]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

---

## Copilot Review Handling

### CR-1 (MUST): Authorized Review Workflow

Process requested reviewers and feedback under PP-1. PR creation alone does not
authorize replies, reviews, or thread resolution; an approved workflow covers
only its stated interactions. Validate actionable concerns in the current agent
and preserve unrelated changes.

```mermaid
flowchart TD
    A[Collect complete review inventory] --> B{Unprocessed in-scope feedback?}
    B -->|No| C[Report current result]
    B -->|Yes| D[Evaluate concerns and fix where needed]
    D --> E[Run applicable local checks]
    E --> F[Commit and push under CE-1]
    F --> G[Verify local, upstream, remote, and PR heads]
    G --> H[Reply with evidence under PP-1]
    H --> I[Resolve addressed threads]
    I --> J[Fetch complete inventory again]
    J --> B
```

A false positive or an already-delivered fix needs an evidence-backed reply and
resolution when authorized; it does not need an empty commit. If publication or
reply permission is absent, finish independent local work and report that exact
remaining action.

### CR-2 (MUST): Complete Review Inventory

Resolve the repository and PR from current task context. For this repository the
GitHub identity is `kent8192/reinhardt-web`. Prefer the selected review skill's
inventory helper when available, or use the callable GitHub tools/`gh api`.

Fetch all pages of:

- Review threads, including resolved state and stable GraphQL thread IDs.
- Comments in every thread, including author identity and code locations.
- Review bodies and PR conversation comments within the requested feedback scope.

Follow `pageInfo.hasNextPage` / `endCursor` for each GraphQL connection, including
nested comments. REST lists need their pagination too. A `first: 100` query or a
default CLI page is not evidence that the inventory is complete.

Classify authors by verified identity, not a loose substring. Preserve human or
unknown-author feedback unless it is explicitly in scope. Track actionable
body-level findings as well as inline threads.

Fetch a fresh complete inventory after every push and again before closeout;
include new in-scope findings. If no review exists, report that observed state.
Use an available persistent monitor only when future monitoring is requested;
avoid repeated polling of an unchanged PR.

### CR-3 (MUST): Evaluating and Responding to Comments

Evaluate each Copilot comment against these categories:

| Category | Action | Response |
|----------|--------|----------|
| Valid concern | Fix, validate, push, and verify delivery | Reply with fix evidence → Resolve |
| False positive | No code change | Reply with technical explanation → Resolve |
| Already addressed | No code change | Reply with reference to existing handling → Resolve |

**Response Template (extends RR-2):**

For valid concerns with code fix:
```markdown
**Fixed:** [Brief description of the fix]

[Technical explanation of the change]

Commit: [commit hash] — `path/to/file.rs:L42`

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

For false positives or already addressed:
```markdown
**Re: [Copilot's concern summary]**

[Technical explanation of why this is not an issue or is already handled]

Reference: `path/to/file.rs:L42` — [Description of existing handling]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Guidelines:**
- Follow RR-3 for code reference format (repository-relative paths)
- Follow FF-1 for the actual agent's attribution footer
- Follow CG-2 content restrictions (no absolute paths, no user request details)
- Every thread MUST receive a reply before being resolved (no silent resolves)

### CR-4 (MUST): Resolving Threads via GraphQL

**Prerequisite:** Follow PP-1 authorization. For code fixes, verify that local
HEAD, upstream, remote branch, and PR head contain the fix before replying or
resolving. Use the GraphQL thread ID from the complete inventory, not a numeric
comment/database ID.

**Step 1: Reply to the thread**

```bash
gh api graphql -f query='
mutation($threadId: ID!, $body: String!) {
  addPullRequestReviewThreadReply(input: {
    pullRequestReviewThreadId: $threadId,
    body: $body
  }) {
    comment {
      id
    }
  }
}' -f threadId='<THREAD_ID>' -f body='<REPLY_BODY>'
```

**Step 2: Resolve the thread**

```bash
gh api graphql -f query='
mutation($threadId: ID!) {
  resolveReviewThread(input: {
    threadId: $threadId
  }) {
    thread {
      isResolved
    }
  }
}' -f threadId='<THREAD_ID>'
```

**Rules:**
- **MUST** reply before resolving (CR-3 compliance)
- **NEVER** resolve a thread without posting a reply first
- Verify `isResolved: true` in the mutation response
- Re-fetch the complete inventory before claiming that all actionable feedback is resolved

### CR-5 (SHOULD): Completion Summary

After a fresh complete inventory, report selected, addressed, and remaining
feedback counts, any unresolved body-level findings, and the verified PR head.
Report current CI separately. The summary may include:

**Summary Format:**

```markdown
## Copilot Review Handling Summary

| # | File | Line | Category | Action |
|---|------|------|----------|--------|
| 1 | `path/to/file.rs` | L42 | Valid concern | Fixed (commit abc1234) |
| 2 | `path/to/other.rs` | L15 | False positive | Explained |
| 3 | `path/to/third.rs` | L88 | Already addressed | Referenced |

**Commits created:** 1
**Threads resolved:** 3 / 3
```

---

## Issue Discussion

### ID-1 (MUST): Issue Comment Guidelines

When commenting on issues:

1. **Stay on topic** — address the specific issue being discussed
2. **Be actionable** — provide information that helps resolve the issue
3. **Reference code** — link to relevant source code using repository-relative paths
4. **Avoid noise** — do not post comments that add no value (e.g., "+1", "same here")

### ID-2 (SHOULD): Implementation Context for Issues

When providing implementation context for issue discussion:

```markdown
## Technical Analysis

**Current Behavior:**
[Description of current behavior with code references]

**Root Cause:**
[Analysis of why the issue occurs]
- `path/to/file.rs:L42` - [Relevant code explanation]

**Proposed Solution:**
[Description of proposed fix or implementation]

**Affected Components:**
- [Component 1]: [How it's affected]
- [Component 2]: [How it's affected]

**Estimated Scope:** [Small/Medium/Large]
- Files to modify: [count]
- Tests to add/update: [count]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

---

## GitHub Discussions

### GD-1 (SHOULD): Discussions vs Issues

Use GitHub Discussions for:
- Usage questions and how-to inquiries
- Ideas and brainstorming
- General community discussion
- Show and tell (sharing projects built with Reinhardt)

Use Issues for:
- Bug reports with reproduction steps
- Feature requests with clear requirements
- Documentation errors
- Performance issues with benchmarks

**Discussion URL:** https://github.com/kent8192/reinhardt-web/discussions

### GD-2 (SHOULD): Redirecting Questions

When encountering question-type Issues that are better suited for Discussions:
- Politely suggest GitHub Discussions as a more appropriate venue
- Provide the Discussions URL
- Follow PP-1 authorization policy before posting redirect comments

---

## Agent Context Provision

### AC-1 (SHOULD): Structured Context for Coding Agents

When providing context for external coding agents (GitHub Copilot, Devin, etc.) on Issues or PRs, use structured formats that are easily parseable by both humans and machines.

**When to Provide Agent Context:**
- Issue is assigned to an external coding agent
- PR review requests implementation changes that could be automated
- Issue discussion would benefit from structured task specification

### AC-2 (SHOULD): Agent Context Template

```markdown
## Agent Context

### Task
- **Type:** [Bug Fix | Feature | Refactor | Test | Docs]
- **Scope:** [Affected crate(s) and module(s)]
- **Priority:** [Critical | High | Medium | Low]

### Entry Points
| File | Symbol | Description |
|------|--------|-------------|
| `crates/reinhardt-core/src/model.rs` | `Model::validate` | Primary validation entry point |
| `crates/reinhardt-orm/src/query.rs` | `QueryBuilder::build` | Query construction |

### Reference Implementations
- Pattern to follow: `crates/reinhardt-core/src/existing_feature.rs`
- Test pattern: `crates/reinhardt-core/tests/existing_test.rs`

### Project Constraints
- **Module system:** Rust 2024 edition (`module.rs` + `module/` directory, NO `mod.rs`)
- **SQL construction:** `reinhardt-query` is the default; raw SQL and direct SeaQuery are permitted only as documented fallbacks for cases `reinhardt-query` cannot express yet (e.g., partial indexes and select schema operations)
- **Testing:** `rstest` framework with Arrange-Act-Assert pattern
- **Comments:** English only
- **Indent:** Tab (not spaces)

### Acceptance Criteria
- [ ] [Criterion 1 — verifiable statement]
- [ ] [Criterion 2 — verifiable statement]
- [ ] All existing tests pass (`cargo test --workspace --all --all-features`)
- [ ] Clippy clean (`cargo make clippy-check`)
- [ ] Format clean (`cargo make fmt-check`)

### Files NOT to Modify
- `Cargo.toml` version fields (managed by release-plz)
- `CHANGELOG.md` files (auto-generated)
- `.github/workflows/` (CI configuration)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

---

## Content Guidelines

### CG-1 (MUST): What to Include

- Technical explanations and justifications
- Code references with repository-relative paths and line numbers
- Relevant error messages or log output (wrapped in `<details>` if long)
- Links to related issues, PRs, or documentation
- Structured data (tables, lists) for complex information

### CG-2 (MUST): What to Avoid

- **User requests or AI interaction details** — never mention "user asked me to..." or "I was instructed to..."
- **Absolute local paths** — never include `/Users/...`, `/home/...`, or other machine-specific paths
- **Sensitive information** — never include credentials, tokens, API keys, or private configuration
- **Unfolded long output** — wrap long logs, stack traces, or code blocks in `<details>` tags:

```markdown
<details>
<summary>Full error output</summary>

\`\`\`
[long output here]
\`\`\`

</details>
```

- **Speculation without evidence** — state uncertainty explicitly ("This may be caused by..." not "This is caused by...")
- **Non-actionable comments** — every comment should provide value or move discussion forward

---

## Footer Format

### FF-1 (MUST): Agent Attribution

Use the footer matching the agent that produced the comment. For Codex:

```markdown
🤖 Generated with [Codex](https://openai.com/codex/)
```

For Claude Code:

```markdown
🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Rules:**
- Place at the very end of the comment
- Separate from content with one blank line
- Do NOT include `Co-Authored-By` in comments (that is for commits only)
- This footer is consistent with the PR description footer format

---

## Quick Reference

### ✅ MUST DO

- Get authorization before posting (explicit instruction or Plan Mode approval)
- Preview self-initiated comments and wait for user confirmation
- Write ALL comments in English
- Use GitHub MCP tools or CLI for posting
- Include the actual agent's attribution footer on all comments
- Use repository-relative paths for code references
- Include line numbers when referencing specific code
- Use markdown code blocks with language specifiers
- Wrap long output in `<details>` tags
- Stay on topic and be actionable
- Evaluate and respond to Copilot review threads after PR creation (CR-1 ~ CR-4)
- Reply to every Copilot review thread before resolving (no silent resolves)
- Use GraphQL mutations for thread replies and resolution (CR-4)
- Report Copilot review handling summary after completion (CR-5)

### ❌ NEVER DO

- Post comments without authorization (explicit instruction or Plan Mode approval)
- Post self-initiated comments without previewing and getting confirmation
- Include absolute local paths (`/Users/...`, `/home/...`)
- Include user requests or AI interaction details in comments
- Include sensitive information (credentials, tokens, API keys)
- Post non-actionable or noise comments ("+1", "same here")
- Skip the actual agent's attribution footer
- Post vague comments without code references or technical detail
- Use raw `curl` or a browser for GitHub operations
- Reference code without file path and line number
- Resolve Copilot review threads without posting a reply first
- Poll in a loop waiting for Copilot review to appear
- Dismiss valid Copilot review concerns without fixing the code

---

## Related Documentation

- **Pull Request Guidelines**: instructions/PR_GUIDELINE.md
- **Issue Guidelines**: instructions/ISSUE_GUIDELINES.md
- **Commit Guidelines**: instructions/COMMIT_GUIDELINE.md
- **Documentation Standards**: instructions/DOCUMENTATION_STANDARDS.md
- **Main Quick Reference**: CLAUDE.md (see Quick Reference section)

---

**Note**: This document focuses on commenting and interacting with existing PRs and Issues. For creating PRs, see instructions/PR_GUIDELINE.md. For creating Issues, see instructions/ISSUE_GUIDELINES.md.
