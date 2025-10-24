# reinhardt-templates-macros

Procedural macros for compile-time template path validation in Reinhardt.

## Overview

This crate provides macros that validate template paths at compile time, ensuring that template paths follow correct syntax and security constraints before runtime.

## Features

- **Compile-time validation**: Catch template path errors during compilation
- **Path traversal protection**: Prevents `../` parent directory references
- **Cross-platform safety**: Enforces Unix-style paths (no backslashes)
- **Extension validation**: Ensures valid file extensions (.html, .txt, etc.)
- **Clear error messages**: Helpful compile-time error messages with examples

## Usage

```rust
use reinhardt_templates_macros::template;

// Valid template paths
let path = template!("emails/welcome.html");
let path = template!("admin/users/list.html");
let path = template!("base.html");
```

## Validation Rules

The `template!` macro enforces the following rules:

1. **Relative paths only**: No leading slash (`/`)
2. **No parent directory references**: No `..` in paths
3. **Unix-style paths**: No backslashes (`\`)
4. **Valid extensions**: Only allowed file extensions
5. **No double slashes**: No consecutive `/` characters
6. **Valid characters**: Alphanumeric, hyphens, underscores, dots, and slashes

### Allowed Extensions

- `.html`, `.htm` - HTML templates
- `.txt` - Text templates
- `.xml` - XML templates
- `.json` - JSON templates
- `.css`, `.js` - Style/script templates
- `.md` - Markdown templates
- `.svg` - SVG templates
- `.jinja`, `.j2`, `.tpl`, `.template` - Template files

## Examples

### Valid Paths

```rust
use reinhardt_templates_macros::template;

// Simple paths
template!("index.html");
template!("base.html");

// Nested paths
template!("emails/welcome.html");
template!("admin/users/list.html");

// Different extensions
template!("config.json");
template!("styles.css");
template!("template.jinja");

// With hyphens and underscores
template!("user-profile.html");
template!("user_details.html");
```

### Invalid Paths (Compile-time Errors)

```rust
use reinhardt_templates_macros::template;

// Error: parent directory reference
template!("../etc/passwd");

// Error: backslash not allowed
template!("path\\to\\file.html");

// Error: absolute path
template!("/etc/passwd");

// Error: invalid extension
template!("file.exe");

// Error: double slash
template!("templates//index.html");

// Error: empty path
template!("");
```

## Security

This macro helps prevent common security vulnerabilities:

- **Path Traversal Attacks**: By rejecting `..` references
- **Cross-platform Issues**: By enforcing Unix-style paths
- **Invalid Resources**: By validating file extensions

## Integration

This crate is designed to work with `reinhardt-templates` and can be used standalone for path validation.

```toml
[dependencies]
reinhardt-templates-macros = "0.1.0"
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.