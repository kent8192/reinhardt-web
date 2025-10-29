# Anti-Patterns and What NOT to Do

## Purpose

This document explicitly lists common mistakes, anti-patterns, and practices to avoid in the Reinhardt project. Use this as a quick reference for code review and development.

---

## Code Organization Anti-Patterns

### ❌ Using `mod.rs` Files

**DON'T:**
```
src/database/mod.rs  // ❌ Old Rust 2015 style
```

**DO:**
```
src/database.rs      // ✅ Rust 2024 style
```

**Why?** `mod.rs` is deprecated and makes file navigation harder. See @docs/MODULE_SYSTEM.md

### ❌ Glob Imports

**DON'T:**
```rust
pub use database::*;  // ❌ Pollutes namespace
```

**DO:**
```rust
pub use database::{Pool, Connection, PoolConfig};  // ✅ Explicit
```

**Why?** Makes it unclear what's exported and causes naming conflicts.

### ❌ Circular Module Dependencies

**DON'T:**
```rust
// module_a.rs
use crate::module_b::TypeB;  // ❌ A → B

// module_b.rs
use crate::module_a::TypeA;  // ❌ B → A (circular!)
```

**DO:**
```rust
// types.rs - Extract common types
pub struct TypeA;
pub struct TypeB;

// module_a.rs
use crate::types::{TypeA, TypeB};  // ✅ No cycle

// module_b.rs
use crate::types::{TypeA, TypeB};  // ✅ No cycle
```

**Why?** Causes compilation errors and indicates poor module design.

### ❌ Excessive Flat Structure

**DON'T:**
```
src/
├── user_handler.rs      // ❌ Related files
├── user_service.rs      // scattered across
├── user_repository.rs   // the same level
├── auth_handler.rs
├── auth_service.rs
└── auth_repository.rs
```

**DO:**
```
src/
├── user.rs              // ✅ Grouped by
├── user/                // feature/domain
│   ├── handler.rs
│   ├── service.rs
│   └── repository.rs
├── auth.rs
└── auth/
    ├── handler.rs
    ├── service.rs
    └── repository.rs
```

**Why?** Grouping related files improves maintainability and navigation.

### ❌ Deep Relative Paths

**DON'T:**
```rust
use crate::../../config/settings.toml;  // ❌ Goes up 2+ levels
use super::super::super::utils;         // ❌ Confusing
```

**DO:**
```rust
use crate::config::Settings;  // ✅ Absolute from crate root
use super::sibling_module;    // ✅ One level up is OK
```

**Why?** Deep relative paths are hard to understand and maintain.

---

## Code Style Anti-Patterns

### ❌ Excessive `.to_string()` Calls

**DON'T:**
```rust
fn process_name(name: &str) -> String {
    let greeting = format!("Hello, {}", name.to_string());  // ❌ Unnecessary
    greeting.to_string()  // ❌ Already a String!
}
```

**DO:**
```rust
fn process_name(name: &str) -> String {
    format!("Hello, {}", name)  // ✅ name is already &str
}

// Or use Cow for conditional ownership
use std::borrow::Cow;

fn process_name(name: &str) -> Cow<str> {
    if name.is_empty() {
        Cow::Borrowed("Anonymous")
    } else {
        Cow::Borrowed(name)
    }
}
```

**Why?** Unnecessary allocations hurt performance. Prefer borrowing.

### ❌ Leaving Obsolete Code

**DON'T:**
```rust
// fn old_implementation() {  // ❌ Commented out code
//     // ...
// }

pub fn new_implementation() {
    // ...
}
```

**DO:**
```rust
pub fn new_implementation() {  // ✅ Old code deleted
    // ...
}
```

**Why?** Git history preserves old code. Commented code creates clutter.

### ❌ Deletion Record Comments

**DON'T:**
```rust
// Removed empty test: test_foo - This test was empty  // ❌ Don't document deletions
// Deleted: old_module.rs (deprecated)                  // ❌ Git history has this

pub fn active_function() {
    // ...
}
```

**DO:**
```rust
pub fn active_function() {  // ✅ No deletion comments
    // ...
}

// If important notes are needed, extract to docs/IMPLEMENTATION_NOTES.md
```

**Why?** Git history is the permanent record. Comments clutter the codebase.

