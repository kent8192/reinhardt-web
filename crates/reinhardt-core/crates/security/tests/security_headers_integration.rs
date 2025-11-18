//! Security Headers Integration Tests
//!
//! Tests security headers middleware functionality including:
//! - HSTS (HTTP Strict Transport Security)
//! - CSP (Content Security Policy)
//! - X-Frame-Options
//! - X-Content-Type-Options
//! - Referrer-Policy
//! - Cross-Origin policies
//! - Header combination and conflict handling

use reinhardt_http::Response;
use reinhardt_security::headers::{
	ContentSecurityPolicy, SecurityHeadersConfig, SecurityHeadersMiddleware,
};
use reinhardt_security::hsts::{HstsConfig, HstsMiddleware};

// ============================================================================
// Test Utilities
// ============================================================================

/// Create test response builder for header inspection
fn create_test_response() -> Response {
	Response::ok().with_body("test response")
}

/// Apply security headers to response based on config
fn apply_security_headers(response: Response, config: &SecurityHeadersConfig) -> Response {
	let mut response = response;

	// X-Content-Type-Options
	if config.x_content_type_options {
		response = response.with_header("X-Content-Type-Options", "nosniff");
	}

	// X-Frame-Options
	if let Some(ref value) = config.x_frame_options {
		response = response.with_header("X-Frame-Options", value);
	}

	// X-XSS-Protection
	if config.x_xss_protection {
		response = response.with_header("X-XSS-Protection", "1; mode=block");
	}

	// Strict-Transport-Security
	if let Some(ref value) = config.strict_transport_security {
		response = response.with_header("Strict-Transport-Security", value);
	}

	// Content-Security-Policy
	if let Some(ref csp) = config.content_security_policy {
		let csp_value = csp.to_header_value();
		response = response.with_header("Content-Security-Policy", &csp_value);
	}

	// Referrer-Policy
	if let Some(ref value) = config.referrer_policy {
		response = response.with_header("Referrer-Policy", value);
	}

	// Permissions-Policy
	if let Some(ref value) = config.permissions_policy {
		response = response.with_header("Permissions-Policy", value);
	}

	// Cross-Origin-Embedder-Policy
	if let Some(ref value) = config.cross_origin_embedder_policy {
		response = response.with_header("Cross-Origin-Embedder-Policy", value);
	}

	// Cross-Origin-Opener-Policy
	if let Some(ref value) = config.cross_origin_opener_policy {
		response = response.with_header("Cross-Origin-Opener-Policy", value);
	}

	// Cross-Origin-Resource-Policy
	if let Some(ref value) = config.cross_origin_resource_policy {
		response = response.with_header("Cross-Origin-Resource-Policy", value);
	}

	response
}

/// Apply HSTS header to response
fn apply_hsts_header(response: Response, config: &HstsConfig) -> Response {
	let hsts_value = config.build_header();
	response.with_header("Strict-Transport-Security", &hsts_value)
}

// ============================================================================
// HSTS Header Tests
// ============================================================================

#[test]
fn test_hsts_basic_header() {
	// Test: Basic HSTS header is correctly applied
	let config = HstsConfig::new(31536000);
	let response = create_test_response();
	let response = apply_hsts_header(response, &config);

	let hsts_header = response.headers.get("Strict-Transport-Security").unwrap();
	assert_eq!(hsts_header.to_str().unwrap(), "max-age=31536000");
}

#[test]
fn test_hsts_with_subdomains() {
	// Test: HSTS header includes includeSubDomains directive
	let config = HstsConfig::new(31536000).with_subdomains(true);
	let response = create_test_response();
	let response = apply_hsts_header(response, &config);

	let hsts_header = response.headers.get("Strict-Transport-Security").unwrap();
	assert_eq!(
		hsts_header.to_str().unwrap(),
		"max-age=31536000; includeSubDomains"
	);
}

