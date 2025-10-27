# Documentation Standards

## Purpose

This document defines documentation maintenance standards for the Reinhardt project, ensuring documentation stays synchronized with code changes.

---

## Core Principles

### DM-1 (MUST): Documentation Updates with Code Changes

**ALWAYS** update relevant documentation when implementing or modifying features.

**Documentation updates MUST be done in the same workflow as the code changes.**

**DO NOT** leave documentation outdated after code modifications.

**Example Workflow:**
```
1. Implement feature
2. Update relevant docs in the SAME session
3. Verify docs match implementation
4. Submit both code and docs together
```

❌ **BAD:**
```
Day 1: Implement async fetch_data()
Day 2: (Documentation still says sync)
Day 3: User asks: "Why doesn't it match docs?"
Day 4: Fix documentation
```

✅ **GOOD:**
```
Session 1:
1. Implement async fetch_data()
2. Update README.md usage section
3. Update crate documentation
4. Update API reference
5. Submit all changes together
```

---

## Documentation Locations

### DM-2 (MUST): Documentation Locations

When modifying features, check and update the following documentation as applicable:

#### Project-Level Documentation
**File:** `README.md`
**Contents:**
- Project overview
- Installation instructions
- Quick start guide
- Main features (implemented only)
- Basic usage examples
- Links to detailed documentation

**When to Update:**
- Adding new major features
- Changing installation process
- Modifying project structure
- Updating dependencies

#### Crate-Level Documentation
**File:** `crates/<crate>/README.md` (if exists)
**Contents:**
- Crate-specific overview
- Crate features
- Usage examples
- API highlights

**File:** `crates/<crate>/src/lib.rs`
**Contents:**
- Module documentation (`//!`)
- Planned features section
- Architecture overview
- Code examples

**When to Update:**
- Adding new crate features
- Changing public API
- Modifying crate behavior
- Adding planned features

#### Detailed Guides
**Location:** `docs/` directory
**Files:**
- `docs/GETTING_STARTED.md` - Getting started guide
- `docs/FEATURE_FLAGS.md` - Feature flags documentation
- `docs/tutorials/` - Tutorial files
- `docs/MODULE_SYSTEM.md` - Module system standards
- `docs/TESTING_STANDARDS.md` - Testing standards
- `docs/ANTI_PATTERNS.md` - Anti-patterns guide
- Other relevant documentation files

**When to Update:**
- Adding new features requiring detailed explanation
- Changing established patterns
- Adding new standards or conventions
- Updating tutorials

---

## Documentation Consistency

### DM-3 (MUST): Documentation Consistency

Ensure consistency across all documentation levels (project, crate, docs/).

**Consistency Checklist:**
- [ ] Terminology is consistent across all docs
- [ ] Code examples use the same style
- [ ] Version numbers match
- [ ] Links are valid and point to correct locations
- [ ] Examples actually work with current code
- [ ] API signatures match implementation

**Example - Consistent Terminology:**
```markdown
<!-- ✅ GOOD: Consistent across all docs -->
README.md:        "Use QueryBuilder to construct SQL queries"
crate README:     "QueryBuilder provides type-safe query construction"
lib.rs:           "QueryBuilder is the main interface for building queries"
docs/tutorial:    "Create queries with QueryBuilder"

<!-- ❌ BAD: Inconsistent terminology -->
README.md:        "Use QueryBuilder to construct SQL queries"
crate README:     "The QueryConstructor provides query building"
lib.rs:           "Use QueryMaker for database queries"
docs/tutorial:    "Build queries with the SqlBuilder"
```

---

## Documentation Scope

### DM-4 (SHOULD): Documentation Scope

Update documentation for:

#### New Features
- Add feature descriptions
- Add usage examples
- Add API references
- Add to feature list in README

**Example:**
```rust
// New feature implemented
pub fn validate_email(email: &str) -> Result<()> {
    // ...
}
```

