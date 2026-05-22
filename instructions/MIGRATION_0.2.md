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
| `reinhardt-query` | TBD | 🔄 in progress | [reinhardt-query](#reinhardt-query) |
| `reinhardt-di` | TBD | ⏳ pending | [reinhardt-di](#reinhardt-di) |
| `reinhardt-conf` | TBD | ⏳ pending | [reinhardt-conf](#reinhardt-conf) |
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

## reinhardt-core

Section to be populated by PR #1.

---

## reinhardt-query

PR: TBD · Closes part of [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).

### `SeaRc<T>` type alias removed

Deprecated since `0.1.0-rc.16`. The alias was a transitional shim
left over from the SeaQuery → `reinhardt-query` fork; the canonical
name has been [`SharedRc`](../crates/reinhardt-query/src/types/iden.rs)
since 0.1.0-rc.16, and the alias is now gone.

`SharedRc<T>` expands to:

- `std::sync::Arc<T>` with the `thread-safe` feature enabled
- `std::rc::Rc<T>` without it (lower overhead in single-threaded contexts)

**Before:**

```rust
use reinhardt_query::SeaRc;

let iden: SeaRc<dyn Iden> = SeaRc::new(MyTable);
```

**After:**

```rust
use reinhardt_query::SharedRc;

let iden: SharedRc<dyn Iden> = SharedRc::new(MyTable);
```

The `pub use iden::SeaRc;` re-export in `crates/reinhardt-query/src/types.rs`
is also dropped, so paths that imported the alias via the crate root
(`reinhardt_query::SeaRc`) must switch to `reinhardt_query::SharedRc`.

No in-tree call sites referenced `SeaRc`, so no examples or
integration tests required migration as part of this PR.

---

## reinhardt-di

Section to be populated by PR #3.

---

## reinhardt-conf

Section to be populated by PR #4.

---

## reinhardt-db

Section to be populated by PR #5.

---

## reinhardt-auth

### CurrentUser → AuthUser (closes #4652)

Section to be populated by PR #6.

---

## reinhardt-rest

Section to be populated by PR #7.

---

## reinhardt-urls

Section to be populated by PR #8.

---

## reinhardt-pages

Section to be populated by PR #9.

---

## reinhardt-testkit

Section to be populated by PR #10.

---

## reinhardt-test

Section to be populated by PR #11.

---

## reinhardt-admin

Section to be populated by PR #12.
