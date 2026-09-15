# Task Prompts

Use these templates when starting Reinhardt work. Replace angle-bracket fields
with task facts and remove unused lines. A natural-language request with the same
information is equally valid; this is not a mandatory intake form.

Specify the observable result, scope, authoritative context, and completion
evidence. Let the agent choose routine implementation steps. Refer to repository
instructions instead of pasting their full contents into every task.

## Implementation or Bug Fix

```text
Implement <behavior> in <repository/worktree or branch>.
Context: <issue, accepted design, or file paths>.
Acceptance criteria:
- <observable result, including an important error/boundary case>
- <compatibility or target requirement>
Scope: <included components and any explicit exclusions>.
Delivery: <local changes / commit and push / Draft PR / Ready PR>.
Preserve the accepted design. Make routine choices within scope and continue
through the relevant checks. Ask only for decisions that materially affect the
result or require permission not already supplied by this task and repository.
Report acceptance results, changed files, validation, and remaining work.
```

If delivery is unspecified, use the repository's standing authorization and task
context. Preserve explicit local-only, Draft, no-push, or no-comment constraints.

## Review Without Modification

```text
Review <branch/PR/diff> against <base commit or branch>.
Compare both repository standards and <issue/specification> acceptance criteria.
Work in the current agent. Inspect the implementation and affected callers.
Report actionable findings with severity, path/line, concrete failure behavior,
and supporting evidence; state when no actionable finding is established.
Deliver the review locally. Leave files and GitHub state unchanged.
```

## Resolve Review Feedback

```text
Resolve all actionable <reviewer/all-reviewer scope> feedback on <PR URL>.
Use its authoritative worktree and fetch every page of review threads, comments,
and review bodies. Validate findings and implement the smallest complete fixes.
This task authorizes focused commits, normal pushes, English replies with the
required attribution, and resolution of threads whose concerns are addressed.
Verify local/upstream/remote/PR heads, then reply before resolving. Re-fetch the
complete inventory after each push and include new in-scope findings.
Finish with no actionable unresolved feedback in the requested scope, or list
the exact blocker for each remaining item. Report CI separately.
```

For a task that requests only analysis or local fixes, omit the authorization
paragraph and state that delivery boundary explicitly.

## Repair CI

```text
Diagnose and repair the failed checks on <PR URL or run ID>.
Use the PR's current head and authoritative worktree. Read every failed leaf-job
log and enumerate independent failures before choosing fixes.
Distinguish source regressions from infrastructure/setup failures.
Reproduce and validate the affected checks; expand coverage where the changed
surface requires it.
Delivery: <local fixes / commit and normal push>.
Report root causes, fixes, validation, and current remote check state.
```

## Frontend Verification

```text
Verify <interaction and expected behavior> in <worktree/application>.
Use <known runtime URL/configuration, if available> and disposable fixtures.
Confirm the served native/WASM bundle corresponds to the selected source.
Exercise <required success/error/boundary scenarios> with browser assertions.
Work in the current agent using the existing suite or available browser tools.
Delivery: <evidence and findings / fixes with regression coverage>.
Report passed, failed, and unrun scenarios with the source, URL, and evidence.
```

To delegate QA, add an explicit instruction such as: "Delegate browser
verification to frontend-qa-tester; assign it <bounded scenario and file
ownership>." A request for comprehensive testing alone is not delegation.

## Instructions and Prompt Maintenance

```text
Audit and improve <specified AGENTS/instructions/task-prompt paths>.
Read the relevant installed skills and identify contradictions, stale tool
assumptions, repeated rules, unnecessary approval pauses, and weak completion
criteria. Preserve authorization, release, security, and target boundaries.
Keep universal constraints in the entrypoint and conditional detail behind
explicit references. Mirror AGENTS.md and CLAUDE.md.
Validate links, structured prompt syntax, and representative task decisions.
Report the resulting changes and any limitations; do not claim measured model
improvements without a comparison on representative tasks.
```

## Continue Existing Work

```text
Continue <task> in <authoritative worktree>.
The accepted goal and design are <reference or concise summary>.
Completed and verified: <results with revision/command evidence>.
Remaining: <ordered obligations and known blockers>.
Existing delivery authorization: <scope>; additional constraints: <if any>.
Check current state, preserve completed work, and finish the remaining criteria.
Answer status questions briefly while continuing the task unless told to stop.
```

## Repository-Managed Runtime Prompts

- `.codex/agents/frontend-qa-tester.toml`: delegated QA role. Keep its invocation
  condition consistent with AGENTS.md and parse TOML after edits.
- `scripts/prompts/announcement-system.md`: release-announcement generation.
  Preserve its JSON fields, Markdown output structure, and factual source rules.
  The caller in `.github/workflows/release-plz.yml` controls model selection and
  publication; editing this prompt does not authorize changing either.
- Shared skill installations and user runtime settings are outside repository
  prompt maintenance. Propose separate changes when those are necessary.