### ❌ Using Alternative TODO Notations

**DON'T:**
```rust
// Implementation Note: This needs to be completed    // ❌ Custom notation
// FIXME: Add validation                              // ❌ Use TODO instead
// NOTE: Not implemented yet                          // ❌ NOTE is for info only
```

**DO:**
```rust
// TODO: Implement input validation logic
fn validate_input(data: &str) -> Result<()> {
    todo!("Add validation - planned for next sprint")
}

// Or for intentionally omitted features:
fn legacy_feature() {
    unimplemented!("This feature is intentionally not supported")
}
```

**Why?** Standardized notation (`TODO`, `todo!()`, `unimplemented!()`) is searchable and clear.

### ❌ Unmarked Placeholder Implementations

**DON'T:**
```rust
pub fn get_cache_config() -> CacheConfig {
    CacheConfig::default()  // ❌ Looks like production code!
}

pub fn send_email(to: &str, body: &str) -> Result<()> {
    println!("Would send: {}", body);  // ❌ Mock without marker
    Ok(())
}
```

**DO:**
```rust
pub fn get_cache_config() -> CacheConfig {
    todo!("Implement cache configuration loading from settings")
}

pub fn send_email(to: &str, body: &str) -> Result<()> {
    // TODO: Integrate with actual email service provider
    println!("Would send: {}", body);
    Ok(())
}
```

**Why?** Unmarked placeholders can be mistaken for production code.

---

## Testing Anti-Patterns

### ❌ Skeleton Tests

**DON'T:**
```rust
#[test]
fn test_user_creation() {
    // ❌ Empty test - always passes
}

#[test]
fn test_validation() {
    let result = validate_email("test@example.com");
    // ❌ No assertion - useless
}

#[test]
fn test_always_true() {
    assert!(true);  // ❌ Meaningless
}
```

**DO:**
```rust
#[test]
fn test_user_creation() {
    let user = User::new("Alice", "alice@example.com");
    assert_eq!(user.name, "Alice");  // ✅ Real assertion
}

#[test]
fn test_validation() {
    assert!(validate_email("test@example.com").is_ok());  // ✅ Tests behavior
    assert!(validate_email("invalid").is_err());
}
```

**Why?** Tests must be capable of failing. Skeleton tests provide no value.

### ❌ Tests Without Reinhardt Components

**DON'T:**
```rust
#[test]
fn test_standard_library() {
    let vec = vec![1, 2, 3];
    assert_eq!(vec.len(), 3);  // ❌ Only tests std library
}
```

**DO:**
```rust
use reinhardt_orm::QueryBuilder;

#[test]
fn test_query_building() {
    let query = QueryBuilder::new()  // ✅ Uses Reinhardt component
        .table("users")
        .build();
    assert_eq!(query.to_sql(), "SELECT * FROM users");
}
```

**Why?** Every test must verify at least one Reinhardt component.

### ❌ Integration Tests in Functional Crates

**DON'T:**
```
crates/reinhardt-orm/
├── src/
└── tests/
    └── with_serializers.rs  // ❌ Uses reinhardt-serializers (2 crates)
```

```toml
# crates/reinhardt-orm/Cargo.toml
[dev-dependencies]
reinhardt-serializers = { path = "../reinhardt-serializers" }  # ❌ NEVER
```

**DO:**
```
tests/                           // ✅ Integration tests here
└── integration/
    └── tests/
        └── orm_serializer_integration.rs
```

```toml
# tests/Cargo.toml
[dependencies]
reinhardt-orm = { path = "../crates/reinhardt-orm" }          # ✅ OK
reinhardt-serializers = { path = "../crates/reinhardt-serializers" }
```

**Why?** Integration tests (2+ crates) MUST be in the `tests/` crate.

### ❌ Tests Without Cleanup

**DON'T:**
```rust
#[test]
fn test_file_creation() {
    std::fs::write("/tmp/test_file.txt", "data").unwrap();
    // ❌ File left behind
}
```

**DO:**
```rust
#[test]
fn test_file_creation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "data").unwrap();
    // ✅ Cleanup happens automatically when temp_dir drops
}
```

**Why?** Test artifacts must be cleaned up.

### ❌ Global State Tests Without Serialization

