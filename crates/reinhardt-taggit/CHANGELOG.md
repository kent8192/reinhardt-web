# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-taggit@v0.1.0) - 2026-03-06

### Added

- *(taggit)* add reinhardt-taggit crate for tagging functionality
- *(taggit)* implement slug auto-generation from tag name
- *(taggit)* implement taggable trait and derive macro

### Documentation

- *(taggit)* align lib.rs exports and documentation with actual implementation

### Fixed

- *(reinhardt-taggit)* add missing macro imports and dependencies
- *(taggit)* add reinhardt-taggit-tests to workspace members
- *(taggit)* disable autotests for reinhardt-taggit crate
- *(taggit)* resolve CI test failures
- *(taggit)* revert changelog and fix doc comment backtick syntax
- *(taggit)* resolve foreign key table name mismatch for tagged item
- *(taggit)* fix clone tests to compare fields individually
- *(taggit)* include created_at in fixture insert queries and fix schema workarounds

### Styling

- *(taggit)* fix clippy warnings in macros and tests

### Testing

- *(taggit)* add unit and integration test scaffolding
- *(taggit)* implement integration tests with testcontainers fixtures
