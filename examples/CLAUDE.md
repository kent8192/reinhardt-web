# Examples - Project-Specific Instructions

## Purpose

This file defines coding standards and dependency management rules specific to the `examples/` directory. These rules ensure examples demonstrate best practices for using Reinhardt.

---

## Examples Directory Structure

The `examples/` directory contains two types of example projects that serve different purposes:

### Local Examples (`examples/local/`)

**Purpose**: Development and testing with the latest workspace implementation

**Key Characteristics**:
- Always uses local workspace version via `[patch.crates-io]`
- Build script always enables tests with `with-reinhardt` feature
- For development, testing, and validating latest changes
- Dependencies: workspace-local `reinhardt` crate

**Dependency Resolution**:
```toml
[dependencies]
# Default uses 'full' feature (all features enabled)
reinhardt = { path = "../../..", package = "reinhardt-web" }

# Or with explicit feature selection:
# reinhardt = { path = "../../..", package = "reinhardt-web", features = ["standard"] }
# reinhardt = { path = "../../..", package = "reinhardt-web", default-features = false, features = ["minimal"] }

[patch.crates-io]
reinhardt-web = { path = "../../.." }  # Always uses workspace version
```

**Note**: ⚠️ Default feature changed from `standard` to `full`. Local examples now use all features by default for comprehensive testing.

**Build Configuration**:
```rust
// build.rs
fn main() {
	// Always enable tests for local development
	println!("cargo:rustc-cfg=with_reinhardt");
}
```

**Use Cases**:
- ✅ Testing new features during development
- ✅ Validating API changes before publication
- ✅ Internal framework development workflow
- ❌ User onboarding (use `remote` instead)

---

### Remote Examples (`examples/remote/`)

**Purpose**: Validation and user onboarding with published crates.io versions

**Key Characteristics**:
- Uses published crates.io versions (no `[patch.crates-io]`)
- Tests are enabled conditionally based on published version
- For validating published versions and user learning
- Dependencies: crates.io `reinhardt` crate

**Dependency Resolution**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", package = "reinhardt-web", features = ["standard"] }

# NO [patch.crates-io] section
```

**Build Configuration**:
```rust
// build.rs with #[example_test] macro support
fn main() {
	let current_version = "0.1.0-alpha.1";

	// Check if the version exists on crates.io
	if version_exists(current_version) {
		println!("cargo:rustc-cfg=with_reinhardt");
	}
	// Tests are skipped if version not published
}
```

**Version-Specific Testing**:
```rust
#[example_test(version = "0.1.0-alpha.1")]
fn test_feature() {
	// Only runs if version 0.1.0-alpha.1 is published
}
```

**Use Cases**:
- ✅ User onboarding and tutorials
- ✅ Validating published releases
- ✅ Testing compatibility with specific versions
- ❌ Development of unreleased features (use `local` instead)

---

### Shared Utilities

Both local and remote examples share common utilities:

**Location**: `examples/remote/common/` and `examples/remote/test-macros/`

**Shared Components**:
- `example-common`: Common test utilities and helpers
- `example-test-macros`: Test macros (`#[example_test]` for version-specific tests)

**Usage in Local Examples**:
```toml
[dev-dependencies]
example-test-macros = { path = "../remote/test-macros" }
example-common = { path = "../remote/common" }
```

**Usage in Remote Examples**:
```toml
[dev-dependencies]
example-test-macros = { path = "../../remote/test-macros" }
example-common = { path = "../../remote/common" }
```

---

### Comparison Table

| Aspect | Local Examples | Remote Examples |
|--------|---------------|-----------------|
| **Dependency Source** | Workspace (`path = "../../.."`) | crates.io (`version = "..."`) |
| **`[patch.crates-io]`** | ✅ Always uses workspace | ❌ No patching |
| **Test Enablement** | Always enabled | Conditional (version-specific) |
| **Build Config** | Unconditional `with_reinhardt` | Checks crates.io availability |
| **Purpose** | Development & latest validation | User onboarding & version validation |
| **Typical Users** | Framework developers | End users learning Reinhardt |
| **Update Frequency** | With every workspace change | With new crate publications |

---

## Dependency Management

### DM-1 (MUST): Reinhardt Dependencies Only

**ALL** example projects MUST include ONLY the `reinhardt` crate in their dependencies.

