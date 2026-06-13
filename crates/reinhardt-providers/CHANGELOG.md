# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-providers@v0.2.0) - 2026-06-11

Stable release of `reinhardt-providers` for the Reinhardt 0.2.0 line. This
crate provides cloud-provider client helpers shared by storage and future
provider integrations.

### Migration Notes

- Use the normal AWS SDK credential chain for S3-compatible integrations.
- See [`instructions/MIGRATION_0.2.md`](../../instructions/MIGRATION_0.2.md) for the workspace migration checklist.

### Added

- *(providers)* add minimal S3 provider client

### Fixed

- *(providers)* preserve AWS credential chain

### Maintenance

- *(ci)* pin broken upstream transitive releases
