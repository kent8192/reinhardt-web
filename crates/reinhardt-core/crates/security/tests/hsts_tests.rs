//! HSTS (HTTP Strict Transport Security) tests
//!
//! Tests based on Django's middleware/test_security.py

use reinhardt_security::HstsConfig;

#[test]
fn test_hsts_basic_header() {
	// Test: With max_age=3600, the config generates "max-age=3600"
	let config = HstsConfig::new(3600);
	assert_eq!(config.build_header(), "max-age=3600");
}

#[test]
fn test_hsts_with_zero_max_age() {
	// Test: With max_age=0, the config still generates "max-age=0"
	let config = HstsConfig::new(0);
	assert_eq!(config.build_header(), "max-age=0");
}

#[test]
fn test_hsts_include_subdomains() {
	// Test: With include_subdomains=true, adds "includeSubDomains" directive
	let config = HstsConfig::new(600).with_subdomains(true);
	assert_eq!(config.build_header(), "max-age=600; includeSubDomains");
}

#[test]
fn test_hsts_no_include_subdomains() {
	// Test: With include_subdomains=false, does not add "includeSubDomains" directive
	let config = HstsConfig::new(600).with_subdomains(false);
	assert_eq!(config.build_header(), "max-age=600");
}

#[test]
fn test_hsts_preload() {
	// Test: With preload=true, adds "preload" directive
	let config = HstsConfig::new(10886400).with_preload(true);
	assert_eq!(config.build_header(), "max-age=10886400; preload");
}

#[test]
fn test_hsts_subdomains_and_preload() {
	// Test: With both include_subdomains and preload true, adds both directives
	let config = HstsConfig::new(10886400)
		.with_subdomains(true)
		.with_preload(true);
	assert_eq!(
		config.build_header(),
		"max-age=10886400; includeSubDomains; preload"
	);
}

#[test]
fn test_hsts_no_preload() {
	// Test: With preload=false, does not add "preload" directive
	let config = HstsConfig::new(10886400).with_preload(false);
	assert_eq!(config.build_header(), "max-age=10886400");
}

#[test]
fn test_hsts_default_config() {
	// Test: Default config has sensible defaults (1 year max-age)
	let config = HstsConfig::default();
	assert_eq!(config.max_age, 31536000); // 1 year
	assert!(!config.include_subdomains);
	assert!(!config.preload);
}

#[test]
fn test_hsts_builder_pattern() {
	// Test: Builder pattern works correctly
	let config = HstsConfig::new(31536000)
		.with_subdomains(true)
		.with_preload(false);
	assert_eq!(config.max_age, 31536000);
	assert!(config.include_subdomains);
	assert!(!config.preload);
}

#[test]
fn test_hsts_all_options() {
	// Test: All options can be combined
	let config = HstsConfig::new(63072000) // 2 years
		.with_subdomains(true)
		.with_preload(true);
	let header = config.build_header();
	assert!(header.contains("max-age=63072000"));
	assert!(header.contains("includeSubDomains"));
	assert!(header.contains("preload"));
}

#[test]
fn test_hsts_header_format() {
	// Test: Header format is correct with semicolons and spaces
	let config = HstsConfig::new(3600)
		.with_subdomains(true)
		.with_preload(true);
	let header = config.build_header();
	// Verify format: "max-age=3600; includeSubDomains; preload"
	assert_eq!(header, "max-age=3600; includeSubDomains; preload");
}

#[test]
fn test_hsts_large_max_age() {
	// Test: Large max-age values work correctly
	let config = HstsConfig::new(u64::MAX);
	let header = config.build_header();
	assert!(header.starts_with("max-age="));
}

#[test]
fn test_hsts_clone() {
	// Test: Config can be cloned
	let config1 = HstsConfig::new(3600).with_subdomains(true);
	let config2 = config1.clone();
	assert_eq!(config1.max_age, config2.max_age);
	assert_eq!(config1.include_subdomains, config2.include_subdomains);
	assert_eq!(config1.preload, config2.preload);
}

#[test]
fn test_hsts_debug_format() {
	// Test: Debug format works
	let config = HstsConfig::new(3600);
	let debug_str = format!("{:?}", config);
	assert!(debug_str.contains("HstsConfig"));
}
