# CLAUDE.md

## Project

Reinhardt is a Rust 2024 workspace: https://github.com/kent8192/reinhardt-web.
Use `reinhardt-query` for SQL construction and Docker for TestContainers.
Read [README.md](README.md) for the product overview.

## Working Agreement

- Communicate in Japanese; write code, comments, technical documentation, commits,
  and GitHub content in English.
- Carry action requests through implementation and relevant verification. Reuse
  approved designs and existing authorization. Ask when a missing decision changes
  scope, compatibility, or an operation's permission; continue independent work.
- Work in the current agent. Delegate only when the user explicitly requests it
  for this task; a skill, agent description, or separate crate does not authorize it.
- Confirm the authoritative checkout, branch, and existing changes before editing.
  Preserve unrelated work and the user's stated file boundaries.
- Use `rg` / `rg --files` for discovery. Batch independent reads; keep dependent
  edits and Git mutations sequential.
- Read only the instructions and skills relevant to the task. For skill conflicts,
  long tasks, or tool selection, read [Agent Workflow](instructions/AGENT_WORKFLOW.md).
- Report the result, relevant verification, and remaining work concisely. Keep
  local validation, pushed changes, CI, merge, and publication status distinct.

## Engineering Constraints

- Use `module.rs` with a `module/` directory; `mod.rs` is prohibited by project
  convention. Apply [Design Philosophy](instructions/DESIGN_PHILOSOPHY.md) when
  designing APIs and [Module System](instructions/MODULE_SYSTEM.md) when adding modules.
- Prefer borrowing over unnecessary `.to_string()`. Remove obsolete code instead
  of leaving deletion records. Explain every `#[allow(...)]`.
- Own resources through RAII guards. Manual cleanup that can be skipped on early
  return or panic is not sufficient. Justify intentional `mem::forget`,
  `ManuallyDrop`, or immediate guard drops. See [Anti-Patterns](instructions/ANTI_PATTERNS.md).
- Mark unfinished implementations with `todo!()` or `// TODO:` during development,
  then remove new placeholders before delivery. Reserve `unimplemented!()` for
  intentionally excluded features; check the applicable lints before using it.
- Use absolute paths or at most one `../` in filesystem operations. Keep temporary
  files under `/tmp` and remove task-owned temporary and backup files when no
  longer needed. Preserve deliverables and other worktrees.
- Update relevant documentation with behavior changes. Planned features belong
  in the `lib.rs` header. Describe technical reasons rather than conversation
  history. Read [Documentation Standards](instructions/DOCUMENTATION_STANDARDS.md)
  when changing documentation or examples.
- For external dependency workarounds, read
  [Upstream Issue Reporting](instructions/UPSTREAM_ISSUE_REPORTING.md) before editing:
  record evidence internally and document the removal condition and ideal
  implementation. External reporting is not a prerequisite for a local fix.
- Before changing public or cross-target APIs, read
  [Stability Policy](instructions/STABILITY_POLICY.md) and
  [API Parity](instructions/API_PARITY.md). Preserve native/WASM and compatibility
  boundaries; API changes during RC have specific approval requirements.

## Verification

