# backends

Database backend implementations for Reinhardt ORM

## Overview

`backends` provides database backend implementations for the Reinhardt ORM layer. It includes support for PostgreSQL, MySQL, and SQLite databases with unified abstractions for query building and execution.

## Features

- PostgreSQL backend implementation
- MySQL backend implementation
- SQLite backend implementation
- Unified database abstraction layer
- Query builder integration with sea-query
- Type-safe parameter binding with sqlx

## Installation

```toml
[dependencies]
backends = "0.1.0"
```

### Features

- `postgres` (default): PostgreSQL support
- `mysql`: MySQL support
- `sqlite`: SQLite support
- `all-databases`: All database backends

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.