**NEVER** include Reinhardt sub-crates (`reinhardt-*`) directly in:
- `[dependencies]`
- `[dev-dependencies]`
- `[build-dependencies]`

#### ✅ CORRECT Pattern

```toml
[dependencies]
# ✅ Main reinhardt crate only
reinhardt = { path = "../../..", package = "reinhardt-web", features = ["core", "database"] }

# ✅ External crates are fine
tokio = { workspace = true }
serde = { workspace = true }

[dev-dependencies]
# ✅ External test crates are fine
rstest = "0.23"
example-test-macros = { path = "../../remote/test-macros" }
```

#### ❌ INCORRECT Pattern

```toml
[dependencies]
reinhardt = { path = "../../..", package = "reinhardt-web", features = ["core"] }
reinhardt-http = { path = "../../../crates/reinhardt-http" }      # ❌ NEVER
reinhardt-routers = { path = "../../../crates/reinhardt-urls/crates/routers" }  # ❌ NEVER
reinhardt-di = { path = "../../../crates/reinhardt-di" }          # ❌ NEVER
reinhardt-orm = { path = "../../../crates/reinhardt-db/crates/orm" }  # ❌ NEVER

[dev-dependencies]
reinhardt-http = { path = "../../../crates/reinhardt-http" }      # ❌ NEVER (sub-crate dependency)
```

### DM-2 (MUST): Import from reinhardt Crate Only

**ALL** imports MUST use the `reinhardt` crate, NOT sub-crates.

#### ✅ CORRECT Import Patterns

```rust
// Pattern 1: Use prelude for common types
use reinhardt::prelude::*;

// Pattern 2: Explicit imports from reinhardt
use reinhardt::{Request, Response, StatusCode};
use reinhardt::{Method, UnifiedRouter};
use reinhardt::endpoint;

// Pattern 3: Module-qualified imports
use reinhardt::db::orm::Manager;
use reinhardt::db::DatabaseConnection;

// Pattern 4: External dependencies via reinhardt re-exports
use reinhardt::core::serde::{Serialize, Deserialize};
use reinhardt::core::serde::json::json;
use reinhardt::core::async_trait;
```

#### ❌ INCORRECT Import Patterns

```rust
// ❌ NEVER import from sub-crates directly
use reinhardt_http::{Request, Response};
use reinhardt_routers::UnifiedRouter;
use reinhardt_macros::endpoint;
use reinhardt_orm::Manager;
use reinhardt_db::DatabaseConnection;
use reinhardt_test::fixtures::postgres_container;

// ❌ NEVER import from hyper directly (use reinhardt re-exports)
use hyper::{Method, StatusCode};

// ❌ NEVER import external dependencies directly when re-exports exist
use serde::{Serialize, Deserialize};  // Use reinhardt::core::serde instead
use serde_json::json;                 // Use reinhardt::core::serde::json::json instead
use async_trait::async_trait;         // Use reinhardt::core::async_trait instead
```

---

## Why These Rules?

### 1. **Demonstrates Correct Usage**
Examples should show users the **recommended way** to use Reinhardt, not internal implementation details.

### 2. **Maintains Facade Pattern**
Reinhardt uses a facade pattern where the main crate re-exports commonly used types. Direct sub-crate dependencies break this abstraction.

### 3. **Prevents Breaking Changes**
Sub-crates are internal implementation details that may change. The `reinhardt` crate provides a stable API surface.

### 4. **Consistency with Documentation**
README.md and other documentation recommend using `reinhardt::prelude::*` and `use reinhardt::{...}`. Examples must match this guidance.

---

## Available Re-Exports

### Always Available (from src/lib.rs)

```rust
use reinhardt::{
	// HTTP & Routing
	StatusCode, Method,
	Request, Response,
	UnifiedRouter, Router, DefaultRouter,

	// Macros
	endpoint,
	installed_apps,

	// Settings
	Settings, SettingsBuilder,

	// View Classes
	View, ListView, DetailView,
	ViewSet, ModelViewSet, ReadOnlyModelViewSet,

	// View Result Type
	ViewResult,
};
```

### Feature-Dependent Re-Exports

```rust
// With feature = "core"
use reinhardt::{Handler, Middleware};

// With feature = "database"
use reinhardt::db::{
	DatabaseConnection,
	orm::Manager,
	orm::Model,
	orm::{F, Q, Transaction},
	migrations::Migration,  // ✅ Correct: module-qualified only
};

// ❌ NOT available at top level
// use reinhardt::Migration;  // This does NOT compile

// With feature = "di"
use reinhardt::{Body, Cookie, Header, Json, Path, Query};

// With feature = "test"
use reinhardt::test::{
	client::APIClient,
	fixtures::test_server_guard,
	resource::TeardownGuard,
};
```

