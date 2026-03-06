# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-deploy@v0.1.0-alpha.1) - 2026-03-06

### Added

- *(deploy)* create reinhardt-deploy crate scaffold
- *(deploy)* add deploy error types
- *(deploy)* add deploy.toml configuration types with parsing
- *(deploy)* add layer 1 feature flag analysis
- *(deploy)* add layer 2 code analysis for FDI detection
- *(deploy)* add layer 3 interactive wizard and unified detection
- *(deploy)* add deploy provider trait and factory
- *(deploy)* add terraform CLI runner with version checking
- *(deploy)* add HCL template generator with Tera rendering
- *(deploy)* add terraform plan result parser and state management
- *(deploy)* add Docker provider with HCL template generation
- *(deploy)* add pre-flight check system
- *(deploy)* add Docker image builder with multi-stage Dockerfile
- *(deploy)* add deploy pipeline orchestration
- *(deploy)* add deployment report generation (human/JSON/markdown)
- *(deploy)* add deploy init command with auto-detection
- *(deploy)* add AWS provider with ECS Fargate templates
- *(deploy)* add GCP provider with Cloud Run templates
- *(deploy)* add fly.io provider with Machine templates
- *(deploy)* add deployment history and rollback mechanism
- *(deploy)* add preview deployment support with per-PR isolation
- *(deploy)* add monthly cost estimation engine
- *(deploy)* add CI/CD workflow generator for GitHub Actions
- *(deploy)* add dry-run diff command for idempotency verification
- *(deploy)* implement detection wizard interactive prompts

### Documentation

- *(deploy)* add crate and module documentation

### Fixed

- use serde_json::json! for valid JSON error fallback in report.rs
- *(deploy)* correct nosql_engine field name to nosql_engines in wizard

### Maintenance

- *(deploy)* replace dialoguer with inquire, console, and comfy-table

### Testing

- *(deploy)* add integration tests for deploy pipeline
