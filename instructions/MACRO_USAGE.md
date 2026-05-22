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

### MU-3 (SHOULD): Prefer `Model::build()` for Forward-Compatible Call Sites

Alongside the positional `Model::new(...)` constructor described in MU-2,
`#[model(...)]` also generates a typestate builder `Model::build()` (issue
#4400). Both entry points construct the same model; the builder trades a few
extra characters at the call site for the property that adding a required
field to the model becomes a **non-breaking** change for every caller that
used `build()`.

**When to prefer `build()`:**

- Tutorials, examples, and long-lived application code where the model schema
  is expected to evolve. Adding a new required field surfaces as a new setter
  rather than a new positional parameter that breaks every caller in
  lock-step.
- Call sites with three or more required fields where positional arguments
  start to obscure intent.
- Code that benefits from passing related models by reference: FK setters
  accept any `IntoPrimaryKey<Related>` value (see #4398), so
  `.author(&user)` is exactly as valid as `.author(user_id)`.

**When `new(...)` is still appropriate:**

- One-shot test fixtures and tight, internal call sites where positional
  arguments are unambiguous.
- Performance-sensitive hot paths (the builder is a thin compile-time
  abstraction, but `new(...)` is the most direct form).

**Examples:**

```rust
// ✅ Positional constructor — concise but order-sensitive.
let question = Question::new("What's your favorite color?".to_string());

// ✅ Typestate builder — each required field named, ordering free, and
// adding a new required field to `Question` keeps this call site compiling.
let question = Question::build()
    .question_text("What's your favorite color?")
    .finish();

// ✅ FK setter accepts `&User` directly (composes with #4398).
let choice = Choice::build()
    .choice_text("Red")
    .votes(0)
    .question(&question)
    .finish();
```

**Type-state guarantees:**

- Each required-field setter transitions exactly one slot from `Unset` to
  `Set`. Setters can be called in any order.
- `finish()` is only available when every required slot is `Set`. Calling
  `finish()` with any remaining required setter unused is a **compile-time
  error**, not a runtime panic.
- Optional fields (`Option<T>`, `default = ...`, `auto_now_add`, FK relation
  fields, identity / auto-increment primary keys) are filled in by
  `finish()` using the same expressions `new(...)` uses — no setter call is
  required.

**Rationale:**

- DESIGN_PHILOSOPHY #5 ("API ergonomics is paramount"): named setters scale
  with model size in a way positional arguments do not.
- DESIGN_PHILOSOPHY #9 ("Every framework eventually becomes outdated"):
  model schemas evolve; `build()` absorbs that evolution without breaking
  callers.
- DESIGN_PHILOSOPHY #4 ("Fail early"): the per-field type-state lifts
  "required field missing" from a runtime error into a compile error.

---

## Quick Reference

### ✅ MUST DO
- Initialize `#[model(...)]` structs via the macro-generated `new(...)` or `build()` constructor
- Add unrelated derives (e.g., `Debug`, `Clone`) via a separate `#[derive(...)]`

### ✅ SHOULD DO
- Use `#[model(...)]` alone (do not also write `#[derive(Model)]`) — the attribute applies the derive for you
- Prefer `Model::build()` over `Model::new(...)` in tutorials, examples, and call sites where the model schema is expected to evolve (MU-3)
- Pass FK values via `.<related>(&model)` in `build()` setters when the related instance is already in scope (composes with #4398)

### ❌ NEVER DO
- Initialize `#[model(...)]` structs via struct-literal syntax in production code (use `new(...)` or `build()`)

---

## Related Documentation

- **Module System**: instructions/MODULE_SYSTEM.md
- **Anti-Patterns**: instructions/ANTI_PATTERNS.md
- **Testing Standards**: instructions/TESTING_STANDARDS.md
