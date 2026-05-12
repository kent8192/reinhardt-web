# Changelog

All notable changes to this crate are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial crate. Exposes the `VersionedRouter` trait and the
  `RouteVersionInfo` value type so that `reinhardt-urls` and
  `reinhardt-rest` can share a router abstraction without forming a
  circular dependency (#4321).
