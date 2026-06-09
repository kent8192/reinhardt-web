# Reinhardt 0.2.0 Stable Release Execution Tracker

This tracker is the merge gate for closing the 0.2.0 stable milestone phase
issues. It must not be merged until each section below has current evidence.

## Phase Closure Order

1. Phase 1 quality audit closes after public docs, TODO, SemVer, security, and
   agent-assisted QA records are merged.
2. Phase 2 stability verification closes after the blocker inventory is empty
   or each remaining blocker has an explicit waiver issue.
3. Phase 3 release preparation closes after changelog review, documentation,
   migration guide, and final validation are current on `develop/0.2.0`.
4. Phase 4 release execution closes only after the stable release PR is merged,
   crates are published, release notes exist, and the next development branch is
   established.

## Pre-Release Validation

Required before promoting `0.2.0` stable:

- `cargo check --workspace --all --all-features`
- `cargo nextest run --workspace --all-features`
- `cargo doc --workspace --no-deps --all-features`
- `cargo make fmt-check`
- `cargo make clippy-check`
- `cargo make clippy-todo-check`
- Semgrep TODO/FIXME scan
- `cargo make audit`
- release-plz dry run or equivalent generated release PR review

Known current state:

- PR #5226 fixed the release CI unit and intra-crate failures observed on PR
  #5223.
- PR #5223 is stale and must be regenerated from current `develop/0.2.0`
  before stable promotion decisions are made.

## Stable Release Execution

Required for the stable release issue:

- Strip prerelease suffixes from all publishable crates through the documented
  release workflow.
- Review the release-plz-generated PR for version diffs, changelog diffs,
  Cargo.toml dependency versions, and publish-order correctness.
- Merge the stable release PR through the protected-branch process.
- Do not manually create release tags.

## Publication Verification

Required after publishing:

- Confirm every publishable crate is available on crates.io at `0.2.0`.
- Confirm each expected `<crate>@v0.2.0` tag exists.
- Confirm docs.rs builds have succeeded or are linked to follow-up blockers.
- Confirm `main` has no partial-publish or version-regression state.
- Apply the partial-release recovery procedure from
  `instructions/RELEASE_PROCESS.md` if any crate publish fails after earlier
  crates succeeded.

## Release Notes and Announcement

Required after publication:

- Create the GitHub Release from the reviewed stable changelog.
- Mark the release as stable, not pre-release.
- Link the migration guide from release notes.
- Publish the public announcement in the project-approved channel.
- Ensure install snippets reference `0.2.0` stable.

## Next Development Branch

Required after stable release:

- Confirm the next development branch name according to the branch policy.
- Use worktree-based merge flow; do not rebase or force-push.
- Merge `origin/main` into the next development branch after the stable release
  lands.
- Resolve conflicts with intent preserved and push the branch normally.
- Review CI status on the next development branch.

## Final Parent Tracker Closure

The parent milestone tracker closes only after:

- Phase 1, Phase 2, Phase 3, and Phase 4 issues are closed by merged PRs or
  explicit linked deferrals.
- Excluded issues #5089 and #5128 remain outside this closure batch.
- No high or critical release blocker remains ambiguous.
- The release execution evidence above is current and linked from the final PR.
