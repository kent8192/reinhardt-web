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
| `reinhardt-core` | TBD | 🔄 in progress | [reinhardt-core](#reinhardt-core) |
| `reinhardt-query` | TBD | ⏳ pending | [reinhardt-query](#reinhardt-query) |
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

PR: TBD · Closes part of [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).

The macro layer removed three classes of deprecated URL resolver codegen.
All three were emitted by `#[routes]` / `#[get(name = …)]` / `#[viewset]`
into the user crate and surfaced as `pub` items at the user crate root,
so removal is a hard breaking change for any project that called the
flat accessors.

### 1. `#[routes]` 2-level URL accessor (`urls.<app>()` / `urls.<app>_client()`)

Deprecated since `0.1.0-rc.16`. The macro no longer emits the
`impl ResolvedUrls { fn <app>(&self) -> <App>Urls<'_> { … } }` block
for either the server or the client gateway.

**Before:**

```rust
let snippets = urls.snippets();           // server side
let snippets = urls.snippets_client();    // client side
```

**After:**

```rust
let snippets = urls.server().snippets();
let snippets = urls.client().snippets();
```

### 2. `#[get(name = "...")]` / `#[post(name = "...")]` per-route resolver trait

Deprecated since `0.1.0-rc.16`. The macro no longer emits the
`Resolve<Name>` blanket-impl trait that produced flat `urls.<name>(...)`
calls. The metadata macro consumed by `__for_each_url_resolver!`
(Issue #3526) is unchanged, so the namespaced typed accessors keep
working.

**Before:**

```rust
use crate::config::urls::url_prelude::*;
let url = urls.snippets_list();           // flat
```

**After:**

```rust
let url = urls.server().snippets().snippets_list();
```

### 3. `#[viewset]` flat ViewSet accessor (`urls.<basename>_list()` / `_detail(id)`)

Deprecated since `0.1.0-rc.29` (refs Issue
[#4507](https://github.com/kent8192/reinhardt-web/issues/4507)). The
macro no longer emits the `Resolve<Pascal>List` /
`Resolve<Pascal>Detail` traits, their blanket impls over
`UrlResolverUnprefixed`, or the `pub use` re-exports that brought them
into the user crate's `url_prelude`. The `__viewset_resolvers_<fn>`
bundle module now contains only the manifest macro alias used by
`#[url_patterns]`.

**Before:**

```rust
use crate::config::urls::url_prelude::*;
let list_url   = urls.snippet_list();
let detail_url = urls.snippet_detail("42");
```

**After:**

```rust
let list_url   = urls.server().snippets().snippet_list();
let detail_url = urls.server().snippets().snippet_detail("42");
```

### Companion removal in `reinhardt-core` itself

`#[routes]` previously emitted an
`impl UrlResolverUnprefixed for ResolvedUrls` override so the flat
ViewSet accessor could resolve against `"<app>:<name>"` instead of the
bare name. With the flat accessor gone there is no caller for the
override, so the override is removed as part of the same PR. The
`UrlResolverUnprefixed` trait itself is removed from `reinhardt-urls`
in PR #8.

### In-tree call site migration

The `examples-tutorial-rest` example dropped:

- its crate-level `#![allow(deprecated)]` (no longer needed)
- the `deprecated_flat_viewset_accessor_matches_typed_accessor` test
  that pinned the flat-vs-typed equivalence
- assorted documentation pointing callers at the flat surface

The typed surface demonstrated in
`examples/examples-tutorial-rest/src/urls_demo.rs` is the canonical
migration target and continues to work unchanged.

---

## reinhardt-query

Section to be populated by PR #2.

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
