# Macro Usage Guidelines

## Purpose

This file defines the policy for using Reinhardt's procedural macros (notably `#[model(...)]`) consistently across the codebase.

---

## `#[model(...)]` Macro

### MU-1 (MUST): Do Not Combine with `#[derive(Model)]`

The `#[model(...)]` attribute macro automatically applies `#[derive(Model)]` internally. Combining the two produces duplicate derives and is rejected by the macro.

**Rule:**
- When using `#[model(...)]`, **do not** also write `#[derive(Model)]` on the same struct.
- Add other derives that `#[model(...)]` does not provide via a separate `#[derive(...)]` (e.g., `Debug`, `Clone`, `serde::Serialize`).

**Examples:**

```rust
// ✅ Correct
#[model(table = "people")]
#[derive(Debug, Clone)]
pub struct Person {
    pub id: i64,
    pub name: String,
}

// ❌ Incorrect — duplicate Model derive
#[model(table = "people")]
#[derive(Debug, Clone, Model)]
pub struct Person {
    pub id: i64,
    pub name: String,
}
```

### MU-2 (MUST): Initialize via the Macro-Generated Constructor

The `#[model(...)]` macro generates a `new(...)` associated function on the annotated struct. This generated constructor is the canonical initialization path.

**Rule:**
- Initialize `#[model(...)]` structs using the macro-generated `new(...)` function.
- Do not initialize them via struct literals (`Person { id: 0, name: ... }`) when the macro-generated `new` is available.
- Exception: Use struct literal syntax in tests *only when* the test specifically needs to bypass `new`'s validation (and document why with a comment).

**Examples:**

```rust
// ✅ Correct — use macro-generated constructor
let person = Person::new(1, "Alice".to_string());

// ❌ Incorrect — bypasses macro-generated initialization
let person = Person {
    id: 1,
    name: "Alice".to_string(),
};
```

**Rationale:**
- The generated `new` function applies any validation, defaulting, or invariant enforcement that the macro injects.
- Direct struct-literal initialization can silently bypass these guarantees, leading to invalid model instances.
- Centralizing initialization through `new` keeps call sites stable as the macro evolves.

---

## Quick Reference

### ✅ MUST DO
- Use `#[model(...)]` alone (do not also write `#[derive(Model)]`)
- Initialize `#[model(...)]` structs via the macro-generated `new(...)` function
- Add unrelated derives (e.g., `Debug`, `Clone`) via a separate `#[derive(...)]`

### ❌ NEVER DO
- Combine `#[model(...)]` with `#[derive(Model)]`
- Initialize `#[model(...)]` structs via struct-literal syntax in production code
- Re-implement validation logic that the macro already provides

---

## Related Documentation

- **Module System**: instructions/MODULE_SYSTEM.md
- **Anti-Patterns**: instructions/ANTI_PATTERNS.md
- **Testing Standards**: instructions/TESTING_STANDARDS.md
