# API Stability Policy

This document defines the API stability policy for the Reinhardt project, describing
version scheme, stability levels, breaking change policy, and deprecation procedures.

---

## Version Scheme

Reinhardt uses [Semantic Versioning 2.0.0](https://semver.org/) for all published crates.

### Pre-Release Labels

| Label | Description | Stability Guarantee |
|-------|-------------|---------------------|
| `alpha` (0.x.y) | Initial development | No guarantees; anything may change |
| `rc` (x.y.z-rc.N) | Release candidate | Feature-complete; bug fixes only |
| `stable` (x.y.z) | General availability | Full SemVer guarantees apply |

### Version Bump Rules

| Commit Type | Version Change |
|-------------|----------------|
| `feat!:` or `BREAKING CHANGE:` | MAJOR bump |
| `feat:` | MINOR bump |
| `fix:`, `perf:`, `docs:`, others | PATCH bump |

---

## API Categories

### Stable API

Public items not marked with `#[doc(hidden)]` or `#[unstable]` are considered **stable**:

- All items in the primary re-exports of each crate's `lib.rs`
- Trait definitions and their required methods
- Struct field accessibility (public fields)
- Enum variant names

**Guarantee**: No breaking changes without a MAJOR version bump.

### Experimental API

Items marked with `#[doc(cfg(feature = "unstable"))]` or documented as experimental are
**experimental** and may change in MINOR releases:

- New traits under active development
- Extension points that may be redesigned
- Performance-sensitive APIs pending benchmarking

**Guarantee**: Breaking changes require documenting migration paths.

### Internal API

Items marked with `#[doc(hidden)]`, starting with `__`, or not publicly accessible are
**internal** and provide no stability guarantees:

- Macro implementation helpers
- Proc-macro infrastructure
- `pub(crate)` and `pub(super)` items

**Guarantee**: None. May change in any release.

---

## Breaking Change Policy

### Definition

A breaking change is any modification that causes downstream code compiled against the
previous version to fail compilation or to exhibit different runtime behavior when recompiled
against the new version.

### Common Breaking Changes

- Removing or renaming a public item
- Changing function signatures (parameters, return types)
- Adding required methods to a public trait
- Changing enum variants without `#[non_exhaustive]`
- Adding fields to non-`#[non_exhaustive]` structs
- Narrowing trait bounds on public functions

### Breaking Change Approval Process

1. Open an RFC issue with title `[RFC]: <description>` labeling it `enhancement`
2. Final Comment Period (FCP) of at least 7 days for community feedback
3. Breaking changes require MAJOR version bump
4. Migration guide must be provided (see Migration Guide Requirements below)

### `#[non_exhaustive]` as a Preventative Measure

All public error enums and configuration structs are marked `#[non_exhaustive]` to allow
adding new variants/fields in MINOR releases without breaking downstream exhaustive matches
or struct literal initializations.

This means:

- `match` expressions on error enums must include a `_ =>` wildcard arm
- Struct literals for config structs must use `..Default::default()` or builder methods

Example for error enums:
```rust
match error {
    MyError::NotFound => handle_not_found(),
    MyError::PermissionDenied => handle_denied(),
    _ => handle_unknown(), // Required for #[non_exhaustive] enums
}
```

Example for config structs:
```rust
// Use builder methods (preferred):
let config = MyConfig::new()
    .with_timeout(Duration::from_secs(30));

// Or use struct update syntax:
let config = MyConfig {
    timeout: Duration::from_secs(30),
    ..MyConfig::default()
};
```

---

## Deprecation Policy

### Deprecation Process

1. Mark the item with `#[deprecated(since = "X.Y.Z", note = "Use <replacement> instead.")]`
2. Add the `deprecated` commit type to the changelog entry
3. Keep the deprecated item for at least one MINOR release
4. Remove in the next MAJOR release with a documented migration path

### Deprecation Attribute Format

```rust
#[deprecated(
    since = "0.3.0",
    note = "Use `new_function` instead. This function will be removed in 1.0.0."
)]
pub fn old_function() { ... }
```

Requirements:
- `since` field: version when deprecation was introduced
- `note` field: specific replacement and removal timeline

---

## Migration Guide Requirements

Every breaking change (MAJOR release) and every significant deprecation must include:

1. **What changed**: Clear description of the old vs. new behavior
2. **Why it changed**: Technical rationale for the breaking change
3. **How to migrate**: Step-by-step migration instructions with code examples
4. **Timeline**: When the old behavior will be fully removed (if applicable)

Migration guides are placed in `docs/migrations/vX.Y.Z-migration.md`.

---

## Continuous SemVer Verification

Automated SemVer checking is performed on every pull request targeting `main` using
[`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks).

The CI workflow at `.github/workflows/semver-check.yml` reports any detected SemVer
violations before code is merged. A full breaking change audit is maintained at
`docs/breaking-change-audit.md`.

---

## References

- [Semantic Versioning 2.0.0](https://semver.org/)
- [RFC 1105: API Evolution](https://rust-lang.github.io/rfcs/1105-api-evolution.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Breaking Change Audit](breaking-change-audit.md)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