Update in same session:
```markdown
<!-- README.md -->
## Features
- Email validation ✅

<!-- crates/reinhardt-validators/README.md -->
### Email Validation
Validate email addresses with RFC 5322 compliance.

<!-- crates/reinhardt-validators/src/lib.rs -->
//! ## Email Validation
//!
//! ```rust
//! use reinhardt_validators::validate_email;
//!
//! assert!(validate_email("user@example.com").is_ok());
//! ```
```

#### Modified Features
- Update affected sections to reflect changes
- Update code examples
- Update API signatures
- Add migration notes if breaking changes

**Example:**
```rust
// Changed from sync to async
pub async fn fetch_data() -> Result<Data> {
    // ...
}
```

Update immediately:
```markdown
<!-- Before -->
## Usage
```rust
let data = fetch_data()?;
```

<!-- After -->
## Usage
```rust
let data = fetch_data().await?;
```

**Migration Note:** `fetch_data()` is now async. Update calls to use `.await`.
```

#### Deprecated Features
- Mark as deprecated
- Provide migration guides
- Set removal timeline

**Example:**
```rust
#[deprecated(since = "0.2.0", note = "Use new_function instead")]
pub fn old_function() {
    // ...
}
```

Update docs:
```markdown
## Deprecated Functions

### `old_function()`
**Deprecated since:** 0.2.0
**Removed in:** 0.3.0
**Migration:** Use `new_function()` instead

