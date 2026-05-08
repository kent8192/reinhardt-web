//! Implementation of the `createsuperuser` management command.
//!
//! Creates a superuser account interactively or via CLI arguments.
//! Delegates actual user creation to the registered [`SuperuserCreator`]
//! from `reinhardt-auth`, which supports any user model decorated with
//! `#[user(...)]` + `#[model(...)]` macros.

use console::style;
use dialoguer::{Confirm, Input, Password};

/// Environment variable that supplies the superuser password under `--noinput`.
pub(crate) const SUPERUSER_PASSWORD_ENV: &str = "REINHARDT_SUPERUSER_PASSWORD";

/// Minimum password length, mirrored from the interactive prompt validator.
const MIN_PASSWORD_LEN: usize = 8;

fn validate_email(email: &str) -> bool {
	email.contains('@') && email.contains('.')
}

fn validate_username(username: &str) -> bool {
	!username.is_empty() && username.len() >= 3
}

/// Errors that can occur when resolving a password from CLI flags + env vars.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PasswordResolutionError {
	/// `--no-password` was combined with a non-empty `REINHARDT_SUPERUSER_PASSWORD`.
	MutuallyExclusive,
	/// `--noinput` was passed but no password source was provided.
	MissingEnvVar,
	/// The env-var password is shorter than [`MIN_PASSWORD_LEN`].
	TooShort,
}

impl std::fmt::Display for PasswordResolutionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MutuallyExclusive => write!(
				f,
				"--no-password and {SUPERUSER_PASSWORD_ENV} are mutually exclusive"
			),
			Self::MissingEnvVar => write!(
				f,
				"--noinput requires the {SUPERUSER_PASSWORD_ENV} env var (or use --no-password)"
			),
			Self::TooShort => write!(
				f,
				"{SUPERUSER_PASSWORD_ENV} must be at least {MIN_PASSWORD_LEN} characters"
			),
		}
	}
}

/// Outcome of [`resolve_noninteractive_password`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NoninteractivePassword {
	/// Use this password. Already passed length validation.
	Use(String),
	/// Create the account without a password (`--no-password` path).
	None,
}

/// Resolve the password to use under `--noinput` or `--no-password`.
///
/// Empty env-var strings are treated as unset to avoid surprising the caller
/// when the var is exported but blank in shell config.
pub(crate) fn resolve_noninteractive_password(
	no_password: bool,
	env_password: Option<&str>,
) -> Result<NoninteractivePassword, PasswordResolutionError> {
	let env_pw = env_password.filter(|s| !s.is_empty());

	match (no_password, env_pw) {
		(true, Some(_)) => Err(PasswordResolutionError::MutuallyExclusive),
		(true, None) => Ok(NoninteractivePassword::None),
		(false, Some(pw)) if pw.len() < MIN_PASSWORD_LEN => Err(PasswordResolutionError::TooShort),
		(false, Some(pw)) => Ok(NoninteractivePassword::Use(pw.to_string())),
		(false, None) => Err(PasswordResolutionError::MissingEnvVar),
	}
}

