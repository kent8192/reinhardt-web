# RFC - CI Style Auto-Fix

| Key | Value |
|---|---|
| Status | Draft |
| Date | 2026-06-30 |
| Scope | GitHub Actions CI for pull-request style gates |
| Author | kent8192 |

---

## 1. Context

The CI workflow currently fails pull requests when either the format gate or the
Clippy gate finds style drift. The repository already has the authoritative fix
commands:

- `cargo make fmt-fix`
- `cargo make clippy-fix`
- `cargo make auto-fix`

The format command is repository-specific. It runs the Reinhardt formatter and
therefore covers `page!` DSL formatting in addition to regular Rust formatting.
Using plain `cargo fmt` would not match the existing CI gate.

## 2. Goals

Automatically commit and push safe style fixes when a pull request fails the
format or Clippy gate.

Keep the write boundary narrow:

- only same-repository pull requests are eligible;
- fork pull requests remain read-only;
- protected branches remain read-only;
- release-managed branches remain read-only;
- no pull-request comments are posted by the auto-fix workflow.

## 3. Non-Goals

- Fixing compile, test, WASM, docs, publish, audit, or SemVer failures.
- Running auto-fix on fork pull requests.
- Pushing to `main`, `master`, `develop/*`, `release/*`, `release-plz-*`, or
  `develop-release-plz-*`.
- Force-pushing, rebasing, or rewriting existing branch history.
- Creating pull requests, closing issues, or posting GitHub comments.

## 4. Selected Approach

Add an `auto-fix-style` job to `.github/workflows/ci.yml`.

The job runs only when the reusable `fmt` or `clippy` job fails. It is gated to
`pull_request` events where the pull-request head repository is the same as the
base repository, and where the head branch is not protected or release-managed.

The job checks out the pull-request head branch, installs the same tools used by
the style gates, runs the matching fix command, and commits only when the fix
command leaves a worktree diff.

## 5. Execution Flow

1. `ci.yml` runs the existing `fmt` and `clippy` reusable workflows.
2. If either job fails, `auto-fix-style` evaluates its safety gate.
3. If the pull request is ineligible, the job exits without pushing.
4. If eligible, the job checks out the PR head branch.
5. The job installs Rust, `rustfmt`, `clippy`, `protoc`, and `cargo-make`.
6. The job runs:
   - `cargo make fmt-fix` when only formatting failed;
   - `cargo make clippy-fix` when only Clippy failed;
   - `cargo make auto-fix` when both failed.
7. If there is no diff after the fix command, the job exits without committing.
8. If there is a diff, the job commits it as `ci: auto-fix fmt and clippy`.
9. The job pushes the new commit to the pull-request head branch.

## 6. Token and Push Model

The job runs formatter and Clippy fix commands without a write token available to
the checkout. The checkout uses `persist-credentials: false`, and the GitHub App
token is generated only after the fix commands have completed and a worktree diff
has been detected.

Use the existing GitHub App token pattern that is already present in repository
workflows that need write access. This avoids relying on `GITHUB_TOKEN` behavior
for triggering follow-up workflows after a push.

The generated token is used only by the final push step. The push target is the
exact pull-request head branch. The workflow never pushes to protected or
release-managed branches.

## 7. Failure Handling

The auto-fix job does not mask the original style failure unless it successfully
pushes a new commit.

If the fix command fails, the job fails and pushes nothing. If the fix command
succeeds but leaves no diff, the job pushes nothing. If the push fails, the job
fails and leaves the pull request in its original failing state.

This keeps false-positive auto-fix runs visible while still allowing a successful
push to trigger a fresh CI run on the corrected branch.

## 8. Commit Shape

The auto-fix commit uses:

- author: `github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>`;
- message: `ci: auto-fix fmt and clippy`;
- body: a short note with the workflow run URL.

The workflow does not post PR comments. The pushed commit is the audit trail.

## 9. Validation

Local validation for the workflow change:

- `actionlint -shellcheck= .github/workflows/ci.yml`
- shell syntax checks for any extracted multi-line shell script used by the job

Hosted validation:

- create or update a same-repository test PR with intentional formatter drift;
- confirm the `auto-fix-style` job pushes a fix commit;
- confirm the new pushed commit triggers a fresh CI run;
- confirm fork and protected-branch cases remain read-only by inspection of the
  workflow conditions.

## 10. Open Risks

Clippy auto-fix may not repair every warning. That is acceptable: the workflow
pushes only when a concrete diff is produced, and otherwise leaves the original
failure for manual repair.

The auto-fix job can add CI runtime after a style failure. This is acceptable
because it runs only after a failing gate and can save a full manual fix cycle.
