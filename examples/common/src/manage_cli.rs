//! Common manage CLI implementation
//!
//! Provides the logic in manage.rs shared among examples.

#[cfg(feature = "manage")]
pub use available::*;

#[cfg(feature = "manage")]
mod available {
	use reinhardt::commands::{
		BaseCommand, CheckCommand, CollectStaticCommand, CommandContext, MakeMigrationsCommand,
		MigrateCommand, RunServerCommand, ShellCommand,
	};
	use std::path::PathBuf;

	pub async fn run_makemigrations(
		app_labels: Vec<String>,
		dry_run: bool,
		name: Option<String>,
		check: bool,
		empty: bool,
		_migration_dir: PathBuf,
		verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut ctx = CommandContext::default();
		ctx.set_verbosity(verbosity);

		if !app_labels.is_empty() {
			for label in app_labels {
				ctx.add_arg(label);
			}
		}

		if dry_run {
			ctx.set_option("dry-run".to_string(), "true".to_string());
		}
		if check {
			ctx.set_option("check".to_string(), "true".to_string());
		}
		if empty {
			ctx.set_option("empty".to_string(), "true".to_string());
		}
		if let Some(n) = name {
			ctx.set_option("name".to_string(), n);
		}

		let cmd = MakeMigrationsCommand;
		cmd.execute(&ctx).await.map_err(|e| e.into())
	}

	pub async fn run_migrate(
		app_label: Option<String>,
		migration_name: Option<String>,
		database: Option<String>,
		fake: bool,
		fake_initial: bool,
		plan: bool,
		_migration_dir: PathBuf,
		verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut ctx = CommandContext::default();
		ctx.set_verbosity(verbosity);

		if let Some(app) = app_label {
			ctx.add_arg(app);
			if let Some(migration) = migration_name {
				ctx.add_arg(migration);
			}
		}

		if fake {
			ctx.set_option("fake".to_string(), "true".to_string());
		}
		if fake_initial {
			ctx.set_option("fake-initial".to_string(), "true".to_string());
		}
		if plan {
			ctx.set_option("plan".to_string(), "true".to_string());
		}
		if let Some(db) = database {
			ctx.set_option("database".to_string(), db);
		}

		let cmd = MigrateCommand;
		cmd.execute(&ctx).await.map_err(|e| e.into())
	}

	pub async fn run_runserver(
		address: String,
		noreload: bool,
		insecure: bool,
		verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut ctx = CommandContext::default();
		ctx.set_verbosity(verbosity);
		ctx.add_arg(address);

		if noreload {
			ctx.set_option("noreload".to_string(), "true".to_string());
		}
		if insecure {
			ctx.set_option("insecure".to_string(), "true".to_string());
		}

		let cmd = RunServerCommand;
		cmd.execute(&ctx).await.map_err(|e| e.into())
	}

	pub async fn run_shell(
		command: Option<String>,
		verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut ctx = CommandContext::default();
		ctx.set_verbosity(verbosity);

		if let Some(cmd_str) = command {
			ctx.set_option("command".to_string(), cmd_str);
		}

		let cmd = ShellCommand;
		cmd.execute(&ctx).await.map_err(|e| e.into())
	}

	pub async fn run_check(
		app_label: Option<String>,
		deploy: bool,
		verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut ctx = CommandContext::default();
		ctx.set_verbosity(verbosity);

		if let Some(app) = app_label {
			ctx.add_arg(app);
		}

		if deploy {
			ctx.set_option("deploy".to_string(), "true".to_string());
		}

		let cmd = CheckCommand;
		cmd.execute(&ctx).await.map_err(|e| e.into())
	}

	pub async fn run_collectstatic(
		_clear: bool,
		_no_input: bool,
		_dry_run: bool,
		_link: bool,
		_ignore: Vec<String>,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		// TODO: Implement collectstatic command integration
		// CollectStaticCommand requires StaticFilesConfig which is not available in examples
		Err("collectstatic command is not yet implemented for examples".into())
	}

	pub async fn run_showurls(
		_names: bool,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		// TODO: Implement showurls command
		// This requires routers feature which is not enabled in example-common
		Err("showurls command is not yet implemented for examples".into())
	}
}

#[cfg(not(feature = "manage"))]
pub use unavailable::*;

#[cfg(not(feature = "manage"))]
mod unavailable {
	use std::path::PathBuf;

	pub async fn run_makemigrations(
		_app_labels: Vec<String>,
		_dry_run: bool,
		_name: Option<String>,
		_check: bool,
		_empty: bool,
		_migration_dir: PathBuf,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}

	pub async fn run_migrate(
		_app_label: Option<String>,
		_migration_name: Option<String>,
		_database: Option<String>,
		_fake: bool,
		_fake_initial: bool,
		_plan: bool,
		_migration_dir: PathBuf,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}

	pub async fn run_runserver(
		_address: String,
		_noreload: bool,
		_insecure: bool,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}

	pub async fn run_shell(
		_command: Option<String>,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}

	pub async fn run_check(
		_app_label: Option<String>,
		_deploy: bool,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}

	pub async fn run_collectstatic(
		_clear: bool,
		_no_input: bool,
		_dry_run: bool,
		_link: bool,
		_ignore: Vec<String>,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}

	pub async fn run_showurls(
		_names: bool,
		_verbosity: u8,
	) -> Result<(), Box<dyn std::error::Error>> {
		Err("reinhardt is not available".into())
	}
}
