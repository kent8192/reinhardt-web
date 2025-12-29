# reinhardt-security

Security utilities for the Reinhardt framework.

## Overview

Security utilities and middleware for protecting web applications. Provides comprehensive security features including CSRF protection, XSS prevention, security headers management, and HSTS support.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["security"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import security features:

```rust
use reinhardt::security::{CsrfMiddleware, SecurityHeadersMiddleware};

// Or use specific modules
use reinhardt::security::csrf::{get_token_hmac, check_token_hmac};
use reinhardt::security::xss::escape_html;
```

**Note:** Security features are included in the `standard` and `full` feature presets.

## Features

### Implemented ✓

#### CSRF Protection

- **Token Generation & Validation**:
  - `get_secret_bytes()`: Generate cryptographically secure 32-byte secret for HMAC
  - `generate_token_hmac()`: Generate HMAC-SHA256 token from secret and message
  - `get_token_hmac()`: High-level token generation using secret and session ID
  - `verify_token_hmac()`: Constant-time HMAC verification
  - `check_token_hmac()`: Token validation with detailed error reporting
- **Token Rotation Support**:
  - `generate_token_with_timestamp()`: Generate token with timestamp for rotation tracking
  - `verify_token_with_timestamp()`: Verify timestamped token and extract timestamp
  - `get_token_timestamp()`: Get current Unix timestamp for rotation logic
  - `should_rotate_token()`: Determine if token rotation is due based on interval
  - Configurable via `CsrfConfig::with_token_rotation(interval)`
- **Origin/Referer Checking**: `check_origin()` and `check_referer()` validate request sources
- **Domain Validation**: `is_same_domain()` for cross-domain request protection
- **Configurable Cookie Settings**: Full control over SameSite, Secure, HttpOnly, Domain, Path, and Max-Age
- **Production-Ready Config**: `CsrfConfig::production()` with security hardening (includes token rotation)
- **Middleware**: `CsrfMiddleware` with customizable configuration
- **Error Handling**: Detailed rejection reasons for debugging (bad origin, bad referer, missing token, etc.)
- **Constants**: `CSRF_TOKEN_LENGTH`, `CSRF_SECRET_LENGTH`, `CSRF_SESSION_KEY`, rejection reason constants

#### XSS Prevention

- **HTML Escaping**:
  - `escape_html()`: Escapes dangerous characters (`<`, `>`, `&`, `"`, `'`)
  - `escape_html_attr()`: Escapes HTML attributes including newlines and control characters
- **JavaScript Context Escaping**: `escape_javascript()` for safe embedding in JavaScript strings
- **URL Encoding**: `escape_url()` for URL encoding to prevent injection
- **HTML Sanitization**: `sanitize_html()` for basic HTML input sanitization
- **XSS Pattern Detection**: `detect_xss_patterns()` detects dangerous patterns (script tags, event handlers, etc.)
- **URL Validation**: `is_safe_url()` validates URLs and allows only safe protocols (http, https, mailto, ftp)
- **Safe Output**: Prevents script injection in user-generated content across multiple contexts

#### Security Headers

- **Content Security Policy (CSP)**: Configurable CSP with granular control over:
  - `default-src`, `script-src`, `style-src`, `img-src`
  - `connect-src`, `font-src`, `object-src`, `media-src`, `frame-src`
  - **CSP Reporting**: `report-uri` and `report-to` for violation reporting via `with_report_uri()` and `with_report_to()`
  - **Nonce Generation**: `generate_nonce()` for inline script/style nonces
  - **Auto Nonce**: Automatic nonce injection with `with_auto_nonce()`
- **Security Headers Middleware**: `SecurityHeadersMiddleware` with comprehensive defaults
- **Configurable Headers**:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY` (clickjacking protection)
  - `X-XSS-Protection: 1; mode=block`
  - `Strict-Transport-Security` (HSTS)
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy` (optional)
  - **Cross-Origin Policies**:
    - `Cross-Origin-Embedder-Policy: require-corp`
    - `Cross-Origin-Opener-Policy: same-origin`
    - `Cross-Origin-Resource-Policy: same-origin`
- **Environment Presets**:
  - `SecurityHeadersConfig::production()`: Strict security headers for production
  - `SecurityHeadersConfig::development()`: Relaxed headers for development (no HSTS, no CSP)

#### HSTS (HTTP Strict Transport Security)

- **HSTS Configuration**: `HstsConfig` with builder pattern
- **Configurable Options**:
  - `max_age`: Configurable duration in seconds
  - `includeSubDomains`: Optional subdomain protection
  - `preload`: HSTS preload list support
- **Header Generation**: `build_header()` for automatic header value construction
- **Secure Defaults**: 1-year max-age default configuration

#### Security Utilities

The `utils` module provides internal security utilities:

- **Secure Token Generation**: `generate_token()` creates cryptographically random tokens (internal use)
- **SHA-256 Hashing**: `hash_sha256()` for secure string hashing (internal use)
- **Random Number Generation**: Built on `rand` crate for security

**Note**: These utilities are available through the `utils` module but are not re-exported at the crate root. They are primarily used internally by CSRF and other security features.

#### Error Handling

- **Comprehensive Error Types**: `SecurityError` enum with specific variants
- **CSRF Validation Errors**: Detailed error messages for debugging
- **XSS Detection**: Error type for potential XSS attempts
- **Configuration Errors**: Validation for security configurations

#### IP Filtering

- **Whitelist/Blacklist Modes**: `IpFilterMode` enum for configurable filtering strategy
  - `Whitelist`: Only allow IPs in the allowed list
  - `Blacklist`: Deny IPs in the blocked list (default)
- **IP Range Support**: Add individual IPs or CIDR ranges (e.g., `192.168.1.0/24`)
- **IPv4 and IPv6**: Full support for both IP versions
- **Flexible Configuration**: `IpFilterConfig` with builder-style methods
  - `new(mode)`: Create with specified mode
  - `whitelist()`: Create with whitelist mode
  - `blacklist()`: Create with blacklist mode
  - `add_allowed_ip(ip_or_range)`: Add IP addresses or ranges to whitelist
  - `add_blocked_ip(ip_or_range)`: Add IP addresses or ranges to blacklist
  - `is_allowed(&ip)`: Check if an IP address is permitted
- **Blacklist Override**: Blocked IPs take precedence over allowed IPs
- **Middleware**: `IpFilterMiddleware` for request filtering based on IP address

## Usage

### CSRF Protection

```rust
use reinhardt::security::csrf::{get_token_hmac, check_token_hmac};

// Generate CSRF token
let secret = b"your-secret-key-32-bytes-long!!!";
let session_id = "user-session-id";
let token = get_token_hmac(secret, session_id)?;

// Validate CSRF token
match check_token_hmac(secret, session_id, &token) {
    Ok(_) => println!("Token is valid"),
    Err(e) => println!("Token validation failed: {}", e),
}
```

### XSS Prevention

```rust
use reinhardt::security::xss::{escape_html, escape_javascript, is_safe_url};

// Escape user input for HTML context
let user_input = "<script>alert('xss')</script>";
let safe_html = escape_html(user_input);
// Output: &lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;

// Escape for JavaScript context
let js_string = escape_javascript("'; alert('xss'); //");
// Output: \'; alert(\'xss\'); \/\/

// Validate URL safety
if is_safe_url("https://example.com") {
    println!("URL is safe");
}
```

### Security Headers

```rust
use reinhardt::security::headers::{SecurityHeadersConfig, CspConfig};

// Production configuration with strict security
let config = SecurityHeadersConfig::production()
    .with_csp(
        CspConfig::strict()
            .with_default_src(vec!["'self'".to_string()])
            .with_script_src(vec!["'self'".to_string(), "'unsafe-inline'".to_string()])
    );

// Development configuration (relaxed)
let dev_config = SecurityHeadersConfig::development();
```

### IP Filtering

```rust
use reinhardt::security::{IpFilterConfig, IpFilterMode};
use std::net::IpAddr;

// Whitelist mode: Only allow specific IPs
let mut config = IpFilterConfig::whitelist();
config.add_allowed_ip("192.168.1.0/24")?;
config.add_allowed_ip("10.0.0.1")?;

let ip: IpAddr = "192.168.1.100".parse()?;
if config.is_allowed(&ip) {
    println!("IP is allowed");
} else {
    println!("IP is blocked");
}

// Blacklist mode: Block specific IPs
let mut block_config = IpFilterConfig::blacklist();
block_config.add_blocked_ip("192.168.1.100")?;
block_config.add_blocked_ip("10.0.0.0/8")?;
```