# reinhardt-admin

Global command-line tool for Reinhardt project management.

## Overview

`reinhardt-admin` is the Django's `django-admin` equivalent for Reinhardt. It provides utilities for creating new projects and applications.

## Installation

Install globally using cargo:

```bash
cargo install reinhardt-admin
```

## Usage

### Create a New Project

```bash
# Create a RESTful API project (default)
reinhardt-admin startproject myproject

# Create an MTV-style project
reinhardt-admin startproject myproject --template-type mtv

# Create project in a specific directory
reinhardt-admin startproject myproject /path/to/directory
```

### Create a New App

```bash
# Create a RESTful app (default)
reinhardt-admin startapp myapp

# Create an MTV-style app
reinhardt-admin startapp myapp --template-type mtv

# Create app in a specific directory
reinhardt-admin startapp myapp /path/to/directory
```

### Other Commands

```bash
# Display help
reinhardt-admin help

# Display version
reinhardt-admin --version
```

## Django Equivalents

| Django                                | Reinhardt                                |
|---------------------------------------|------------------------------------------|
| `django-admin startproject myproject` | `reinhardt-admin startproject myproject` |
| `django-admin startapp myapp`         | `reinhardt-admin startapp myapp`         |

## Project Templates

`reinhardt-admin` includes two project templates:

- **RESTful** (default): API-focused applications
- **MTV**: Traditional server-rendered web applications (Model-Template-View)

## App Templates

Apps can be created in two forms:

- **Module** (default): Created in `apps/` directory
- **Workspace**: Separate crate in workspace

## Features

- **Embedded Templates**: Templates are compiled into the binary using `rust-embed`
- **No External Dependencies**: Works without internet connection
- **Django-Compatible**: Familiar interface for Django developers

## Architecture

`reinhardt-admin` depends on `reinhardt-commands` for its core functionality:

```
reinhardt-admin (CLI binary)
    ↓
reinhardt-commands (Library)
    ↓
StartProjectCommand / StartAppCommand
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.