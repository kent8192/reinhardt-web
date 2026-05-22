# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-pages-components@v0.2.0-rc.1) - 2026-05-22

### Added

- *(pages)* add reinhardt-pages-components crate for UI component library

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(pages-components)* update quick start example to use Theme API

### Fixed

- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)

### Maintenance

- *(pages-components)* migrate dev-dependencies to workspace management

### Styling

- *(pages-components)* apply rustfmt formatting to source files
- fix formatting in component_tests.rs

### Testing

- *(pages-components)* add unit tests for core types and Component trait

### Added
- Initial project structure
- Component trait and base infrastructure
- Layout components (Container, Grid, Layout)
- Navigation components (Nav, Breadcrumb, Tabs, Pagination)
- Feedback components (Alert, Toast, Modal, Progress)
- Data Display components (Badge, Card)
- Input components (Button, Dropdown, Accordion)
- Form components (LoginForm, RegisterForm)
- `page!` and `form!` declarative macros
- Theme system with CSS variables
- Responsive design support (6 breakpoints)
- WAI-ARIA accessibility attributes
