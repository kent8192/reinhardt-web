# Reinhardt 0.2.0 Release Readiness

This document records the release-readiness audit surface for the 0.2.0
stable line. It complements `instructions/RELEASE_PROCESS.md` with concrete
evidence for the current `develop/0.2.0` train.

## Quality Audit

Audit timestamp: 2026-06-09T12:23:09Z.

Release PR context:

- Develop release PR: <https://github.com/kent8192/reinhardt-web/pull/5223>
- CI run: <https://github.com/kent8192/reinhardt-web/actions/runs/27195739150>
- SemVer run: <https://github.com/kent8192/reinhardt-web/actions/runs/27195738823>
- Current release PR state at audit time: open, behind `develop/0.2.0`
- Release CI blocker fix already merged: PR #5226

### Public API Documentation

Evidence:

- `Docs.rs Build Check` passed in run `27195739150`.
- `Documentation Tests` passed in run `27195739150`.
- `cargo check`, clippy, feature checks, UI tests, and cross-platform checks
  passed in the same release PR run.

Merge gate:

- Regenerate or update the release PR after PR #5226 so the docs evidence is
  attached to a current merge ref.
- Treat any new rustdoc warning as a release blocker until fixed or linked to a
  focused issue.

### TODO and Placeholder Audit

Evidence:

- `TODO Check / Clippy TODO/unimplemented/dbg lint` passed in run
  `27195739150`.
- `TODO Check / Scan for unresolved TODO/FIXME` passed in run `27195739150`.
- Semgrep cloud scan passed for the same release PR run.

Merge gate:

- Re-run the TODO workflow on the regenerated release PR.
- Do not merge stable-release preparation with new `todo!()`, `// TODO`, or
  `// FIXME` markers in public API paths.

### SemVer Audit

Evidence:

- SemVer discovery completed in run `27195738823`.
- The visible SemVer jobs for core public crates passed, including
  `reinhardt-core`, `reinhardt-conf`, `reinhardt-db`, `reinhardt-auth`,
  `reinhardt-pages`, `reinhardt-rest`, `reinhardt-urls`, and
  `reinhardt-websockets`.

Merge gate:

- Wait for all SemVer matrix jobs on the current release PR to finish.
- Classify any failure as either an intentional 0.2 breaking change covered by
  `instructions/MIGRATION_0.2.md` or a release blocker with a linked issue.

### Security Audit

Evidence:

- `Security Audit / Scan for known vulnerabilities` passed in run
  `27195739150`.
- CodeQL completed successfully for the same release PR context.

Merge gate:

- Re-run security audit on the regenerated release PR.
- High or critical advisories block stable release unless a linked issue
  records the mitigation or acceptable-risk rationale.

### Agent-Assisted QA

Evidence:

- PR #5223 exposed two concrete release CI failures:
  - stale `reinhardt-pages-macros` attribute codegen expectation;
  - hot-reload broadcast test waiting on `127.0.0.1:0`.
- PR #5226 fixed both failures on `develop/0.2.0`.
- The remaining `CI Success` failure in the stale run was caused by aggregate
  evaluation against the old run state, not a distinct source-code failure.

Merge gate:

- Regenerate the release PR after PR #5226.
- If a regenerated release PR still fails, create a focused blocker issue
  instead of carrying ambiguous release-blocker state in the phase tracker.

## Blocker Inventory

Known blocker status at audit time:

- PR #5223 is behind `develop/0.2.0` and must be regenerated.
- PR #5226 has merged the known unit and intra-crate test fixes.
- `instructions/MIGRATION_0.2.md` still needs the dedicated migration-guide
  completion PR before the stable release preparation phase can close.

Stable release must not proceed until:

- all high and critical non-`agent-suspect` blockers are closed or explicitly
  waived with a linked issue;
- the regenerated release PR has current CI evidence;
- the migration guide is release-ready.
