# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-cms@v0.1.0-alpha.1) - 2026-03-06

### Added

- *(cms)* add reinhardt-cms crate scaffold
- *(cms)* add pages module for hierarchical page tree management
- *(cms)* add blocks module for StreamField content composition
- *(cms)* add media module for file and image management
- *(cms)* add permissions module for page-level access control
- *(cms)* add workflow module for page lifecycle management
- *(cms)* add admin module for CMS UI integration

### Fixed

- *(cms)* reduce type complexity with BlockFactory alias

### Testing

- *(cms)* add proptest and arbitrary to dev-dependencies
- *(cms)* add comprehensive tests for blocks module
- *(cms)* add error path, boundary, and property tests for pages module
- *(cms)* add error path, boundary, and property tests for media module
- *(cms)* add decision table, combination, and boundary tests for permissions module
- *(cms)* add state transition, decision table, and property tests for workflow module
- *(cms)* add registry, validation, and decision table tests for admin module
- *(cms)* add cross-module use case and combination tests
