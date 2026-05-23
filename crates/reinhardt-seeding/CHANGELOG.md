# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-seeding@v0.2.0-rc.1) - 2026-05-23

### Added

- *(seeding)* add reinhardt-seeding and reinhardt-seeding-macros crates

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(seeding)* drop redundant intra-doc link targets and wrap bare URL

### Fixed

- add explicit path attributes for test module resolution
- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)
- *(seeding)* add version to reinhardt-seeding-macros workspace dep

### Styling

- *(reinhardt-seeding)* apply rustfmt code formatting

### Testing

- *(reinhardt-seeding)* add test fixtures, helpers, and trybuild tests
