# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-payment@v0.2.0-rc.1) - 2026-05-23

### Added

- *(reinhardt-payment)* add crate scaffold and documentation
- *(reinhardt-payment)* add payment domain types
- *(reinhardt-payment)* add PaymentProvider trait and Stripe integration
- *(reinhardt-payment)* add TokenVault trait and BasisTheory integration
- *(reinhardt-payment)* add webhook event handling and signature verification
- *(reinhardt-payment)* add idempotency and retry utilities

### Changed

- *(reinhardt-payment)* move integration tests to tests crate
- *(payment)* remove duplicate VaultError definition from vault.rs
- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(payment)* update README examples with correct type references

### Fixed

- *(payment)* use ignore marker for doc example to avoid dev-dependency violation
- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)

### Styling

- *(reinhardt-payment)* apply rustfmt formatting to tests

### Testing

- *(reinhardt-payment)* add payment provider integration tests

### Added

- Initial implementation of `reinhardt-payment` crate
- `PaymentProvider` trait for payment abstraction
- `TokenVault` trait for card tokenization
- Stripe integration with PaymentIntent support
- Checkout Sessions support for hosted payment pages
- Subscription management for recurring payments
- BasisTheory integration for PCI-compliant tokenization
- Webhook signature verification with HMAC-SHA256
- Idempotency key generation for safe retry
- Exponential backoff retry strategy with jitter
- Comprehensive error types for payment and vault operations
- Domain types for PaymentIntent, CheckoutSession, and Subscription
- Webhook event parsing and dispatch
- Security features:
  - Constant-time signature comparison
  - Timestamp validation for replay attack prevention
  - Secure token handling

### Documentation

- Complete rustdoc documentation for all public APIs
- README with quick start guide and examples
- Architecture diagrams with Mermaid
- Security best practices guide

### Testing

- Unit tests for core functionality
- Integration tests for Stripe API
- Integration tests for BasisTheory API
- Property-based tests for idempotency and retry logic
- Security tests for webhook verification

## [0.1.0] - Unreleased

Initial release (in progress)
