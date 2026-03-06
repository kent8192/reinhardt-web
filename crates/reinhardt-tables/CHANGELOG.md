# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-tables@v0.1.0-alpha.1) - 2026-03-06

### Added

- *(tables)* add reinhardt-tables crate for data table rendering

### Changed

- *(tables)* remove duplicate table_test.rs file

### Documentation

- *(tables)* align crate and module docs with actual implementation

### Fixed

- *(tables)* add ColumnNotFilterable error variant for filter operations
- *(tables)* reject per_page == 0 in paginate() to prevent division by zero

### Styling

- *(tables)* remove unused dependencies and imports

### Testing

- *(tables)* add table component unit tests with fixtures
