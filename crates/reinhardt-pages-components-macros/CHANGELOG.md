# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-pages-components-macros@v0.2.0-rc.1) - 2026-05-23

### Added

- *(pages)* add reinhardt-pages-components crate for UI component library

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)
- *(pages-components-macros)* emit compile_error! instead of todo!() panics