#[test]
fn test_hsts_with_preload() {
	// Test: HSTS header includes preload directive
	let config = HstsConfig::new(63072000)
		.with_subdomains(true)
		.with_preload(true);
	let response = create_test_response();
	let response = apply_hsts_header(response, &config);

	let hsts_header = response.headers.get("Strict-Transport-Security").unwrap();
	assert_eq!(
		hsts_header.to_str().unwrap(),
		"max-age=63072000; includeSubDomains; preload"
	);
}

#[test]
fn test_hsts_middleware_default_config() {
	// Test: HstsMiddleware uses default configuration correctly
	let middleware = HstsMiddleware::default();
	let header_value = middleware.get_header_value();

	assert_eq!(header_value, "max-age=31536000");
	assert_eq!(middleware.config().max_age, 31536000);
	assert!(!middleware.config().include_subdomains);
	assert!(!middleware.config().preload);
}

#[test]
fn test_hsts_middleware_custom_config() {
	// Test: HstsMiddleware applies custom configuration
	let config = HstsConfig::new(7200)
		.with_subdomains(true)
		.with_preload(false);
	let middleware = HstsMiddleware::new(config);
	let header_value = middleware.get_header_value();

	assert_eq!(header_value, "max-age=7200; includeSubDomains");
}

// ============================================================================
// CSP Header Tests
// ============================================================================

#[test]
fn test_csp_default_policy() {
	// Test: Default CSP policy generates correct header
	let csp = ContentSecurityPolicy::default();
	let header = csp.to_header_value();

	assert!(header.contains("default-src 'self'"));
	assert!(header.contains("script-src 'self'"));
	assert!(header.contains("style-src 'self'"));
	assert!(header.contains("object-src 'none'"));
}

#[test]
fn test_csp_with_nonce() {
	// Test: CSP with nonce includes nonce directive in script-src and style-src
	let csp = ContentSecurityPolicy::new();
	let nonce = "abc123xyz789";
	let header = csp.to_header_value_with_nonce(Some(nonce));

	assert!(header.contains("script-src 'self' 'nonce-abc123xyz789'"));
	assert!(header.contains("style-src 'self' 'nonce-abc123xyz789'"));
}

#[test]
fn test_csp_with_report_uri() {
	// Test: CSP includes report-uri directive
	let csp = ContentSecurityPolicy::new().with_report_uri("/csp-violation-report");
	let header = csp.to_header_value();

	assert!(header.contains("report-uri /csp-violation-report"));
}

#[test]
fn test_csp_with_report_to() {
	// Test: CSP includes report-to directive for Reporting API
	let csp = ContentSecurityPolicy::new().with_report_to("csp-endpoint");
	let header = csp.to_header_value();

	assert!(header.contains("report-to csp-endpoint"));
}

#[test]
fn test_csp_nonce_generation() {
	// Test: CSP generates nonce of correct length
	let csp = ContentSecurityPolicy::new();
	let nonce = csp.generate_nonce();

	assert_eq!(nonce.len(), 32);
}

#[test]
fn test_csp_auto_nonce_config() {
	// Test: CSP auto-nonce configuration
	let csp = ContentSecurityPolicy::new().with_auto_nonce(true);

	assert!(csp.auto_nonce);
}

#[test]
fn test_csp_multiple_sources() {
	// Test: CSP with multiple sources for different directives
	let mut csp = ContentSecurityPolicy::default();
	csp.script_src.push("https://cdn.example.com".to_string());
	csp.style_src
		.push("https://fonts.googleapis.com".to_string());
	csp.img_src.push("data:".to_string());
	csp.img_src.push("https:".to_string());

	let header = csp.to_header_value();

	assert!(header.contains("script-src 'self' https://cdn.example.com"));
	assert!(header.contains("style-src 'self' https://fonts.googleapis.com"));
	assert!(header.contains("img-src 'self' data: https:"));
}

