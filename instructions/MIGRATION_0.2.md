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
| `reinhardt-di` | TBD | 🔄 in progress | [reinhardt-di](#reinhardt-di) |
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

Section to be populated by PR #2.

---

## reinhardt-di

PR: TBD · Closes part of [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).

Removed two FastAPI-inspired injection wrappers in favour of the
canonical [`Depends<T>`](../crates/reinhardt-di/src/depends.rs)
extractor.

### `Injected<T>` → `Depends<T>`

Deprecated since `0.1.0-rc.16`. `Injected<T>` was a transitional
wrapper that coexisted with `Depends<T>` during the migration window;
the migration window is closed and the wrapper is gone.

**Before:**

```rust
use reinhardt_di::{Injected, Injectable};

#[injectable]
struct Handler {
    #[inject]
    db: Injected<Database>,
}
```

**After:**

```rust
use reinhardt_di::Depends;

#[injectable]
struct Handler {
    #[inject]
    db: Depends<Database>,
}
```

Field-access semantics are unchanged — `Depends<T>` derefs to `&T` the
same way `Injected<T>` did.

### `OptionalInjected<T>` → `Option<Depends<T>>`

Deprecated since `0.1.0-rc.16`. The alias was sugar for
`Option<Injected<T>>`; the canonical form drops the alias.

**Before:**

```rust
#[injectable]
struct Handler {
    #[inject]
    cache: OptionalInjected<Cache>,
}
```

**After:**

```rust
#[injectable]
struct Handler {
    #[inject]
    cache: Option<Depends<Cache>>,
}
```

### `#[injectable]` macro error message

The `#[inject]` field validation error now reads:

```text
#[inject] field must have type Depends<T> or Option<Depends<T>>
```

Previously it also accepted `Injected<T>` and `OptionalInjected<T>`.

### What survived this PR

`InjectionMetadata` (struct) and `DependencyScope` (enum) — the
metadata types that lived alongside `Injected<T>` — are unchanged.
`Depends<T>` still uses them, so they remain in
`crates/reinhardt-di/src/injected.rs` for now. The module file name
(`injected.rs`) is a candidate for renaming in a follow-up PR; this
PR keeps the rename out of scope to focus the diff.

### In-tree call site migration

No in-tree call sites referenced `Injected<T>` or `OptionalInjected<T>`
in this repository (verified via workspace-wide grep before this PR),
so no `examples/` or integration-test updates were required here. The
sister project `reinhardt-cloud` should audit its own DI usage as part
of the cross-repo migration tracked separately.

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
