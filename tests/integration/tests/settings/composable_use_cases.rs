use reinhardt_conf::settings::cache::{CacheSettings, HasCacheSettings};
use reinhardt_conf::settings::core_settings::{CoreSettings, HasCoreSettings};
use reinhardt_conf::settings::fragment::SettingsValidation;
use reinhardt_conf::settings::profile::Profile;
use reinhardt_conf::settings::security::SecuritySettings;
use rstest::rstest;

#[rstest]
fn use_case_fragment_accessed_via_has_trait() {
	// Arrange
	struct MySettings {
		core: CoreSettings,
		cache: CacheSettings,
	}

	impl HasCoreSettings for MySettings {
		fn core(&self) -> &CoreSettings {
			&self.core
		}
	}

	impl HasCacheSettings for MySettings {
		fn cache(&self) -> &CacheSettings {
			&self.cache
		}
	}

	fn get_debug(s: &impl HasCoreSettings) -> bool {
		s.core().debug
	}

	fn get_cache_backend(s: &impl HasCacheSettings) -> &str {
		&s.cache().backend
	}

	let settings = MySettings {
		core: CoreSettings {
			secret_key: "test-key".to_string(),
			..Default::default()
		},
		cache: CacheSettings::default(),
	};

	// Act
	let debug = get_debug(&settings);
	let backend = get_cache_backend(&settings);

	// Assert
	assert!(debug, "Default core debug should be true");
	assert_eq!(
		backend, "memory",
		"Default cache backend should be 'memory'"
	);
}

#[rstest]
fn use_case_core_and_security_nested_validation_production() {
	// Arrange
	let valid_settings = CoreSettings {
		secret_key: "a-very-long-secure-random-key-that-is-at-least-32-chars".to_string(),
		debug: false,
		allowed_hosts: vec!["example.com".to_string()],
		security: SecuritySettings {
			secure_ssl_redirect: true,
			session_cookie_secure: true,
			csrf_cookie_secure: true,
			..Default::default()
		},
		..Default::default()
	};

	let invalid_settings = CoreSettings {
		secret_key: "a-very-long-secure-random-key-that-is-at-least-32-chars".to_string(),
		debug: false,
		allowed_hosts: vec!["example.com".to_string()],
		security: SecuritySettings::default(), // ssl_redirect = false
		..Default::default()
	};

	// Act
	let valid_result = valid_settings.validate(&Profile::Production);
	let invalid_result = invalid_settings.validate(&Profile::Production);

	// Assert
	assert!(
		valid_result.is_ok(),
		"Full production settings should validate: {valid_result:?}"
	);
	assert!(
		invalid_result.is_err(),
		"Settings with insecure SecuritySettings should fail production validation"
	);
}