**DON'T:**
```rust
#[test]  // ❌ No #[serial] - can conflict with other tests
fn test_i18n_activation() {
    activate("fr", catalog);
    assert_eq!(get_language(), "fr");
}
```

**DO:**
```rust
use serial_test::serial;

#[test]
#[serial(i18n)]  // ✅ Serialized with other i18n tests
fn test_i18n_activation() {
    activate("fr", catalog);
    assert_eq!(get_language(), "fr");
    deactivate();  // ✅ Cleanup
}
```

**Why?** Global state tests can conflict if run in parallel.

### ❌ Loose Assertions

**DON'T:**
```rust
#[test]
fn test_error_message() {
    let result = validate_input("");
    let error = result.unwrap_err();
    // ❌ Too permissive - could match unintended substrings
    assert!(error.to_string().contains("invalid"));
}

#[test]
fn test_calculation() {
    let result = calculate_discount(100, 10);
    // ❌ Should check exact value, not just range
    assert!(result > 0);
    assert!(result < 100);
}

#[test]
fn test_response_body() {
    let response = get_user_info();
    // ❌ Loose pattern matching
    assert!(response.contains("\"id\":"));
    assert!(response.contains("\"name\":"));
}
```

**DO:**
```rust
#[test]
fn test_error_message() {
    let result = validate_input("");
    let error = result.unwrap_err();
    // ✅ Exact error message verification
    assert_eq!(error.to_string(), "Input cannot be empty");
}

#[test]
fn test_calculation() {
    let result = calculate_discount(100, 10);
    // ✅ Exact value expected
    assert_eq!(result, 90);
}

#[test]
fn test_response_body() {
    let response = get_user_info();
    // ✅ Deserialize and check exact structure
    let user: UserInfo = serde_json::from_str(&response).unwrap();
    assert_eq!(user.id, 123);
    assert_eq!(user.name, "Alice");
}
```

**EXCEPTION - When Loose Assertions Are Acceptable:**
```rust
#[test]
fn test_generate_uuid() {
    let uuid = generate_uuid();
    // ✅ UUID is random, can only check format
    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid.chars().filter(|&c| c == '-').count(), 4);
}

#[test]
fn test_timestamp() {
    let before = SystemTime::now();
    let timestamp = get_current_timestamp();
    let after = SystemTime::now();
    // ✅ Timestamp is system-dependent, verified within bounds
    assert!(timestamp >= before);
    assert!(timestamp <= after);
}
```

**Why?** Loose assertions like `contains()` or range checks can pass with incorrect values. Strict assertions catch bugs that loose assertions would miss.

---

## File Management Anti-Patterns

### ❌ Saving Files to Project Directory

**DON'T:**
```bash
# Script execution
./analyze.sh > results.md          # ❌ Saved to project root
python3 process.py > output.txt    # ❌ Saved to project root
```

**DO:**
```bash
# Script execution
./analyze.sh > /tmp/results.md     # ✅ Use /tmp
python3 process.py > /tmp/output.txt

# Delete when done
rm /tmp/results.md /tmp/output.txt
```

**Why?** Keeps project directory clean. Prevents accidental commits.

### ❌ Leaving Backup Files

**DON'T:**
```bash
# After editing
ls
file.rs
file.rs.bak        # ❌ Backup file left behind
config.toml.old    # ❌ Old version not deleted
script.sh~         # ❌ Temporary backup
```

**DO:**
```bash
# Clean up immediately
rm file.rs.bak config.toml.old script.sh~  # ✅ Delete backups
```

**Why?** Backup files clutter the codebase and can be accidentally committed.

### ❌ Not Cleaning Up /tmp Files

**DON'T:**
```bash
# Create temp files
echo "data" > /tmp/analysis_results.md
./process.sh > /tmp/output.txt

# ... do work ...

# ❌ Never delete them
```

**DO:**
```bash
# Create temp files
echo "data" > /tmp/analysis_results.md
./process.sh > /tmp/output.txt

# ... do work ...

# ✅ Clean up when done
rm /tmp/analysis_results.md /tmp/output.txt
```

**Why?** Prevents /tmp accumulation and ensures clean environment.

---

## Workflow Anti-Patterns

### ❌ Committing Without User Instruction

**DON'T:**
```bash
# ❌ AI creates commit automatically
git add .
git commit -m "feat: Add feature"
```

