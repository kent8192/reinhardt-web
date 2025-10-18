# reinhardt

Main entry point and umbrella crate for the Reinhardt framework

## Overview

This is the main crate that re-exports all the core components of the Reinhardt framework. It provides a unified interface for building full-stack REST APIs in Rust, combining the functionality of all sub-crates into a single, convenient package.

## Features

### Implemented ✓

- **Full-stack API framework** - Complete re-export of all core components (ORM, ViewSets, Routers, Auth, etc.)
- **Django-inspired architecture** - Familiar patterns: Models, Views, Serializers, Middleware
- **Type-safe development** - Full Rust type system with compile-time guarantees
- **Batteries-included approach** - Everything needed for REST API development in one crate
- **Multiple configuration flavors** - Feature flags for Micro, Standard, and Full configurations
  - Micro: Routing + Params + DI only
  - Standard (default): Core REST API features
  - Full: All features including admin, GraphQL, WebSockets
- **Prelude module** - Convenient re-exports for common imports
- **Feature-based modularity** - Optional features for database, cache, sessions, etc.

### Planned

- Additional feature flag combinations for specific use cases
- More granular feature control

