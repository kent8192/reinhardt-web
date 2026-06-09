# Reinhardt 0.2.0 Changelog Review

This file records the stable-release changelog review for the 0.2.0 train.
It exists because release-plz generated PRs can show only the latest delta, but
the stable release needs a coherent summary of the full RC series.

## Review Inputs

- Current develop release PR: <https://github.com/kent8192/reinhardt-web/pull/5223>
- Current generated version at review time: `0.2.0-rc.5`
- Release branch base: `develop/0.2.0`
- Stable migration guide: `instructions/MIGRATION_0.2.md`
- Release process: `instructions/RELEASE_PROCESS.md`

## Stable 0.2.0 Highlights

The stable announcement and final changelog review should group the RC series
under these user-facing themes:

- Removed public APIs deprecated during the 0.1.0 RC cycle and documented the
  migration path in `instructions/MIGRATION_0.2.md`.
- Stabilized typed settings composition, TOML interpolation, secret references,
  and embedded settings schema nodes.
- Preserved the `develop/0.2.0` facade and feature-gating contract for the
  release-plz publish path.
- Repaired release-publishing infrastructure for publishable tooling crates,
  docs.rs checks, SemVer checks, stale release PR regeneration, and ignored
  tracked files.
- Improved `reinhardt-pages` and `runserver` development-loop behavior through
  targeted hot-reload, WASM, and generated-page updates.
- Updated examples and release documentation so downstream users can migrate
  without relying on RC-only wording.

## Per-Crate Review Requirements

Before the final stable release PR is merged:

- Every publishable crate with a generated `0.2.0` entry must have a changelog
  section that is understandable without reading the full PR history.
- Breaking changes must link to `instructions/MIGRATION_0.2.md` or the
  relevant crate-level migration note.
- Maintenance-only dependency/version churn should remain in the Maintenance
  section and must not obscure behavior-changing entries.
- The release date must be the actual stable publish date, not the date of an
  RC prerelease or preparation PR.
- The final release PR must be reviewed for version diffs, changelog diffs,
  Cargo manifest dependency versions, and publish-order correctness.

## Release-PR Gate

Do not merge the stable release PR while any of these are true:

- generated changelog entries still describe `0.2.0` as only an RC train;
- the migration guide contains unresolved placeholder wording;
- SemVer or docs.rs checks are incomplete on the current merge ref;
- release-plz output omits a publishable crate that changed since the last
  published RC;
- a high or critical release blocker remains unlinked.

## Post-Merge Check

After the stable release PR merges, verify that the generated changelog and
GitHub Release notes agree on:

- the final stable version `0.2.0`;
- the actual publish date;
- the list of published crates and tags;
- the migration guide link;
- any known follow-up issues that remain outside the stable release scope.