**DO:**
```bash
# ✅ Wait for explicit user instruction
# User: "Please commit these changes"
git add <specific files>
git commit -m "..."
```

**Why?** Commits should only be made with explicit user authorization.

### ❌ Batch Operations Without Dry-Run

**DON'T:**
```bash
# ❌ Bulk replace without verification
sed -i 's/old_pattern/new_pattern/g' **/*.rs
```

**DO:**
```bash
# ✅ Create dry-run script first
cat > /tmp/dryrun.sh << 'EOF'
grep -r "old_pattern" --include="*.rs" .
EOF

bash /tmp/dryrun.sh  # Review scope

# Only proceed after confirmation
cat > /tmp/replace.sh << 'EOF'
sed -i 's/old_pattern/new_pattern/g' **/*.rs
EOF

bash /tmp/replace.sh
rm /tmp/dryrun.sh /tmp/replace.sh
```

**Why?** Dry-run prevents unintended bulk changes.

### ❌ Monolithic Commits

**DON'T:**
```bash
# ❌ One huge commit for entire feature
git add .
git commit -m "feat(auth): Implement authentication feature"
# Changes: JWT, sessions, OAuth, password hashing, middleware, tests...
```

**DO:**
```bash
# ✅ Split into specific intents
git add src/auth/password.rs
git commit -m "feat(auth): Implement bcrypt password hashing"

git add src/auth/jwt.rs
git commit -m "feat(auth): Add JWT token generation with RS256"

git add src/auth/session.rs
git commit -m "feat(auth): Create session storage middleware"
```

**Why?** Small, focused commits make history easier to understand and review.

---

## Documentation Anti-Patterns

### ❌ Outdated Documentation After Code Changes

**DON'T:**
```rust
// Code changes from sync to async
pub async fn fetch_data() -> Result<Data> {
    // ...
}
```

```markdown
<!-- README.md still says: -->
## Usage
Use `fetch_data()` synchronously...  // ❌ Outdated!
```

**DO:**
```rust
pub async fn fetch_data() -> Result<Data> {
    // ...
}
```

```markdown
<!-- README.md updated: -->
## Usage
Use `fetch_data().await` asynchronously...  // ✅ Current!
```

**Why?** Documentation must be updated with code changes in the same workflow.

### ❌ Planned Features in README

**DON'T:**
```markdown
<!-- README.md -->
## Features
- User authentication ✅
- Database migrations ✅

### Planned Features
- GraphQL support
- WebSockets
```

**DO:**
```rust
//! crates/reinhardt-api/src/lib.rs
//!
//! ## Planned Features
//!
//! - GraphQL support
//! - WebSockets
```

```markdown
<!-- README.md - Only implemented features -->
## Features
- User authentication ✅
- Database migrations ✅
```

**Why?** Planned features belong in `lib.rs`, README shows implemented features only.

---

## Quick Reference

### Top Anti-Patterns to Avoid

**Module System:**
- ❌ Using `mod.rs` files
- ❌ Glob imports (`use module::*`)
- ❌ Circular dependencies
- ❌ Deep relative paths (`../../..`)

**Code Style:**
- ❌ Excessive `.to_string()` calls
- ❌ Commented-out code
- ❌ Deletion record comments
- ❌ Unmarked placeholders

**Testing:**
- ❌ Skeleton tests (no assertions)
- ❌ Tests without Reinhardt components
- ❌ Integration tests in functional crates
- ❌ No test cleanup
- ❌ Global state without `#[serial]`
- ❌ Loose assertions (`contains`, range checks) without justification

**File Management:**
- ❌ Saving to project directory (use /tmp)
- ❌ Leaving backup files (.bak, .old, ~)
- ❌ Not cleaning up /tmp files

**Workflow:**
- ❌ Committing without user instruction
- ❌ Bulk operations without dry-run
- ❌ Monolithic commits

**Documentation:**
- ❌ Outdated docs after code changes
- ❌ Planned features in README

---

## Related Documentation

- Main standards: @CLAUDE.md
- Module system: @docs/MODULE_SYSTEM.md
- Testing standards: @docs/TESTING_STANDARDS.md
- Documentation standards: @docs/DOCUMENTATION_STANDARDS.md
