# UI Tests

This directory contains compile-time tests for procedural macros using [`trybuild`](https://docs.rs/trybuild/).

## Directory Structure

```
tests/ui/
├── pass/                    # ✅ Tests that should compile successfully
│   └── *.rs
├── fail/                    # ❌ Tests that should fail to compile
│   ├── *.rs
│   └── *.stderr             # 📄 Expected compiler error messages
├── path/pass/               # Path macro: success cases
├── path/fail/               # Path macro: failure cases
├── permissions/pass/        # Permission macro: success cases
├── permissions/fail/        # Permission macro: failure cases
├── routes/pass/             # Routes macro: success cases
└── routes/fail/             # Routes macro: failure cases
```

## What are `.stderr` files?

`.stderr` files contain **expected compiler error messages** for tests in the `fail/` directories. When you run `cargo test`, `trybuild`:

1. Compiles the `.rs` test file
2. Captures the compiler error output
3. Compares it with the corresponding `.stderr` file
4. Fails the test if the error message doesn't match

**Why this extension?**
The `.stderr` extension is required by `trybuild` and follows Rust ecosystem conventions for "standard error output". This cannot be changed to a custom extension.

**Purpose:**

- Ensures macros produce clear, helpful error messages
- Prevents regressions in error message quality
- Documents expected failure modes

## Example

For a test file `missing_path.rs`:

```rust
// This should fail because path is missing
installed_apps! {
    auth:,  // ❌ Missing path value
}
```

The corresponding `missing_path.stderr` contains:

```
error: expected string literal
 --> tests/ui/fail/missing_path.rs:8:14
  |
8 |         auth:,
  |              ^
```

This ensures the macro produces a helpful error message pointing to the exact problem location.

## Adding New Tests

1. Create a new `.rs` file in the appropriate directory
2. For `fail/` tests, run `cargo test` to generate the `.stderr` file automatically
3. Review and commit both files

## Reference

- [trybuild documentation](https://docs.rs/trybuild/)
- [compile_tests.rs](../compile_tests.rs) - Test runner implementation