```rust
// Before
old_function();

// After
new_function();
```
```

#### Removed Features
- Remove documentation
- Add migration notes if necessary
- Update examples to remove references

---

## Documentation Quality

### DM-5 (MUST): Documentation Quality

Ensure high-quality documentation:

#### Examples Must Work
All code examples in documentation must be tested and working.

**Use Doc Tests:**
```rust
/// Validates an email address.
///
/// # Examples
///
/// ```
/// use reinhardt_validators::validate_email;
///
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid").is_err());
/// ```
pub fn validate_email(email: &str) -> Result<()> {
    // ...
}
```

**Test Documentation:**
```bash
cargo test --doc  # Runs all doc tests
```

#### Update Code Snippets
Keep code snippets current with API changes.

❌ **BAD - Outdated:**
```markdown
## Usage
```rust
let pool = Pool::new("postgres://localhost");  // ❌ Old API
```
```

✅ **GOOD - Current:**
```markdown
## Usage
```rust
let pool = Pool::builder()
    .url("postgres://localhost")
    .max_connections(10)
    .build()?;  // ✅ Current API
```
```

#### Verify Links
Ensure all links and references are valid.

**Check Internal Links:**
```markdown
See [Module System](docs/MODULE_SYSTEM.md)  ✅
See [Module System](docs/MODULES.md)        ❌ Wrong file
```

**Check External Links:**
```markdown
[Rust Book](https://doc.rust-lang.org/book/)  ✅
[Rust Book](https://rustbook.com/)            ❌ Wrong URL
```

#### Maintain Terminology
Use consistent terminology and formatting.

**Standardized Terms:**
- "QueryBuilder" (not "Query Builder", "QueryConstructor", "query builder")
- "ViewSet" (not "View Set", "Viewset", "view set")
- "Serializer" (not "Serialiser" in UK English context)

---

## Planned Features Location

### DM-6 (MUST): Planned Features Location

**Planned Features MUST be documented in the crate's `lib.rs` file header.**

**DO NOT include Planned Features sections in README.md files.**

**Format in `lib.rs`:**
```rust
//! # Crate Name
//!
//! Brief description of what this crate does.
//!
//! ## Features
//!
//! - Feature 1: Description of implemented feature
//! - Feature 2: Description of implemented feature
//!
//! ## Planned Features
//!
//! - Feature 3: Description of planned feature
//! - Feature 4: Description of planned feature
//!
//! ## Examples
//!
//! ```rust
//! // Example code
//! ```
```

**README.md Focus:**
```markdown
# Crate Name

Brief description.

## Features

- Feature 1: Description of implemented feature ✅
- Feature 2: Description of implemented feature ✅

<!-- ❌ NO Planned Features section here -->

## Usage

...
```

**Why?**
- Keeps planned features close to implementation code
- Better visibility during development
- README focuses on what's available NOW
- Reduces user confusion about what's actually implemented

**When Feature is Implemented:**
1. Remove from "Planned Features" in lib.rs
2. Add to "Features" section in lib.rs
3. Update README.md if it's a major feature
4. Update relevant documentation

---

## Documentation Workflow

### Standard Documentation Update Process

When implementing or modifying a feature:

```
1. ✅ Implement the code
2. ✅ Update lib.rs documentation
3. ✅ Update README.md if needed
4. ✅ Update crate README if exists
5. ✅ Update docs/ files if relevant
6. ✅ Run doc tests: cargo test --doc
7. ✅ Build docs: cargo doc --no-deps --open
8. ✅ Verify examples work
9. ✅ Check links are valid
10. ✅ Submit code + docs together
```

### Documentation Review Checklist

Before submitting:

- [ ] All relevant documentation files updated
- [ ] Code examples tested and working
- [ ] API signatures match implementation
- [ ] Terminology consistent across all docs
- [ ] Links are valid
- [ ] Formatting is correct
- [ ] No outdated information
- [ ] Planned features in lib.rs, not README
- [ ] Migration guides for breaking changes
- [ ] Doc tests pass

---

## Examples

### Example 1: Adding a New Feature

**Code Implementation:**
```rust
// crates/reinhardt-validators/src/email.rs
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    // Implementation
}
```

**Documentation Updates in Same Session:**

**1. lib.rs:**
```rust
//! # reinhardt-validators
//!
//! Validation utilities for Reinhardt.
//!
//! ## Features
//!
//! - Email validation with RFC 5322 compliance ✅ NEW
//! - Phone number validation
//!
//! ## Examples
//!
//! ```rust
//! use reinhardt_validators::validate_email;
//!
//! assert!(validate_email("user@example.com").is_ok());
//! assert!(validate_email("invalid").is_err());
//! ```
```

**2. README.md:**
```markdown
## Features

- Email validation with RFC 5322 compliance ✅ NEW
- Phone number validation
```

**3. docs/tutorials/validation.md:**
```markdown
### Email Validation

Validate email addresses:

```rust
use reinhardt_validators::validate_email;

let result = validate_email("user@example.com");
assert!(result.is_ok());
```
```

### Example 2: Changing API (Breaking Change)

**Code Change:**
```rust
// Before
pub fn connect(url: &str) -> Connection {
    // ...
}

// After (breaking change - returns Result)
pub fn connect(url: &str) -> Result<Connection> {
    // ...
}
```

**Documentation Updates:**

**1. lib.rs:**
```rust
//! ## Breaking Changes in 0.2.0
//!
//! - `connect()` now returns `Result<Connection>` instead of `Connection`
//!
//! ```rust
//! // Before
//! let conn = connect("postgres://localhost");
//!
//! // After
//! let conn = connect("postgres://localhost")?;
//! ```
```

**2. README.md:**
```markdown
## Migration from 0.1.x to 0.2.0

### `connect()` now returns Result

```rust
// Before (0.1.x)
let conn = connect("postgres://localhost");

// After (0.2.0)
let conn = connect("postgres://localhost")?;
```
```

**3. CHANGELOG.md:**
```markdown
## [0.2.0] - 2025-01-XX

### Breaking Changes

- `connect()` now returns `Result<Connection>` for better error handling
```

---

## Quick Reference

### Documentation Files to Check

When making changes, consider updating:

- [ ] `README.md` - Project overview
- [ ] `crates/<crate>/README.md` - Crate overview
- [ ] `crates/<crate>/src/lib.rs` - Module docs, planned features
- [ ] `docs/GETTING_STARTED.md` - Getting started guide
- [ ] `docs/FEATURE_FLAGS.md` - Feature flags
- [ ] `docs/tutorials/` - Relevant tutorials
- [ ] `CHANGELOG.md` - Version changelog

### Documentation Rules Summary

**✅ DO:**
- Update docs in same workflow as code
- Place planned features in lib.rs
- Test all code examples
- Keep terminology consistent
- Verify all links
- Add migration guides for breaking changes

**❌ DON'T:**
- Leave docs outdated
- Put planned features in README.md
- Include untested code examples
- Use inconsistent terminology
- Leave broken links
- Make breaking changes without migration guides

---

## Related Documentation

- Main standards: @CLAUDE.md
- Module system: @docs/MODULE_SYSTEM.md
- Testing standards: @docs/TESTING_STANDARDS.md
- Anti-patterns: @docs/ANTI_PATTERNS.md
