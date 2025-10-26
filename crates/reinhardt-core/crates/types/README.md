# reinhardt-types

Core type definitions

## Overview

Fundamental type definitions used across the Reinhardt framework. Provides traits for handlers, middleware, and common abstractions that other crates depend on.

## Features

### Implemented ✓

- **Handler trait** - Core abstraction for request processing
- **Middleware trait** - Request/response pipeline processing
- **MiddlewareChain** - Composable middleware system with automatic chaining
- **Type aliases** - Re-export of `Request` and `Response` for convenience
- **Async trait support** - Full async/await support via `async_trait`
- **Zero-cost abstractions** - All traits compile to efficient code with no runtime overhead