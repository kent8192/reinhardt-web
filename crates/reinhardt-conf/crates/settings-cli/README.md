# reinhardt-settings-cli

CLI tool for settings management

## Overview

Command-line interface for managing Reinhardt configuration files. Provides commands for encryption, validation, environment management, and secret handling with colorized output.

## Features

### Implemented ✓

#### Configuration Management

- **Validate**: Configuration file validation with syntax checking
  - TOML, JSON, and .env file format support
  - Profile-specific validation (development, staging, production)
  - Line-by-line validation for .env files
  - Integration with `SecurityValidator` for profile-based validation

- **Show**: Display configuration values with multiple output formats
  - Support for TOML, JSON, and .env files
  - Nested key navigation using dot notation (e.g., `database.host`)
  - Multiple output formats: text (colorized), JSON, and TOML
  - Profile-aware configuration display

- **Set**: Modify configuration values
  - Dot notation for nested keys (e.g., `database.port`)
  - Automatic type inference (bool, integer, float, string)
  - Create new configuration files with `--create` flag
  - Automatic backup creation before modification
  - Support for TOML, JSON, and .env file formats

- **Diff**: Compare two configuration files
  - Side-by-side comparison of TOML, JSON, and .env files
  - Show additions, deletions, and modifications
  - Optional value display with `--show-values`
  - Filter to show only differences with `--only-differences`
  - Summary statistics (total differences, additions, deletions)

#### Security Features

- **Encrypt**: AES-256-GCM encryption for configuration files
  - 32-byte (256-bit) key support in hex format
  - Custom output path or automatic `.enc` extension
  - Optional deletion of original file after encryption
  - Integration with `ConfigEncryptor` from reinhardt-settings

- **Decrypt**: Decrypt encrypted configuration files
  - Compatible with AES-256-GCM encrypted files
  - Automatic output path determination
  - Optional deletion of encrypted file after decryption
  - Hex key format validation

#### Output Utilities

- **Colorized Terminal Output**: Rich terminal experience
  - Success messages (green checkmark)
  - Error messages (red cross)
  - Warning messages (yellow warning symbol)
  - Info messages (blue info symbol)
  - Key-value pair formatting
  - Table formatting capabilities
  - Diff visualization with color-coded changes

- **Multiple Output Formats**:
  - Text format with colorized syntax highlighting
  - JSON format with pretty printing
  - TOML format with pretty printing