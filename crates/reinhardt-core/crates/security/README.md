# reinhardt-security

Security utilities for the Reinhardt framework.

## Overview

Security utilities and middleware for protecting web applications. Provides comprehensive security features including CSRF protection, XSS prevention, security headers management, and HSTS support.

## Features

### Implemented ✓

#### CSRF Protection

- **Token Generation & Validation**: Cryptographically secure CSRF token generation with masking/unmasking mechanism
- **Token Management**: `get_secret()`, `get_token()`, `rotate_token()` functions for lifecycle management
- **Format Validation**: `check_token_format()` validates token length and character set
- **Token Matching**: `does_token_match()` for comparing tokens securely
- **Origin/Referer Checking**: `check_origin()` and `check_referer()` validate request sources
- **Domain Validation**: `is_same_domain()` for cross-domain request protection
- **Configurable Cookie Settings**: Full control over SameSite, Secure, HttpOnly, Domain, Path, and Max-Age
- **Production-Ready Config**: `CsrfConfig::production()` with security hardening
- **Middleware**: `CsrfMiddleware` with customizable configuration
- **Error Handling**: Detailed rejection reasons for debugging (bad origin, bad referer, missing token, etc.)

#### XSS Prevention

- **HTML Escaping**: `escape_html()` escapes dangerous characters (`<`, `>`, `&`, `"`, `'`)
- **HTML Sanitization**: `sanitize_html()` for basic HTML input sanitization
- **Safe Output**: Prevents script injection in user-generated content

#### Security Headers

- **Content Security Policy (CSP)**: Configurable CSP with granular control over:
  - `default-src`, `script-src`, `style-src`, `img-src`
  - `connect-src`, `font-src`, `object-src`, `media-src`, `frame-src`
- **Security Headers Middleware**: `SecurityHeadersMiddleware` with comprehensive defaults
- **Configurable Headers**:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY` (clickjacking protection)
  - `X-XSS-Protection: 1; mode=block`
  - `Strict-Transport-Security` (HSTS)
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy` (optional)

#### HSTS (HTTP Strict Transport Security)

- **HSTS Configuration**: `HstsConfig` with builder pattern
- **Configurable Options**:
  - `max_age`: Configurable duration in seconds
  - `includeSubDomains`: Optional subdomain protection
  - `preload`: HSTS preload list support
- **Header Generation**: `build_header()` for automatic header value construction
- **Secure Defaults**: 1-year max-age default configuration

#### Security Utilities

- **Secure Token Generation**: `generate_token()` creates cryptographically random tokens
- **SHA-256 Hashing**: `hash_sha256()` for secure string hashing
- **Random Number Generation**: Built on `rand` crate for security

#### Error Handling

- **Comprehensive Error Types**: `SecurityError` enum with specific variants
- **CSRF Validation Errors**: Detailed error messages for debugging
- **XSS Detection**: Error type for potential XSS attempts
- **Configuration Errors**: Validation for security configurations

### Planned

Currently all planned features are implemented.
