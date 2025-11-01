//! Security headers tests
//!
//! Tests based on Django's middleware/test_security.py

use reinhardt_security::{ContentSecurityPolicy, SecurityHeadersConfig};

#[test]
fn test_content_type_nosniff_enabled() {
	// Test: With x_content_type_options=true, the config enables nosniff
	let config = SecurityHeadersConfig {
		x_content_type_options: true,
		..Default::default()
	};
	assert!(config.x_content_type_options);
}

#[test]
fn test_content_type_nosniff_disabled() {
	// Test: With x_content_type_options=false, the config disables nosniff
	let config = SecurityHeadersConfig {
		x_content_type_options: false,
		..Default::default()
	};
	assert!(!config.x_content_type_options);
}

#[test]
fn test_x_frame_options_deny() {
	// Test: Default config has X-Frame-Options set to DENY
	let config = SecurityHeadersConfig::default();
	assert_eq!(config.x_frame_options, Some("DENY".to_string()));
}

#[test]
fn test_x_frame_options_sameorigin() {
	// Test: X-Frame-Options can be set to SAMEORIGIN
	let config = SecurityHeadersConfig {
		x_frame_options: Some("SAMEORIGIN".to_string()),
		..Default::default()
	};
	assert_eq!(config.x_frame_options, Some("SAMEORIGIN".to_string()));
}

#[test]
fn test_x_frame_options_none() {
	// Test: X-Frame-Options can be disabled
	let config = SecurityHeadersConfig {
		x_frame_options: None,
		..Default::default()
	};
	assert_eq!(config.x_frame_options, None);
}

#[test]
fn test_xss_protection_enabled() {
	// Test: Default config enables XSS protection
	let config = SecurityHeadersConfig::default();
	assert!(config.x_xss_protection);
}

#[test]
fn test_xss_protection_disabled() {
	// Test: XSS protection can be disabled
	let config = SecurityHeadersConfig {
		x_xss_protection: false,
		..Default::default()
	};
	assert!(!config.x_xss_protection);
}

#[test]
fn test_strict_transport_security_default() {
	// Test: Default config has HSTS enabled
	let config = SecurityHeadersConfig::default();
	assert!(config.strict_transport_security.is_some());
	let hsts = config.strict_transport_security.unwrap();
	assert!(hsts.contains("max-age="));
	assert!(hsts.contains("includeSubDomains"));
}

#[test]
fn test_strict_transport_security_custom() {
	// Test: HSTS can be customized
	let config = SecurityHeadersConfig {
		strict_transport_security: Some("max-age=3600".to_string()),
		..Default::default()
	};
	assert_eq!(
		config.strict_transport_security,
		Some("max-age=3600".to_string())
	);
}

#[test]
fn test_strict_transport_security_disabled() {
	// Test: HSTS can be disabled
	let config = SecurityHeadersConfig {
		strict_transport_security: None,
		..Default::default()
	};
	assert_eq!(config.strict_transport_security, None);
}

#[test]
fn test_security_headers_csp_script() {
	// Test: CSP script-src can be configured
	let csp = ContentSecurityPolicy {
		script_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
		..Default::default()
	};
	let header = csp.to_header_value();
	assert!(header.contains("script-src 'self' 'unsafe-inline'"));
}

#[test]
fn test_security_headers_csp_style() {
	// Test: CSP style-src can be configured
	let csp = ContentSecurityPolicy {
		style_src: vec![
			"'self'".to_string(),
			"https://fonts.googleapis.com".to_string(),
		],
		..Default::default()
	};
	let header = csp.to_header_value();
	assert!(header.contains("style-src 'self' https://fonts.googleapis.com"));
}

#[test]
fn test_security_headers_csp_img() {
	// Test: CSP img-src can be configured
	let csp = ContentSecurityPolicy {
		img_src: vec!["'self'".to_string(), "data:".to_string()],
		..Default::default()
	};
	let header = csp.to_header_value();
	assert!(header.contains("img-src 'self' data:"));
}

#[test]
fn test_security_headers_csp_multiple() {
	// Test: CSP can have multiple directives
	let csp = ContentSecurityPolicy {
		default_src: vec!["'self'".to_string()],
		script_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
		style_src: vec!["'self'".to_string()],
		img_src: vec!["'self'".to_string(), "data:".to_string()],
		connect_src: vec!["'self'".to_string()],
		font_src: vec!["'self'".to_string()],
		object_src: vec!["'none'".to_string()],
		media_src: vec!["'self'".to_string()],
		frame_src: vec!["'self'".to_string()],
		report_uri: None,
		report_to: None,
		auto_nonce: false,
	};
	let header = csp.to_header_value();
	assert!(header.contains("default-src"));
	assert!(header.contains("script-src"));
	assert!(header.contains("style-src"));
	assert!(header.contains("img-src"));
}

#[test]
fn test_csp_new() {
	// Test: CSP::new() creates default configuration
	let csp = ContentSecurityPolicy::new();
	assert_eq!(csp.default_src, vec!["'self'".to_string()]);
	assert_eq!(csp.script_src, vec!["'self'".to_string()]);
	assert_eq!(csp.style_src, vec!["'self'".to_string()]);
	assert_eq!(csp.img_src, vec!["'self'".to_string()]);
}

#[test]
fn test_csp_clone() {
	// Test: CSP can be cloned
	let csp1 = ContentSecurityPolicy::new();
	let csp2 = csp1.clone();
	assert_eq!(csp1.default_src, csp2.default_src);
	assert_eq!(csp1.script_src, csp2.script_src);
}

#[test]
fn test_security_headers_config_default() {
	// Test: Default config has secure defaults
	let config = SecurityHeadersConfig::default();
	assert!(config.x_frame_options.is_some());
	assert!(config.x_content_type_options);
	assert!(config.x_xss_protection);
	assert!(config.strict_transport_security.is_some());
}

#[test]
fn test_security_headers_config_clone() {
	// Test: Config can be cloned
	let config1 = SecurityHeadersConfig::default();
	let config2 = config1.clone();
	assert_eq!(config1.x_frame_options, config2.x_frame_options);
	assert_eq!(
		config1.x_content_type_options,
		config2.x_content_type_options
	);
}

#[test]
fn test_security_headers_config_debug() {
	// Test: Config has Debug trait
	let config = SecurityHeadersConfig::default();
	let debug_str = format!("{:?}", config);
	assert!(debug_str.contains("SecurityHeadersConfig"));
}

#[test]
fn test_csp_empty_src() {
	// Test: CSP with empty sources still generates valid header
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
	// Should be empty or minimal
	assert!(header.is_empty() || !header.contains("'self'"));
}

#[test]
fn test_security_headers_all_disabled() {
	// Test: All security headers can be disabled
	let config = SecurityHeadersConfig {
		x_frame_options: None,
		x_content_type_options: false,
		x_xss_protection: false,
		strict_transport_security: None,
		content_security_policy: None,
		referrer_policy: None,
		permissions_policy: None,
		cross_origin_embedder_policy: None,
		cross_origin_opener_policy: None,
		cross_origin_resource_policy: None,
	};
	assert!(config.x_frame_options.is_none());
	assert!(!config.x_content_type_options);
	assert!(!config.x_xss_protection);
	assert!(config.strict_transport_security.is_none());
	assert!(config.content_security_policy.is_none());
}

#[test]
fn test_security_headers_with_csp() {
	// Test: Security headers config can include CSP
	let csp = ContentSecurityPolicy::new();
	let config = SecurityHeadersConfig {
		content_security_policy: Some(csp),
		..Default::default()
	};
	assert!(config.content_security_policy.is_some());
}