/// Execute the `createsuperuser` management command.
///
/// Collects username, email, and password via interactive prompts or
/// CLI arguments, then delegates to the registered [`SuperuserCreator`].
pub(crate) async fn execute_createsuperuser(
	username: Option<String>,
	email: Option<String>,
	no_password: bool,
	noinput: bool,
	database: Option<String>,
	_verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("{}", style("Creating superuser account").cyan().bold());
	println!();

	// Get username
	let username = if let Some(username) = username {
		if noinput && !validate_username(&username) {
			eprintln!(
				"{}",
				style("Error: Username must be at least 3 characters").red()
			);
			std::process::exit(1);
		}
		username
	} else if noinput {
		eprintln!(
			"{}",
			style("Error: --username is required in non-interactive mode").red()
		);
		std::process::exit(1);
	} else {
		Input::<String>::new()
			.with_prompt("Username")
			.validate_with(|input: &String| -> Result<(), &str> {
				if validate_username(input) {
					Ok(())
				} else {
					Err("Username must be at least 3 characters")
				}
			})
			.interact_text()?
	};

	// Get email
	let email = if let Some(email) = email {
		if noinput && !validate_email(&email) {
			eprintln!("{}", style("Error: Invalid email address").red());
			std::process::exit(1);
		}
		email
	} else if noinput {
		eprintln!(
			"{}",
			style("Error: --email is required in non-interactive mode").red()
		);
		std::process::exit(1);
	} else {
		Input::<String>::new()
			.with_prompt("Email address")
			.validate_with(|input: &String| -> Result<(), &str> {
				if validate_email(input) {
					Ok(())
				} else {
					Err("Invalid email address")
				}
			})
			.interact_text()?
	};

	// Get password.
	//
	// Under `--noinput` or `--no-password`, defer to the pure
	// `resolve_noninteractive_password` helper so the policy is unit-testable.
	let password = if no_password || noinput {
		let env_pw = std::env::var(SUPERUSER_PASSWORD_ENV).ok();
		match resolve_noninteractive_password(no_password, env_pw.as_deref()) {
			Ok(NoninteractivePassword::Use(pw)) => Some(pw),
			Ok(NoninteractivePassword::None) => {
				println!(
					"{}",
					style("Warning: Superuser created without password").yellow()
				);
				None
			}
			Err(e) => {
				eprintln!("{}", style(format!("Error: {e}")).red());
				std::process::exit(1);
			}
		}
	} else {
		let password = Password::new()
			.with_prompt("Password")
			.with_confirmation("Password (again)", "Error: Passwords do not match")
			.validate_with(|input: &String| -> Result<(), &str> {
				if input.len() >= MIN_PASSWORD_LEN {
					Ok(())
				} else {
					Err("Password must be at least 8 characters")
				}
			})
			.interact()?;
		Some(password)
	};

	println!();
	println!("{}", style("Superuser details:").green().bold());
	println!("  Username: {}", style(&username).yellow());
	println!("  Email:    {}", style(&email).yellow());
	if password.is_some() {
		println!("  Password: {}", style("(set)").green());
	} else {
		println!("  Password: {}", style("(not set)").red());
	}

	// Confirmation in interactive mode
	if !noinput {
		println!();
		let confirmed = Confirm::new()
			.with_prompt("Create superuser?")
			.default(true)
			.interact()?;

		if !confirmed {
			println!("{}", style("Superuser creation cancelled").yellow());
			return Ok(());
		}
	}

	// Warn if deprecated --database flag was used (#3186)
	if database.is_some() {
		eprintln!(
			"{}",
			style(
				"Warning: --database flag is deprecated. Database URL is resolved from reinhardt-conf settings."
			)
			.yellow()
		);
	}

	// Create the user via the registered SuperuserCreator
	println!();
	println!("{}", style("Creating user in database...").cyan());

	let creator = reinhardt_auth::get_superuser_creator().ok_or(
		"No SuperuserCreator registered. Ensure your user model has \
		 #[user(hasher = ..., username_field = \"...\", full = true)] and \
		 #[model(...)]. Auto-registration happens automatically.\n\
		 \n\
		 If implementing BaseUser manually, also implement SuperuserInit \
		 and call register_superuser_creator(superuser_creator_for::<YourUser>()) \
		 before execute_from_command_line().\n\
		 See reinhardt_auth::SuperuserInit documentation for details.",
	)?;

	match creator
		.create_superuser(&username, &email, password.as_deref())
		.await
	{
		Ok(()) => {
			println!(
				"{}",
				style("Superuser created successfully!").green().bold()
			);
			println!();
			println!("  Username: {}", style(&username).yellow());
			println!("  Email:    {}", style(&email).yellow());
		}
		Err(e) => {
			eprintln!("{}", style(format!("Error: {}", e)).red().bold());
			std::process::exit(1);
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::{
		NoninteractivePassword, PasswordResolutionError, SUPERUSER_PASSWORD_ENV,
		resolve_noninteractive_password,
	};
	use rstest::rstest;

	#[rstest]
	fn returns_password_when_env_var_meets_length_requirement() {
		// Arrange
		let env = Some("correcthorsebattery");

		// Act
		let result = resolve_noninteractive_password(false, env);

		// Assert
		assert_eq!(
			result,
			Ok(NoninteractivePassword::Use(
				"correcthorsebattery".to_string()
			))
		);
	}

	#[rstest]
	#[case::seven_chars("1234567")]
	#[case::single_char("a")]
	#[case::empty_after_filter("ab")]
	fn rejects_env_var_password_below_minimum_length(#[case] password: &str) {
		// Arrange
		let env = Some(password);

		// Act
		let result = resolve_noninteractive_password(false, env);

		// Assert
		assert_eq!(result, Err(PasswordResolutionError::TooShort));
	}

	#[rstest]
	fn missing_env_var_under_noinput_is_rejected() {
		// Arrange
		let env: Option<&str> = None;

		// Act
		let result = resolve_noninteractive_password(false, env);

		// Assert
		assert_eq!(result, Err(PasswordResolutionError::MissingEnvVar));
	}

	#[rstest]
	fn empty_env_var_is_treated_as_unset() {
		// Arrange — exported but blank shells behave like "not set" to avoid surprise.
		let env = Some("");

		// Act
		let result = resolve_noninteractive_password(false, env);

		// Assert
		assert_eq!(result, Err(PasswordResolutionError::MissingEnvVar));
	}

	#[rstest]
	fn no_password_without_env_var_returns_none_variant() {
		// Arrange
		let env: Option<&str> = None;

		// Act
		let result = resolve_noninteractive_password(true, env);

		// Assert
		assert_eq!(result, Ok(NoninteractivePassword::None));
	}

	#[rstest]
	fn no_password_combined_with_env_var_is_rejected_as_mutually_exclusive() {
		// Arrange
		let env = Some("correcthorsebattery");

		// Act
		let result = resolve_noninteractive_password(true, env);

		// Assert
		assert_eq!(result, Err(PasswordResolutionError::MutuallyExclusive));
	}

	#[rstest]
	fn no_password_with_empty_env_var_returns_none() {
		// Arrange — empty env var is treated as unset, so no conflict.
		let env = Some("");

		// Act
		let result = resolve_noninteractive_password(true, env);

		// Assert
		assert_eq!(result, Ok(NoninteractivePassword::None));
	}

	#[rstest]
	fn error_messages_name_the_env_var_for_operator_clarity() {
		// Arrange — operators copy-paste error text into chat / runbooks; the
		// env-var name must appear so the fix is self-evident.

		// Act + Assert
		let mutex = PasswordResolutionError::MutuallyExclusive.to_string();
		assert!(
			mutex.contains(SUPERUSER_PASSWORD_ENV),
			"MutuallyExclusive must mention {SUPERUSER_PASSWORD_ENV}, got: {mutex}"
		);
		assert!(
			mutex.contains("--no-password"),
			"MutuallyExclusive must mention --no-password, got: {mutex}"
		);

		let missing = PasswordResolutionError::MissingEnvVar.to_string();
		assert!(
			missing.contains(SUPERUSER_PASSWORD_ENV),
			"MissingEnvVar must mention {SUPERUSER_PASSWORD_ENV}, got: {missing}"
		);
		assert!(
			missing.contains("--no-password"),
			"MissingEnvVar must mention --no-password as the escape hatch, got: {missing}"
		);

		let too_short = PasswordResolutionError::TooShort.to_string();
		assert!(
			too_short.contains(SUPERUSER_PASSWORD_ENV),
			"TooShort must mention {SUPERUSER_PASSWORD_ENV}, got: {too_short}"
		);
		assert!(
			too_short.contains('8'),
			"TooShort must mention the 8-char minimum, got: {too_short}"
		);
	}
}
