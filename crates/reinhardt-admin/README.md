# reinhardt-admin

Admin functionality for Reinhardt framework.

## Features

This crate provides two main components:

- **Panel** (`reinhardt-panel`): Django-style web admin panel for managing models
- **CLI** (`reinhardt-cli`): Command-line tool for project management

## Usage

### Using the admin panel

Add to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-admin = { version = "0.1.0-alpha.1", features = ["panel"] }
```

### Using the CLI

Install the CLI globally:

```bash
cargo install reinhardt-admin --features cli
```

## Feature Flags

- `panel` (default): Web admin panel
- `cli`: Command-line interface
- `all`: All admin functionality

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
