# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-manouche@v0.1.0-alpha.1...reinhardt-manouche@v0.1.0-alpha.2) - 2026-02-18

### Fixed

- *(manouche)* add compile-time validation for js_condition to prevent injection

### Maintenance

- *(manouche)* convert TODO comments to todo!() macros in IR lowering

## [0.1.0-alpha.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-manouche@v0.1.0-alpha.1) - 2026-02-05

### Added

- *(manouche)* add IRVisitor trait and walk helpers
- *(manouche)* add IR type definitions
- *(manouche)* add validator module with page, form, and head validators
- *(manouche)* add parser module with page, form, and head parsers
- *(manouche)* add head node definitions
- *(manouche)* migrate typed form definitions
- *(manouche)* migrate form node definitions
- *(manouche)* migrate typed page node definitions
- *(manouche)* migrate page node definitions
- *(manouche)* migrate types module from reinhardt-pages-ast
- *(manouche)* add reactive trait definitions

### Fixed

- add reinhardt-manouche to workspace deps and address review comments

### Other

- *(manouche)* add README
- *(manouche)* add module skeleton
- *(manouche)* create reinhardt-manouche crate structure
