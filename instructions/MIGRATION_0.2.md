# Migration Guide: 0.1.0 → 0.2.0

Umbrella tracker: [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).
Companion: [#4652](https://github.com/kent8192/reinhardt-web/issues/4652).

> Filled in incrementally by each per-crate PR.

## Quick removal index

| Crate | Status |
|---|---|
| reinhardt-core / -query / -di / -conf (partial) | shipped via PRs #4713 / #4717 / #4722 / #4728 |
| reinhardt-db | 🔄 this PR |
| (others) | ⏳ pending |

---

## reinhardt-db

### `DatabaseConnection::get_database_url_from_env_or_settings(base_dir)` removed

Deprecated since `0.1.0-rc.29` per Issue #4520. Use
`DatabaseConnection::database_url_from(settings, env_override)` with a
pre-built `ProjectSettings`.

```rust
// Before
let url = DatabaseConnection::get_database_url_from_env_or_settings(None)?;

// After
let settings: ProjectSettings = /* build via SettingsBuilder */;
let url = DatabaseConnection::database_url_from(&settings, env_override)?;
```

The new API is cheaper (no per-call disk re-read) and surfaces the
settings dependency at the call site instead of hiding it behind a
fresh `SettingsBuilder::new()`.

#### Follow-up consumer migration

`reinhardt-commands/src/builtin.rs::get_database_url_from_settings`
still references the removed entry point and will be migrated in a
follow-up `chore(commands)!` PR.