Choose checks using [Verification Scope](instructions/AGENT_WORKFLOW.md#verification-scope).
Complete applicable required checks; broaden only for changed shared behavior,
failures, or unresolved concerns. Reuse successful results for unchanged inputs.

- Rust behavior changes need meaningful regression coverage. Read
  [Testing Standards](instructions/TESTING_STANDARDS.md) before writing tests:
  use Reinhardt components, `rstest`, precise assertions, AAA, scoped fixtures,
  RAII cleanup, and `#[serial(group_name)]` for shared global state.
- Cross-crate integration tests belong in `tests/`; component tests belong in
  the functional crate. Follow [Release Process KI-2](instructions/RELEASE_PROCESS.md)
  for `reinhardt-test` dependencies and publish ordering.
- Infrastructure tests use Docker, never Podman. Check `docker ps` and
  `DOCKER_HOST` before running them; use the configured Docker socket.
- Rustdoc changes require the affected documentation build and executable
  examples. Prose-only instruction changes require links, consistency, and
  format checks; they do not require rebuilding the Rust workspace.
- Diagnose CI from the failed leaf-job logs. A queued check or successful
  aggregate without the relevant results is not proof of success.

## Git Workflow

### Autonomous Operation Policy (Reinhardt Family)

The permission table in [Commit Execution Policy](instructions/COMMIT_GUIDELINE.md#ce-1-must-execution-authorization)
is authoritative for commits, pushes, Draft PRs, Issue creation, and operations
that need explicit permission. Its standing authorization applies to this
repository; do not turn it into a repeated approval question. An explicit task
restriction such as local-only work still controls the delivery scope.

- Before creating or editing an external Issue, PR, comment, reply, or review,
  follow [External Publication](instructions/GITHUB_INTERACTION.md#pp-0-must-external-publication).
  Check the actual destination's contribution/AI policy and specific posting
  authorization. The working directory does not authorize external publication;
  prohibited or unverified publication stays blocked even with user approval.
- `main`, `master`, `develop/*`, and `release/*` are protected. Direct commits
  or pushes, history rewriting, PR merges, and destructive operations need explicit
  authorization. Release tags are created by release-plz, not manually.
- Before branching, inspect `git worktree list` and matching names in
  `git branch -a`. Follow [PR Base Branch Policy](instructions/PR_BASE_BRANCH_POLICY.md)
  for the source and target. Reuse the task's worktree when available.
- Resolve PR conflicts by merging the target branch into the source in a
  worktree, then validate and push normally. Do not rebase or force-push.
- Create one focused commit at a time under
  [Commit Guidelines](instructions/COMMIT_GUIDELINE.md); review the staged diff.
  Dry-run batch operations before applying them.
- Use [GitHub Interaction PP-3](instructions/GITHUB_INTERACTION.md#pp-3-must-github-tool-selection)
  for tool selection and PP-1 for comment authorization. Replies and reviews
  require authorization independently of commit/push permission.
- PRs follow [PR Guidelines](instructions/PR_GUIDELINE.md) and
  [.github/PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md).
  Draft-to-Ready uses PC-4a; CI completion is not required for that transition.
  Breaking PRs use `type!(scope): description` (or `type!: description`) and
  the `breaking-change` label.
- Issues follow [Issue Guidelines](instructions/ISSUE_GUIDELINES.md) and the
  appropriate template. Report vulnerabilities privately under [SECURITY.md](SECURITY.md).
  Read [Issue Handling](instructions/ISSUE_HANDLING.md) for batch work.
- Before release work, read [Release Process](instructions/RELEASE_PROCESS.md).
  Let release-plz own version bumps and tags. Put code fixes on a separate branch
  targeting the release PR's base, never directly on a release-plz branch.

## Instruction Maintenance

`AGENTS.md` and `CLAUDE.md` are mirrored entrypoints. Edit both together.
Only `AGENTS.md` / `CLAUDE.md`, `AGENTS.local.md` / `CLAUDE.local.md`, and
`Codex attribution` / `Claude Code attribution` substitutions may differ.
Run `diff CLAUDE.md AGENTS.md` and verify only those substitutions remain.

Keep always-needed rules here and conditional detail in one canonical instruction
file. Use [Quick Reference](instructions/QUICK_REFERENCE.md) to find the relevant
standard and [Task Prompts](instructions/TASK_PROMPTS.md) when composing work.

If `CLAUDE.local.md` exists, read it for local preferences. When editing
`examples/`, also read [examples/CLAUDE.md](examples/CLAUDE.md).

For durable knowledge beyond existing documentation, follow
[Obsidian Wiki](instructions/OBSIDIAN_WIKI.md). Check MCP availability once and
skip wiki work if unavailable; keep knowledge capture out of the critical path.
