# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/kent8192/reinhardt-web/releases/tag/tree-sitter-reinhardt-head@v0.1.1) - 2026-05-30

### Added

- *(admin-cli)* delegate DSL formatting to Topiary

### Fixed

- address CodeRabbit review comments
- consume opening '*' before scanning block comment body
- *(tree-sitter)* handle Rust lifetime annotations in DSL scanner

### Styling

- apply updated Topiary block formatting rules

### Testing

- add comprehensive tests for tree-sitter scanners and format engine
