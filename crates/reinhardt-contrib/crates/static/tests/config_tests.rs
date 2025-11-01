mod common;

use common::config_helpers;
use std::path::PathBuf;

#[test]
fn test_static_config_default() {
	let config = config_helpers::create_default_config();
	config_helpers::assert_config_properties(&config, &PathBuf::from("static"), "/static/", 0);
}

#[test]
fn test_static_config_custom() {
	let config = config_helpers::create_custom_config(
		PathBuf::from("/var/www/static"),
		"/assets/".to_string(),
		vec![PathBuf::from("/app/static"), PathBuf::from("/app/media")],
	);

	config_helpers::assert_config_properties(
		&config,
		&PathBuf::from("/var/www/static"),
		"/assets/",
		2,
	);
}

#[test]
fn test_static_url_variations() {
	// Test with trailing slash
	let config1 = config_helpers::create_custom_config(
		PathBuf::from("static"),
		"/static/".to_string(),
		vec![],
	);
	assert!(config1.static_url.ends_with('/'));

	// Test without trailing slash
	let config2 = config_helpers::create_custom_config(
		PathBuf::from("static"),
		"/static".to_string(),
		vec![],
	);
	assert!(!config2.static_url.ends_with('/'));
}

#[test]
fn test_static_config_multiple_dirs() {
	let dirs = vec![
		PathBuf::from("/path1"),
		PathBuf::from("/path2"),
		PathBuf::from("/path3"),
	];

	let config = config_helpers::create_custom_config(
		PathBuf::from("static"),
		"/static/".to_string(),
		dirs.clone(),
	);

	config_helpers::assert_config_properties(&config, &PathBuf::from("static"), "/static/", 3);
	assert_eq!(config.staticfiles_dirs, dirs);
}

#[test]
fn test_clone_config() {
	let config1 = config_helpers::create_custom_config(
		PathBuf::from("/var/www/static"),
		"/assets/".to_string(),
		vec![PathBuf::from("/app/static")],
	);

	let config2 = config1.clone();

	assert_eq!(config1.static_root, config2.static_root);
	assert_eq!(config1.static_url, config2.static_url);
	assert_eq!(config1.staticfiles_dirs, config2.staticfiles_dirs);
}
