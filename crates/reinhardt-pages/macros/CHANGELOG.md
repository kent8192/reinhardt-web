# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.8](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0-alpha.7...reinhardt-pages-macros@v0.1.0-alpha.8) - 2026-02-18

### Maintenance

- updated the following local packages: reinhardt-manouche

## [0.1.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0-alpha.6...reinhardt-pages-macros@v0.1.0-alpha.7) - 2026-02-16

### Changed

- update references for flattened examples structure

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0-alpha.5...reinhardt-pages-macros@v0.1.0-alpha.6) - 2026-02-12

### Changed

- convert relative paths to absolute paths
- restore single-level super:: paths preserved by convention

### Reverted

- undo unintended visibility and formatting changes

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0-alpha.4...reinhardt-pages-macros@v0.1.0-alpha.5) - 2026-02-05

### Fixed

- add reinhardt-manouche to workspace deps and address review comments

### Other

- Merge branch 'main' into refactor/extract-manouche-dsl

## [0.1.0-alpha.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0-alpha.3...reinhardt-pages-macros@v0.1.0-alpha.4) - 2026-02-03

### Other

- merge main into chore/release-plz-migration
- add release-plz migration markers to CHANGELOGs

<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.3] - 2026-01-30

### Changed

- Version bump for publish workflow correction (no functional changes)


## [0.1.0-alpha.2] - 2026-01-29

### Changed

- Internal updates (changelog entry backfilled)

## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial release of `reinhardt-pages-macros` crate
- `page!` macro for anonymous component DSL with closure-style props
  - Support for 70+ HTML elements with compile-time validation
  - Event handlers using `@event: handler` syntax
  - Conditional rendering with `if`/`else` and list rendering with `for` loops
  - Reactive features with `watch` blocks for Signal-dependent rendering
  - Component calls with named arguments
  - Accessibility validation (img alt, button labels)
  - Security validation (XSS prevention for URL attributes)
- `head!` macro for HTML head section DSL
  - Support for title, meta, link, script, and style elements
  - SSR metadata injection support
- `form!` macro for type-safe forms with reactive bindings
  - Multiple field types: CharField, TextField, EmailField, IntegerField, etc.
  - Widget customization: TextInput, PasswordInput, Select, RadioSelect, etc.
  - Built-in validation (required, min/max length, pattern)
  - Server-side and client-side validators
  - UI state management (loading, error, success)
  - Two-way Signal binding
  - Computed values with `derived` block
  - Field groups with `FieldGroup`
  - Custom wrapper elements and SVG icon support
  - Slots for custom content injection
  - CSRF protection for non-GET methods
  - Initial value loading with `initial_loader`
  - Dynamic choice loading with `choices_loader`
- `#[server_fn]` attribute macro for Server Functions (RPC)
  - WASM client stub generation
  - Server-side handler generation
  - Custom endpoint paths
  - Codec selection (json, url, msgpack)
  - Dependency injection support with `#[reinhardt::inject]`

