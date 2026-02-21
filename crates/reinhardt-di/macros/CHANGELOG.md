# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-di-macros@v0.1.0-alpha.1...reinhardt-di-macros@v0.1.0-alpha.2) - 2026-02-21

### Fixed

- remove undeclared tracing dependency from injectable macro output

### Security

- improve generated name hygiene, crate path diagnostics, and type path validation
- reject unknown macro arguments and unsupported scope attribute
- remove info leak and validate factory code generation
