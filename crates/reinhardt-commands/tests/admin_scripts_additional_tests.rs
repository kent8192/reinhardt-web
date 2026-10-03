//! Process environment restoration through the shared command-test RAII fixture.

#[path = "support/environment.rs"]
mod environment;

use environment::{EnvVars, env_vars};
use reinhardt_test::TeardownGuard;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[tokio::test]
#[serial(reinhardt_settings)]
async fn test_environment_guard_restores_repeated_writes_and_unwinding(
	#[from(env_vars)] mut env_guard: TeardownGuard<EnvVars>,
	#[from(env_vars)] mut nested_guard: TeardownGuard<EnvVars>,
) {
	// Arrange
	let original = std::env::var_os("REINHARDT_SETTINGS_MODULE");
	env_guard.set("REINHARDT_SETTINGS_MODULE", "settings1");
	assert_eq!(
		std::env::var("REINHARDT_SETTINGS_MODULE").unwrap(),
		"settings1"
	);

	#[cfg(unix)]
	{
		use std::os::unix::ffi::OsStringExt;
		env_guard.set(
			"REINHARDT_SETTINGS_MODULE",
			std::ffi::OsString::from_vec(vec![b's', 0xff]),
		);
	}
	let before_unwind = std::env::var_os("REINHARDT_SETTINGS_MODULE");

	// Act
	let result = std::panic::catch_unwind(move || {
		nested_guard.set("REINHARDT_SETTINGS_MODULE", "settings2");
		nested_guard.set("REINHARDT_SETTINGS_MODULE", "settings3");
		panic!("exercise environment restoration during unwinding");
	});

	// Assert
	assert!(result.is_err());
	assert_eq!(std::env::var_os("REINHARDT_SETTINGS_MODULE"), before_unwind);
	drop(env_guard);
	assert_eq!(std::env::var_os("REINHARDT_SETTINGS_MODULE"), original);
}
