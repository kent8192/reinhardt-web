# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-pages-components@v0.1.0) - 2026-03-06

### Added

- *(pages)* add reinhardt-pages-components crate for UI component library

### Documentation

- *(pages-components)* update quick start example to use Theme API

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