### Prelude Module (use reinhardt::prelude::*)

```rust
use reinhardt::prelude::*;

// This imports:
// - UnifiedRouter, Router, DefaultRouter
// - StatusCode
// - ViewSet, ModelViewSet, ReadOnlyModelViewSet
// - (if core feature) Request, Response, Handler, Middleware
// - (if database feature) Model, DatabaseConnection, F, Q, Transaction
// - (if di feature) Body, Cookie, Header, Json, Path, Query
```

---

## Code Style

### CS-1 (SHOULD): Prefer Prelude for Common Imports

For most examples, start with:

```rust
use reinhardt::prelude::*;
```

Then add explicit imports only for types not in prelude:

```rust
use reinhardt::prelude::*;
use reinhardt::{endpoint, Method};  // Not in prelude
```

### CS-2 (MUST): Consistent Import Style

Within a single file, be consistent:

**Option A: Prelude + Explicit**
```rust
use reinhardt::prelude::*;
use reinhardt::{endpoint, Method};
use serde_json::json;
```

**Option B: Fully Explicit**
```rust
use reinhardt::{Request, Response, StatusCode, UnifiedRouter, Method, endpoint};
use serde_json::json;
```

**❌ Don't mix patterns inconsistently**
```rust
use reinhardt::prelude::*;
use reinhardt::Request;  // ❌ Already in prelude!
use reinhardt_http::Response;  // ❌ Wrong crate!
```

---

## Testing Standards

### TS-1 (MUST): Test Dependencies

**External test crates and shared utilities** can be used in `[dev-dependencies]`:

```toml
[dev-dependencies]
rstest = "0.23"
tokio = { workspace = true, features = ["rt", "macros"] }
example-test-macros = { path = "../remote/test-macros" }
example-common = { path = "../remote/common" }
```

### TS-2 (MUST): Test Imports

Tests should import from `reinhardt::test` or use shared test utilities:

```rust
#[cfg(test)]
mod tests {
	use reinhardt::prelude::*;
	use reinhardt::test::client::APIClient;
	use reinhardt::test::fixtures::test_server_guard;
}
```

---

## Quick Reference

### Checklist for New Examples

- [ ] `Cargo.toml` includes ONLY `reinhardt` crate (not `reinhardt-*` sub-crates)
- [ ] All imports use `reinhardt::` prefix (not `reinhardt_*::`)
- [ ] Consider using `use reinhardt::prelude::*;` for common types
- [ ] No direct imports from `hyper` (use `reinhardt::Method`, `reinhardt::StatusCode`)
- [ ] Consistent import style throughout the example

### Common Mistakes to Avoid

Each mistake is paired with its correct alternative:

**Mistake 1: Importing from sub-crates**
- ❌ `use reinhardt_http::{Request, Response};`
- ✅ `use reinhardt::{Request, Response};`

**Mistake 2: Direct router imports from sub-crate**
- ❌ `use reinhardt_routers::UnifiedRouter;`
- ✅ `use reinhardt::UnifiedRouter;`

**Mistake 3: Importing macros from sub-crate**
- ❌ `use reinhardt_macros::endpoint;`
- ✅ `use reinhardt::endpoint;`

**Mistake 4: Importing from hyper directly**
- ❌ `use hyper::Method;`
- ✅ `use reinhardt::Method;`

**Mistake 5: Including sub-crates in dependencies**
- ❌ `reinhardt-http = { path = "..." }` in `[dependencies]`
- ✅ `reinhardt = { path = "...", features = [...] }`

### Correct Patterns Summary

- ✅ `use reinhardt::{...}` - Explicit imports from main crate
- ✅ `use reinhardt::prelude::*;` - Common types via prelude
- ✅ `use reinhardt::endpoint;` - Macros from main crate
- ✅ `use reinhardt::Method;` - Re-exported types from main crate

---

## Related Documentation

- **Main Project Standards**: @../CLAUDE.md
- **Feature Flags**: @../docs/FEATURE_FLAGS.md
- **Getting Started Guide**: @../docs/GETTING_STARTED.md
- **Project README**: @../README.md
