# Runtime SQL audit

This standalone, non-published Rust tool parses every tracked or non-ignored Rust
source and SQL asset in the repository. It does not use the currently enabled
Cargo features, so disabled backend paths remain visible. Rust syntax, cfg
conditions, module/impl/function identity and argument tokens are recorded.
Deliberately invalid compiler-test fixtures are recorded as test-only assets.

Run from the repository root:

```sh
cargo run --locked --manifest-path scripts/audit-runtime-sql/Cargo.toml -- scan "$PWD" > /tmp/runtime-sql-candidates.json
cargo run --locked --manifest-path scripts/audit-runtime-sql/Cargo.toml -- check "$PWD" "$PWD/docs/runtime-sql-inventory.json"
cargo run --locked --manifest-path scripts/audit-runtime-sql/Cargo.toml -- complete "$PWD" "$PWD/docs/runtime-sql-inventory.json"
```

`scan` includes SQL literals inside macros, SQL asset references, SQLx-style
query calls (including unqualified imports), execution wrapper calls, custom
expressions and possible inline query rendering. Detection is conservative:
HTTP/process `execute` methods may be candidates too. Classify their provenance
in the registry rather than changing detection to match today's callers.
AST-proven test-only code is shown in scan output but does not need an individual
runtime registry entry. Shipped testkit/test helper source is scanned normally.
In particular, `src/fixtures` and public `#[fixture]` functions remain in scope;
only fixtures under test-only source or an enclosing test cfg are excluded.

`check` rejects missing, changed, duplicate or stale sites and invalid workaround
metadata and missing nearby source `Workaround:` comments with the exact Issue URL.
Source line movement alone is allowed. `complete` also rejects every
pending framework-owned site. During incremental migration `pending` is a real
remaining obligation, never evidence of completion.

Identities use source path, module/impl/function, candidate kind and ordinal
within that symbol; line numbers are diagnostic only. Fingerprints cover
candidate syntax, its enclosing function body, or SQL asset contents. Changing
how an unchanged executor call obtains its SQL also requires provenance review.
Review semantic changes before updating an entry. Preserve existing
classifications and evidence for unchanged sites. Do not blindly refresh the
registry to pass CI.

Each runtime entry records ownership, status, operation category, SQL
representation, backend and feature reachability, rationale and evidence. Feature reachability
includes inherited module/Cargo gates after manual review; scanner cfg tokens
alone cannot establish it. A retained workaround additionally needs a tracking
Issue, backend reason, intended typed replacement and concrete removal condition. Typed entries
need focused SQL/argument and backend evidence. Application raw APIs must have a
reviewed caller-ownership rationale; no directory-wide raw SQL exception exists.

The parser cannot resolve Rust types, expand every macro, infer an external
module's inherited cfg, or prove caller ownership. Local cfg conditions and
syntactic facts are inputs to manual provenance/feature review. Inspect wrappers,
macro-generated paths and caller-to-executor flow before closing #5895. The
inventory's pending classifications retain that obligation explicitly.
