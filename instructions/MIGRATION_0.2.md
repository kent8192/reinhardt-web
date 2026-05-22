# Migration Guide: 0.1.0 → 0.2.0

This guide enumerates every public API removed (or, in one case,
converted to a type alias) on the path from `0.1.0-rc.*` to `0.2.0`.

See umbrella Issue [#4520](https://github.com/kent8192/reinhardt-web/issues/4520)
for the full rationale and PR tracker. See companion Issue
[#4652](https://github.com/kent8192/reinhardt-web/issues/4652) for the
`CurrentUser → AuthUser` unification (delivered as part of `reinhardt-auth`).

> **Status:** This file is filled in incrementally as each per-crate PR
> lands. Empty sections below are placeholders for upcoming PRs.

---

## Quick removal index

| Crate | PR | Status | Section |
|---|---|---|---|
| `reinhardt-core` | TBD | ⏳ pending | [reinhardt-core](#reinhardt-core) |
| `reinhardt-query` | TBD | ⏳ pending | [reinhardt-query](#reinhardt-query) |
| `reinhardt-di` | TBD | ⏳ pending | [reinhardt-di](#reinhardt-di) |
| `reinhardt-conf` | TBD | 🔄 in progress (4 of 8 items) | [reinhardt-conf](#reinhardt-conf) |
| `reinhardt-db` | TBD | ⏳ pending | [reinhardt-db](#reinhardt-db) |
| `reinhardt-auth` | TBD | ⏳ pending | [reinhardt-auth](#reinhardt-auth) |
| `reinhardt-rest` | TBD | ⏳ pending | [reinhardt-rest](#reinhardt-rest) |
| `reinhardt-urls` | TBD | ⏳ pending | [reinhardt-urls](#reinhardt-urls) |
| `reinhardt-pages` | TBD | ⏳ pending | [reinhardt-pages](#reinhardt-pages) |
| `reinhardt-testkit` | TBD | ⏳ pending | [reinhardt-testkit](#reinhardt-testkit) |
| `reinhardt-test` | TBD | ⏳ pending | [reinhardt-test](#reinhardt-test) |
| `reinhardt-admin` | TBD | ⏳ pending | [reinhardt-admin](#reinhardt-admin) |

Legend: ✅ done · ⏳ pending · 🔄 in progress

---

## reinhardt-core / reinhardt-query / reinhardt-di

Sections populated by PRs #4713 / #4717 / #4722.

---

## reinhardt-conf

PR: TBD · **First of two PRs** for this crate's #4520 cleanup.

### What this PR removes (4 of 8 items)

The four items below have no `Settings` struct dependents in the
workspace and can be removed cleanly without rippling into other
crates. The remaining four items (`Settings` struct itself plus
`Settings::add_app` and `Settings::with_validated_apps`) require
coordinated migration of `reinhardt-apps` and `reinhardt-middleware`
and are deferred to a follow-up PR.

#### `AdvancedSettings` → fragment composition

Deprecated since `0.1.0-rc.16`. Replace with `ProjectSettings`
composed from the individual fragment types
(`CacheSettings`, `SessionSettings`, `DatabaseSettings`,
`StaticSettings`, `MediaSettings`, `EmailSettings`,
`LoggingSettings`, `CorsSettings`) which all remain available.

#### `TomlFileSource::set_interpolation(bool)` → `with_interpolation()` / `without_interpolation()`

Deprecated since `0.1.0-rc.27` (refs Issue #4224).

```rust
// Before
TomlFileSource::new(path).set_interpolation(true);

// After
TomlFileSource::new(path).with_interpolation();
```

#### `JsonFileSource` → `TomlFileSource`

Deprecated since `0.1.0-rc.26` (refs Issue #4087). TOML is the
canonical configuration format. Migrate `.json` files to `.toml`
(TOML supersets typical JSON config use), or implement the public
`ConfigSource` trait against `serde_json` out of tree.

#### `auto_source(path)` → `TomlFileSource::new(path)`

Deprecated since `0.1.0-rc.26` (refs Issue #4087). Construct the
canonical TOML source directly so the configuration format is
explicit at the call site.

### Follow-up PR scope

The follow-up PR will remove:
- `Settings` struct (`src/settings.rs`, deprecated since `0.1.0-rc.16`)
- `Settings::add_app` and `Settings::with_validated_apps` methods
- Re-exports of `Settings` from `reinhardt-conf` (`lib.rs`, `prelude.rs`)
- `Settings` dependents in `reinhardt-apps/src/lib.rs` (re-export +
  docs) and `reinhardt-middleware/src/{session,allowed_hosts,csrf}.rs`
  (production-code `from_settings(&Settings)` APIs and tests)

---

## reinhardt-db / reinhardt-auth / reinhardt-rest / reinhardt-urls / reinhardt-pages / reinhardt-testkit / reinhardt-test / reinhardt-admin

Sections to be populated by PRs #5–#12.
