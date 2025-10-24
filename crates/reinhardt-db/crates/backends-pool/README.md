# backends-pool

Database connection pool backend abstractions

## Overview

`backends-pool` provides backend abstractions for database connection pooling in the Reinhardt framework. It defines traits and utilities for managing database connection pools with dependency injection support.

## Features

- Connection pool backend abstractions
- Async connection management
- Integration with sqlx connection pools
- Dependency injection support (optional)
- Thread-safe connection handling
- Connection lifecycle management

## Installation

```toml
[dependencies]
backends-pool = "0.1.0"
```

### Features

- `reinhardt-di`: Dependency injection integration

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.