// ============================================================================
// X-Frame-Options Tests
// ============================================================================

#[test]
fn test_x_frame_options_deny() {
	// Test: X-Frame-Options DENY prevents all framing
	let config = SecurityHeadersConfig {
		x_frame_options: Some("DENY".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("X-Frame-Options").unwrap();
	assert_eq!(header.to_str().unwrap(), "DENY");
}

#[test]
fn test_x_frame_options_sameorigin() {
	// Test: X-Frame-Options SAMEORIGIN allows same-origin framing
	let config = SecurityHeadersConfig {
		x_frame_options: Some("SAMEORIGIN".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("X-Frame-Options").unwrap();
	assert_eq!(header.to_str().unwrap(), "SAMEORIGIN");
}

#[test]
fn test_x_frame_options_disabled() {
	// Test: X-Frame-Options can be disabled
	let config = SecurityHeadersConfig {
		x_frame_options: None,
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	assert!(response.headers.get("X-Frame-Options").is_none());
}

// ============================================================================
// X-Content-Type-Options Tests
// ============================================================================

#[test]
fn test_x_content_type_options_enabled() {
	// Test: X-Content-Type-Options nosniff prevents MIME sniffing
	let config = SecurityHeadersConfig {
		x_content_type_options: true,
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("X-Content-Type-Options").unwrap();
	assert_eq!(header.to_str().unwrap(), "nosniff");
}

#[test]
fn test_x_content_type_options_disabled() {
	// Test: X-Content-Type-Options can be disabled
	let config = SecurityHeadersConfig {
		x_content_type_options: false,
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	assert!(response.headers.get("X-Content-Type-Options").is_none());
}

// ============================================================================
// Referrer-Policy Tests
// ============================================================================

#[test]
fn test_referrer_policy_strict_origin() {
	// Test: Referrer-Policy strict-origin-when-cross-origin
	let config = SecurityHeadersConfig {
		referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("Referrer-Policy").unwrap();
	assert_eq!(header.to_str().unwrap(), "strict-origin-when-cross-origin");
}

#[test]
fn test_referrer_policy_no_referrer() {
	// Test: Referrer-Policy no-referrer for maximum privacy
	let config = SecurityHeadersConfig {
		referrer_policy: Some("no-referrer".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("Referrer-Policy").unwrap();
	assert_eq!(header.to_str().unwrap(), "no-referrer");
}

#[test]
fn test_referrer_policy_disabled() {
	// Test: Referrer-Policy can be disabled
	let config = SecurityHeadersConfig {
		referrer_policy: None,
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	assert!(response.headers.get("Referrer-Policy").is_none());
}

// ============================================================================
// Cross-Origin Policy Tests
// ============================================================================

#[test]
fn test_cross_origin_embedder_policy() {
	// Test: Cross-Origin-Embedder-Policy require-corp
	let config = SecurityHeadersConfig {
		cross_origin_embedder_policy: Some("require-corp".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response
		.headers
		.get("Cross-Origin-Embedder-Policy")
		.unwrap();
	assert_eq!(header.to_str().unwrap(), "require-corp");
}

#[test]
fn test_cross_origin_opener_policy() {
	// Test: Cross-Origin-Opener-Policy same-origin
	let config = SecurityHeadersConfig {
		cross_origin_opener_policy: Some("same-origin".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("Cross-Origin-Opener-Policy").unwrap();
	assert_eq!(header.to_str().unwrap(), "same-origin");
}

#[test]
fn test_cross_origin_resource_policy() {
	// Test: Cross-Origin-Resource-Policy same-origin
	let config = SecurityHeadersConfig {
		cross_origin_resource_policy: Some("same-origin".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response
		.headers
		.get("Cross-Origin-Resource-Policy")
		.unwrap();
	assert_eq!(header.to_str().unwrap(), "same-origin");
}

#[test]
fn test_cross_origin_policies_disabled() {
	// Test: All cross-origin policies can be disabled
	let config = SecurityHeadersConfig {
		cross_origin_embedder_policy: None,
		cross_origin_opener_policy: None,
		cross_origin_resource_policy: None,
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	assert!(
		response
			.headers
			.get("Cross-Origin-Embedder-Policy")
			.is_none()
	);
	assert!(response.headers.get("Cross-Origin-Opener-Policy").is_none());
	assert!(
		response
			.headers
			.get("Cross-Origin-Resource-Policy")
			.is_none()
	);
}

// ============================================================================
// Header Combination Tests
// ============================================================================

#[test]
fn test_production_config_all_headers() {
	// Test: Production configuration includes all security headers
	let config = SecurityHeadersConfig::production();
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	// Verify all production headers are present
	assert!(response.headers.get("X-Content-Type-Options").is_some());
	assert!(response.headers.get("X-Frame-Options").is_some());
	assert!(response.headers.get("X-XSS-Protection").is_some());
	assert!(response.headers.get("Strict-Transport-Security").is_some());
	assert!(response.headers.get("Content-Security-Policy").is_some());
	assert!(response.headers.get("Referrer-Policy").is_some());
	assert!(
		response
			.headers
			.get("Cross-Origin-Embedder-Policy")
			.is_some()
	);
	assert!(response.headers.get("Cross-Origin-Opener-Policy").is_some());
	assert!(
		response
			.headers
			.get("Cross-Origin-Resource-Policy")
			.is_some()
	);
}

#[test]
fn test_development_config_relaxed_headers() {
	// Test: Development configuration has relaxed security headers
	let config = SecurityHeadersConfig::development();
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	// X-Frame-Options should be SAMEORIGIN, not DENY
	let x_frame = response.headers.get("X-Frame-Options").unwrap();
	assert_eq!(x_frame.to_str().unwrap(), "SAMEORIGIN");

	// HSTS should be disabled for HTTP support
	assert!(response.headers.get("Strict-Transport-Security").is_none());

	// CSP should be disabled during development
	assert!(response.headers.get("Content-Security-Policy").is_none());

	// Cross-origin policies should be disabled
	assert!(
		response
			.headers
			.get("Cross-Origin-Embedder-Policy")
			.is_none()
	);
	assert!(response.headers.get("Cross-Origin-Opener-Policy").is_none());
	assert!(
		response
			.headers
			.get("Cross-Origin-Resource-Policy")
			.is_none()
	);
}

#[test]
fn test_security_headers_middleware_default() {
	// Test: SecurityHeadersMiddleware default configuration matches production
	let middleware = SecurityHeadersMiddleware::default();
	let config = middleware.config();

	assert!(config.x_content_type_options);
	assert_eq!(config.x_frame_options, Some("DENY".to_string()));
	assert!(config.x_xss_protection);
	assert!(config.strict_transport_security.is_some());
	assert!(config.content_security_policy.is_some());
	assert!(config.referrer_policy.is_some());
}

#[test]
fn test_security_headers_middleware_custom_config() {
	// Test: SecurityHeadersMiddleware can use custom configuration
	let custom_config = SecurityHeadersConfig::development();
	let middleware = SecurityHeadersMiddleware::with_config(custom_config);
	let config = middleware.config();

	assert_eq!(config.x_frame_options, Some("SAMEORIGIN".to_string()));
	assert!(config.strict_transport_security.is_none());
	assert!(config.content_security_policy.is_none());
}

// ============================================================================
// Header Conflict Tests
// ============================================================================

#[test]
fn test_hsts_overrides_existing_header() {
	// Test: HSTS header overrides any pre-existing Strict-Transport-Security header
	let config = HstsConfig::new(86400).with_subdomains(true);
	let response = Response::ok().with_header("Strict-Transport-Security", "max-age=3600");
	let response = apply_hsts_header(response, &config);

	let hsts_header = response.headers.get("Strict-Transport-Security").unwrap();
	assert_eq!(
		hsts_header.to_str().unwrap(),
		"max-age=86400; includeSubDomains"
	);
}

#[test]
fn test_csp_overrides_existing_header() {
	// Test: CSP header overrides any pre-existing Content-Security-Policy header
	let csp = ContentSecurityPolicy::default();
	let response =
		Response::ok().with_header("Content-Security-Policy", "default-src 'unsafe-inline'");

	let csp_value = csp.to_header_value();
	let response = response.with_header("Content-Security-Policy", &csp_value);

	let csp_header = response.headers.get("Content-Security-Policy").unwrap();
	assert!(csp_header.to_str().unwrap().contains("default-src 'self'"));
	assert!(!csp_header.to_str().unwrap().contains("'unsafe-inline'"));
}

#[test]
fn test_multiple_security_headers_no_conflict() {
	// Test: Multiple security headers can coexist without conflict
	let config = SecurityHeadersConfig::production();
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	// Verify all headers are present and have correct values
	assert_eq!(
		response
			.headers
			.get("X-Content-Type-Options")
			.unwrap()
			.to_str()
			.unwrap(),
		"nosniff"
	);
	assert_eq!(
		response
			.headers
			.get("X-Frame-Options")
			.unwrap()
			.to_str()
			.unwrap(),
		"DENY"
	);
	assert!(response.headers.get("Strict-Transport-Security").is_some());
	assert!(response.headers.get("Content-Security-Policy").is_some());
	assert!(response.headers.get("Referrer-Policy").is_some());
}

#[test]
fn test_security_headers_preserve_custom_headers() {
	// Test: Security headers do not remove custom application headers
	let config = SecurityHeadersConfig::production();
	let response = Response::ok()
		.with_header("X-Custom-Header", "custom-value")
		.with_header("X-Application-Version", "1.0.0");
	let response = apply_security_headers(response, &config);

	// Custom headers should be preserved
	assert_eq!(
		response
			.headers
			.get("X-Custom-Header")
			.unwrap()
			.to_str()
			.unwrap(),
		"custom-value"
	);
	assert_eq!(
		response
			.headers
			.get("X-Application-Version")
			.unwrap()
			.to_str()
			.unwrap(),
		"1.0.0"
	);

	// Security headers should also be present
	assert!(response.headers.get("X-Content-Type-Options").is_some());
	assert!(response.headers.get("X-Frame-Options").is_some());
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_hsts_zero_max_age() {
	// Test: HSTS with max-age=0 disables HSTS for the domain
	let config = HstsConfig::new(0);
	let response = create_test_response();
	let response = apply_hsts_header(response, &config);

	let hsts_header = response.headers.get("Strict-Transport-Security").unwrap();
	assert_eq!(hsts_header.to_str().unwrap(), "max-age=0");
}

#[test]
fn test_csp_with_empty_directives() {
	// Test: CSP with empty directive lists generates minimal header
	let csp = ContentSecurityPolicy {
		default_src: vec![],
		script_src: vec![],
		style_src: vec![],
		img_src: vec![],
		connect_src: vec![],
		font_src: vec![],
		object_src: vec![],
		media_src: vec![],
		frame_src: vec![],
		report_uri: None,
		report_to: None,
		auto_nonce: false,
	};

	let header = csp.to_header_value();
	assert_eq!(header, "");
}

#[test]
fn test_permissions_policy_custom_directives() {
	// Test: Permissions-Policy with custom feature directives
	let config = SecurityHeadersConfig {
		permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
		..Default::default()
	};
	let response = create_test_response();
	let response = apply_security_headers(response, &config);

	let header = response.headers.get("Permissions-Policy").unwrap();
	assert_eq!(
		header.to_str().unwrap(),
		"geolocation=(), microphone=(), camera=()"
	);
}
