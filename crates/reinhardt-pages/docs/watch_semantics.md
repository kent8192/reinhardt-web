# `watch { }` semantics — when to flatten

> Tracking: [#4515](https://github.com/kent8192/reinhardt-web/issues/4515)

The `watch { }` block inside the `page!` macro lowers to
`Page::reactive(move || { ... })` (see
`crates/reinhardt-pages/macros/src/page/codegen.rs` `generate_watch`). The
generated closure is then bound by `F: ReactiveRenderFn`, which requires
`Fn() -> Page + 'static` (the reactive runtime re-invokes the body on every
signal change).

This page documents the most common footgun that comes with that bound and
the two recommended fixes, plus the rationale.

## The footgun: nested `watch { }` sharing a `!Copy` signal

`Signal<T>` is intentionally `!Copy` — it is `Rc`/`Arc`-backed and cheap to
`Clone`, but never silently copied. That makes the following pattern
**reject**:

```rust,ignore
page!(|load, loading, error| {
    watch {                                // outer Page::reactive(move || ...)
        if load.is_pending() {
            div { "loading data..." }
        } else {
            div {
                watch {                    // inner Page::reactive(move || ...)
                    if loading.get() { div { "submitting..." } }
                    else if error.get().is_some() {
                        div { class: "alert", { error.get().unwrap_or_default() } }
                    }
                    else { div { "ready" } }
                }
            }
        }
    }
})(load, loading, error)
```

The outer `move` closure already owns `loading` and `error`. The inner
`move` closure tries to take them again, but `Signal<bool>` and
`Signal<Option<String>>` are `!Copy`, so the second move fails. The
compiler reports `E0507: cannot move out of value, a captured variable in
an Fn closure`, pointing at the outer closure body and at the `F: Fn() ->
Page + 'static` bound on `Page::reactive` (or, equivalently, at
`ReactiveRenderFn`).

## Fix 1 (preferred): flatten into a single `watch { }`

A single `watch { }` already subscribes to **every** `Signal` it reads.
There is no reactivity benefit to nesting watches — the inner watch can
almost always become a plain `if` inside the outer `watch`:

```rust,ignore
page!(|load, loading, error| {
    watch {
        if load.is_pending() {
            div { "loading data..." }
        } else {
            div {
                if loading.get() { div { "submitting..." } }
                else if error.get().is_some() {
                    div { class: "alert", { error.get().unwrap_or_default() } }
                }
                else { div { "ready" } }
            }
        }
    }
})(load, loading, error)
```

This is the fix that landed in `examples/examples-tutorial-basis` poll
detail / question edit pages (see the
[PR #4517](https://github.com/kent8192/reinhardt-web/pull/4517)
discussion that surfaced this Issue).

## Fix 2 (when the nested `watch { }` is intentional): clone inside the outer body

If the nested `watch { }` is structurally necessary (for instance, you
need the inner reactive scope to expire independently of the outer), clone
the signal into a fresh binding inside the outer closure before
constructing the inner `watch { }`:

```rust,ignore
page!(|load, loading, error| {
    watch {
        if load.is_pending() {
            div { "loading data..." }
        } else {
            // Clone the signals into outer-scope locals so each inner
            // closure can move its own copy. Signal clone is cheap
            // (Rc-backed).
            let loading_inner = loading.clone();
            let error_inner = error.clone();
            div {
                watch {
                    if loading_inner.get() { div { "submitting..." } }
                    else if error_inner.get().is_some() {
                        div { class: "alert", { error_inner.get().unwrap_or_default() } }
                    }
                    else { div { "ready" } }
                }
            }
        }
    }
})(load, loading, error)
```

## Why is this not auto-fixed by the macro?

In principle, the `watch { }` lowering could detect identifier captures
shared between outer and inner watches and inject `.clone()` calls
automatically. The macro does not currently do this because:

1. Reliable detection requires **type information** — the macro needs to
   know that a given identifier is a `Signal<T>` (or otherwise `Clone`)
   to inject a `.clone()`. Proc macros run before type-checking, so this
   detection would either rely on textual heuristics (fragile) or on a
   custom helper trait + blanket impl (which introduces a new public
   surface that itself can fail in surprising ways).
2. The flatten alternative is almost always preferable — nested
   `Page::reactive` invocations create separate reactive scopes that
   subscribe to overlapping signal sets, which can cause double
   re-renders.

Auto-cloning may be revisited if the flatten guidance proves insufficient
for real-world cases; for now the diagnostic note (see below) plus this
document are the canonical fix path.

## What changed in the framework as a result of #4515

`#4515` originally proposed three options: (1) auto-clone in the macro
codegen, (2) attach `#[diagnostic::on_unimplemented]` to the
`Page::reactive` bound, or (3) docs + doctest. Empirical testing showed
that option (2) does **not** fire for this case: the underlying compiler
error is a borrow-check error (`E0507`), not a trait-bound error, and
`#[diagnostic::on_unimplemented]` only emits notes for the latter. The
trait-wrapping approach was prototyped and removed for that reason.

What ended up landing under #4515:

- An expanded rustdoc on `Page::reactive` that names the footgun
  ("Nested `watch { }` footgun") and points at both fixes.
- This document, referenced from that rustdoc.
- A trybuild fail test
  (`crates/reinhardt-pages/tests/ui/page/fail/reactive_nested_non_copy_capture.rs`)
  that pins the rustc-builtin E0507 diagnostic for the canonical case.
  If a future rustc release silences or weakens the clone suggestion,
  the test fixture diff catches it and we can decide whether to compensate
  framework-side.

The accepted closure set for `Page::reactive` is unchanged — the bound
remains `F: Fn() -> Page + 'static` — so this is a docs-and-tests change
with zero SemVer impact.
