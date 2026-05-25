# Upstream workaround

Crates that use `#[viewset]` (impl-form) currently need the crate-level
allow below until
[rust-lang/rust#52234](https://github.com/rust-lang/rust/issues/52234)
(`macro_expanded_macro_exports_accessed_by_absolute_paths`)
resolves upstream — the macro expands to `pub use` paths whose
absolute-path lookup triggers a `future-incompat` warning:

```rust,ignore
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
```

The allow can be removed once the upstream issue closes. Tracked in
[reinhardt-web#4546](https://github.com/kent8192/reinhardt-web/issues/4546).
