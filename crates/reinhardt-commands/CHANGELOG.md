# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.9](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0-alpha.8...reinhardt-commands@v0.1.0-alpha.9) - 2026-02-07

### Other

- Merge pull request #129 from kent8192/fix/issue-128-bug-runserver-uses-settingsdefault-instead-of-loading-from-settings-directory

## [0.1.0-alpha.8](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0-alpha.7...reinhardt-commands@v0.1.0-alpha.8) - 2026-02-07

### Fixed

- add version to reinhardt-test workspace dependency for crates.io publishing

## [0.1.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0-alpha.6...reinhardt-commands@v0.1.0-alpha.7) - 2026-02-06

### Other

- updated the following local packages: reinhardt-utils, reinhardt-di, reinhardt-rest, reinhardt-conf, reinhardt-server, reinhardt-apps, reinhardt-db, reinhardt-mail, reinhardt-middleware, reinhardt-urls, reinhardt-pages, reinhardt-test, reinhardt-dentdelion, reinhardt-openapi

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0-alpha.5...reinhardt-commands@v0.1.0-alpha.6) - 2026-02-03

### Other

- updated the following local packages: reinhardt-pages, reinhardt-http, reinhardt-utils, reinhardt-conf, reinhardt-di, reinhardt-server, reinhardt-apps, reinhardt-db, reinhardt-mail, reinhardt-middleware, reinhardt-rest, reinhardt-urls, reinhardt-test, reinhardt-dentdelion, reinhardt-openapi

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0-alpha.4...reinhardt-commands@v0.1.0-alpha.5) - 2026-02-03

### Other

- merge main into chore/release-plz-migration
- add release-plz migration markers to CHANGELOGs
- N/A

### Added
- Work in progress features (not yet released)

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A


<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.4] - 2026-01-30

### Changed

- Version bump for publish workflow correction (no functional changes)

## [0.1.0-alpha.3] - 2026-01-29

### Changed

- Update imports for `reinhardt_utils::staticfiles` module rename (#114)

## [0.1.0-alpha.2] - 2026-01-28

### Changed
- Migrated welcome page rendering from Tera to reinhardt-pages SSR
- Added reinhardt-pages dependency

### Removed
- Removed welcome.tpl template (replaced by WelcomePage component)


## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial crates.io release

