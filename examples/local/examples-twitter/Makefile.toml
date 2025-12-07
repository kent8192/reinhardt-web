# Makefile.toml for Reinhardt Project
#
# This file provides cargo-make task definitions for common development operations.
# Install cargo-make: `cargo install cargo-make`
# Usage: `cargo make <task>`

[config]
default_to_workspace = false
skip_core_tasks = true

# ============================================================================
# Development Server
# ============================================================================

[tasks.runserver]
description = "Start the development server"
command = "cargo"
args = ["run", "--bin", "manage", "runserver"]

[tasks.runserver-watch]
description = "Start the development server with auto-reload (requires cargo-watch)"
command = "cargo"
args = ["watch", "-x", "run --bin manage runserver"]
dependencies = ["install-cargo-watch"]

# ============================================================================
# Database Migrations
# ============================================================================

[tasks.makemigrations]
description = "Create new migrations based on model changes"
command = "cargo"
args = ["run", "--bin", "manage", "makemigrations"]

[tasks.makemigrations-app]
description = "Create new migrations for a specific app (usage: cargo make makemigrations-app -- <app_label>)"
command = "cargo"
args = ["run", "--bin", "manage", "makemigrations", "${@}"]

[tasks.migrate]
description = "Apply database migrations"
command = "cargo"
args = ["run", "--bin", "manage", "migrate"]

# ============================================================================
# Static Files
# ============================================================================

[tasks.collectstatic]
description = "Collect static files into STATIC_ROOT"
command = "cargo"
args = ["run", "--bin", "manage", "collectstatic"]

# ============================================================================
# Project Management
# ============================================================================

[tasks.check]
description = "Check the project for common issues"
command = "cargo"
args = ["run", "--bin", "manage", "check"]

[tasks.showurls]
description = "Display all registered URL patterns"
command = "cargo"
args = ["run", "--bin", "manage", "showurls"]

[tasks.shell]
description = "Run an interactive Rust shell (REPL)"
command = "cargo"
args = ["run", "--bin", "manage", "shell"]

# ============================================================================
# Testing
# ============================================================================

[tasks.test]
description = "Run all tests"
command = "cargo"
args = ["nextest", "run", "--all-features"]
dependencies = ["install-nextest"]

[tasks.test-unit]
description = "Run unit tests only"
command = "cargo"
args = ["nextest", "run", "--lib", "--all-features"]
dependencies = ["install-nextest"]

[tasks.test-integration]
description = "Run integration tests only"
command = "cargo"
args = ["nextest", "run", "--test", "*", "--all-features"]
dependencies = ["install-nextest"]

[tasks.test-watch]
description = "Run tests with auto-reload (requires cargo-watch)"
command = "cargo"
args = ["watch", "-x", "nextest run --all-features"]
dependencies = ["install-cargo-watch", "install-nextest"]

# ============================================================================
# Code Quality
# ============================================================================

[tasks.fmt-check]
description = "Check code formatting"
command = "cargo"
args = ["fmt", "--all", "--", "--check"]

[tasks.fmt-fix]
description = "Fix code formatting"
command = "cargo"
args = ["fmt", "--all"]

[tasks.clippy-check]
description = "Check linting rules"
command = "cargo"
args = ["clippy", "--all-features", "--", "-D", "warnings"]

[tasks.clippy-fix]
description = "Fix linting issues automatically"
command = "cargo"
args = ["clippy", "--all-features", "--fix", "--allow-dirty", "--allow-staged"]

[tasks.quality]
description = "Run all code quality checks (format + lint)"
dependencies = ["fmt-check", "clippy-check"]

[tasks.quality-fix]
description = "Fix all code quality issues automatically"
dependencies = ["fmt-fix", "clippy-fix"]

# ============================================================================
# Build & Clean
# ============================================================================

[tasks.build]
description = "Build the project in debug mode"
command = "cargo"
args = ["build", "--all-features"]

[tasks.build-release]
description = "Build the project in release mode"
command = "cargo"
args = ["build", "--release", "--all-features"]

[tasks.clean]
description = "Clean build artifacts"
command = "cargo"
args = ["clean"]

# ============================================================================
# Dependencies Installation
# ============================================================================

[tasks.install-nextest]
description = "Install cargo-nextest if not already installed"
script = '''
if ! command -v cargo-nextest &> /dev/null
then
	echo "Installing cargo-nextest..."
	cargo install cargo-nextest --locked
else
	echo "cargo-nextest is already installed"
fi
'''

[tasks.install-cargo-watch]
description = "Install cargo-watch if not already installed"
script = '''
if ! command -v cargo-watch &> /dev/null
then
	echo "Installing cargo-watch..."
	cargo install cargo-watch
else
	echo "cargo-watch is already installed"
fi
'''

[tasks.install-tools]
description = "Install all required development tools"
dependencies = ["install-nextest", "install-cargo-watch"]

# ============================================================================
# Development Workflow
# ============================================================================

[tasks.dev]
description = "Start development environment (checks, builds, runs server)"
dependencies = ["quality", "build", "runserver"]

[tasks.dev-watch]
description = "Start development with auto-reload"
dependencies = ["quality", "build", "runserver-watch"]

# ============================================================================
# CI/CD Workflow
# ============================================================================

[tasks.ci]
description = "Run CI pipeline (format, lint, build, test)"
dependencies = ["fmt-check", "clippy-check", "build", "test"]

# ============================================================================
# Verbosity Control
# ============================================================================

[tasks.runserver-v]
description = "Start the development server with verbose output"
command = "cargo"
args = ["run", "--bin", "manage", "runserver", "-v"]

[tasks.runserver-vv]
description = "Start the development server with very verbose output"
command = "cargo"
args = ["run", "--bin", "manage", "runserver", "-vv"]

[tasks.runserver-vvv]
description = "Start the development server with maximum verbosity"
command = "cargo"
args = ["run", "--bin", "manage", "runserver", "-vvv"]

# ============================================================================
# Help
# ============================================================================

[tasks.help]
description = "Show available tasks"
script = '''
echo "Available tasks:"
echo "  Development:"
echo "    runserver          - Start the development server"
echo "    runserver-watch    - Start server with auto-reload"
echo "    dev                - Run checks + build + start server"
echo "    dev-watch          - Development with auto-reload"
echo ""
echo "  Database:"
echo "    makemigrations     - Create new migrations"
echo "    makemigrations-app - Create migrations for specific app"
echo "    migrate            - Apply migrations"
echo ""
echo "  Static Files:"
echo "    collectstatic      - Collect static files"
echo ""
echo "  Project Management:"
echo "    check              - Check project for issues"
echo "    showurls           - Show URL patterns"
echo "    shell              - Interactive REPL"
echo ""
echo "  Testing:"
echo "    test               - Run all tests"
echo "    test-unit          - Run unit tests"
echo "    test-integration   - Run integration tests"
echo "    test-watch         - Tests with auto-reload"
echo ""
echo "  Code Quality:"
echo "    fmt-check          - Check formatting"
echo "    fmt-fix            - Fix formatting"
echo "    clippy-check       - Check linting"
echo "    clippy-fix         - Fix linting issues"
echo "    quality            - Run all checks"
echo "    quality-fix        - Fix all issues"
echo ""
echo "  Build:"
echo "    build              - Build (debug)"
echo "    build-release      - Build (release)"
echo "    clean              - Clean artifacts"
echo ""
echo "  CI/CD:"
echo "    ci                 - Run CI pipeline"
echo ""
echo "  Tools:"
echo "    install-tools      - Install dev tools"
echo ""
echo "Usage: cargo make <task>"
'''
