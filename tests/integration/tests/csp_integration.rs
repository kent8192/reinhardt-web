//! Content Security Policy (CSP) Integration Tests
//!
//! Tests CSP header generation and configuration
//! Based on Django's CSP middleware tests

use reinhardt_integration_tests::security_test_helpers::*;

use hyper::header::HeaderValue;
use reinhardt_security::ContentSecurityPolicy;

#[test]
fn test_csp_default_src() {
    // Test: CSP with default-src directive
    let csp = ContentSecurityPolicy {
        default_src: vec!["'self'".to_string()],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("default-src 'self'"));
}

#[test]
fn test_csp_integration_script_src() {
    // Test: CSP with script-src directive
    let csp = ContentSecurityPolicy {
        script_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("script-src"));
    assert!(header.contains("'self'"));
    assert!(header.contains("'unsafe-inline'"));
}

#[test]
fn test_csp_integration_style_src() {
    // Test: CSP with style-src directive
    let csp = ContentSecurityPolicy {
        style_src: vec![
            "'self'".to_string(),
            "https://fonts.googleapis.com".to_string(),
        ],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("style-src"));
    assert!(header.contains("https://fonts.googleapis.com"));
}

#[test]
fn test_csp_integration_img_src() {
    // Test: CSP with img-src directive
    let csp = ContentSecurityPolicy {
        img_src: vec![
            "'self'".to_string(),
            "data:".to_string(),
            "https:".to_string(),
        ],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("img-src"));
    assert!(header.contains("data:"));
    assert!(header.contains("https:"));
}

#[test]
fn test_csp_integration_multiple_directives() {
    // Test: CSP with multiple directives
    let csp = ContentSecurityPolicy {
        default_src: vec!["'self'".to_string()],
        script_src: vec!["'self'".to_string(), "'unsafe-eval'".to_string()],
        style_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
        img_src: vec!["'self'".to_string(), "data:".to_string()],
    };

    let header = csp.to_header_value();
    assert!(header.contains("default-src"));
    assert!(header.contains("script-src"));
    assert!(header.contains("style-src"));
    assert!(header.contains("img-src"));
}

#[test]
fn test_csp_header_format() {
    // Test: CSP header uses semicolon separators
    let csp = ContentSecurityPolicy {
        default_src: vec!["'self'".to_string()],
        script_src: vec!["'self'".to_string()],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("; "));
}

#[test]
fn test_csp_wildcard() {
    // Test: CSP can use wildcard
    let csp = ContentSecurityPolicy {
        default_src: vec!["*".to_string()],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("default-src *"));
}

#[test]
fn test_csp_none() {
    // Test: CSP with 'none' value
    let csp = ContentSecurityPolicy {
        default_src: vec!["'none'".to_string()],
        script_src: vec!["'self'".to_string()],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("'none'"));
}

#[test]
fn test_csp_nonce() {
    // Test: CSP with nonce value
    let nonce = "rAnd0m123";
    let csp = ContentSecurityPolicy {
        script_src: vec!["'self'".to_string(), format!("'nonce-{}'", nonce)],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains(&format!("'nonce-{}'", nonce)));
}

#[test]
fn test_csp_hash() {
    // Test: CSP with hash value
    let hash = "sha256-abc123";
    let csp = ContentSecurityPolicy {
        script_src: vec!["'self'".to_string(), format!("'{}'", hash)],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains(hash));
}

#[test]
fn test_csp_strict_dynamic() {
    // Test: CSP with 'strict-dynamic' keyword
    let csp = ContentSecurityPolicy {
        script_src: vec!["'strict-dynamic'".to_string(), "'self'".to_string()],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("'strict-dynamic'"));
}

#[test]
fn test_csp_multiple_hosts() {
    // Test: CSP with multiple allowed hosts
    let csp = ContentSecurityPolicy {
        script_src: vec![
            "'self'".to_string(),
            "https://cdn.example.com".to_string(),
            "https://analytics.example.com".to_string(),
        ],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("https://cdn.example.com"));
    assert!(header.contains("https://analytics.example.com"));
}

#[test]
fn test_csp_response_header() {
    // Test: CSP header in response
    let csp = ContentSecurityPolicy::new();
    let header_value = csp.to_header_value();

    let mut response = create_test_response();
    response.headers.insert(
        "content-security-policy",
        HeaderValue::from_str(&header_value).unwrap(),
    );

    assert_has_header(&response, "content-security-policy");
}

#[test]
fn test_csp_report_only() {
    // Test: CSP Report-Only mode (header name check)
    let csp = ContentSecurityPolicy::new();
    let header_value = csp.to_header_value();

    let mut response = create_test_response();
    response.headers.insert(
        "content-security-policy-report-only",
        HeaderValue::from_str(&header_value).unwrap(),
    );

    assert_has_header(&response, "content-security-policy-report-only");
}

#[test]
fn test_csp_empty_directives() {
    // Test: CSP with empty directive values
    let csp = ContentSecurityPolicy {
        default_src: vec![],
        script_src: vec![],
        style_src: vec![],
        img_src: vec![],
    };

    let header = csp.to_header_value();
    // Empty CSP should produce minimal output
    assert!(header.is_empty() || !header.contains("'self'"));
}

#[test]
fn test_csp_protocol_sources() {
    // Test: CSP with protocol sources
    let csp = ContentSecurityPolicy {
        img_src: vec![
            "https:".to_string(),
            "data:".to_string(),
            "blob:".to_string(),
        ],
        ..Default::default()
    };

    let header = csp.to_header_value();
    assert!(header.contains("https:"));
    assert!(header.contains("data:"));
    assert!(header.contains("blob:"));
}
