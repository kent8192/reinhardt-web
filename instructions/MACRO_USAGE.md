# Macro Usage Guidelines

## Purpose

This file defines the policy for using Reinhardt's procedural macros (notably `#[model(...)]`) consistently across the codebase.

---

## `#[model(...)]` Macro

### MU-1 (SHOULD): Do Not Combine with `#[derive(Model)]`

The `#[model(...)]` attribute macro automatically applies `#[derive(Model)]` internally. If `#[derive(Model)]` is *also* written on the same struct, the attribute macro detects this and returns the input unchanged (the existing derive then handles the macro logic). Compilation succeeds today, but the explicit `Model` derive becomes redundant noise that obscures intent.

**Rule:**
- When using `#[model(...)]`, **prefer not** to also write `#[derive(Model)]` on the same struct — the attribute applies it for you.
- Add other derives that `#[model(...)]` does not provide via a separate `#[derive(...)]` (e.g., `Debug`, `Clone`, `serde::Serialize`).
- Existing code that combines both is supported and does not need an immediate fix; new code should follow the canonical form.

**Examples:**

```rust
// ✅ Canonical — let #[model(...)] add the Model derive
#[model(table = "people")]
#[derive(Debug, Clone)]
pub struct Person {
    pub id: i64,
    pub name: String,
}

// ⚠️ Redundant but supported — Model derive duplicates what #[model(...)] applies
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
- Exception: struct-literal syntax may be used in tests when a test specifically needs to set fields that the constructor auto-fills (e.g., to inject a fixed primary key). Document the reason with a comment.

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
- The generated `new` accepts only the fields the user must supply and auto-fills macro-managed fields (auto-generated primary keys, foreign-key id columns, defaulted fields). Struct-literal initialization forces the caller to spell out every field, including those `new` would have filled — which is brittle.
- Centralizing initialization through `new` keeps call sites stable as the macro evolves: when future macro versions add fields or validation, struct-literal call sites break or silently bypass the new guarantees, while `new(...)` call sites adapt automatically.
- Today the generated `new` does not perform validation; this rule is about future-proofing and field coverage, not about a current invariant guarantee.

---

## Quick Reference

### ✅ MUST DO
- Initialize `#[model(...)]` structs via the macro-generated `new(...)` function
- Add unrelated derives (e.g., `Debug`, `Clone`) via a separate `#[derive(...)]`

### ✅ SHOULD DO
- Use `#[model(...)]` alone (do not also write `#[derive(Model)]`) — the attribute applies the derive for you

### ❌ NEVER DO
- Initialize `#[model(...)]` structs via struct-literal syntax in production code (use `new(...)`)

---

## Related Documentation

- **Module System**: instructions/MODULE_SYSTEM.md
- **Anti-Patterns**: instructions/ANTI_PATTERNS.md
- **Testing Standards**: instructions/TESTING_STANDARDS.md
