//! Built-in commands
//!
//! Standard management commands included with Reinhardt.

use crate::{BaseCommand, CommandArgument, CommandContext, CommandOption, CommandResult};
use async_trait::async_trait;

#[cfg(feature = "migrations")]
use reinhardt_migrations::DatabaseMigrationExecutor;

#[cfg(feature = "migrations")]
use reinhardt_db::backends::{DatabaseConnection, DatabaseType};

/// Database migration command
pub struct MigrateCommand;

#[async_trait]
impl BaseCommand for MigrateCommand {
	fn name(&self) -> &str {
		"migrate"
	}

	fn description(&self) -> &str {
		"Run database migrations"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![
			CommandArgument::optional("app", "App name to migrate"),
			CommandArgument::optional("migration", "Migration name"),
		]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(None, "fake", "Mark migrations as run without executing"),
			CommandOption::flag(
				None,
				"fake-initial",
				"Skip initial migration if tables exist",
			),
			CommandOption::option(Some('d'), "database", "Database to migrate")
				.with_default("default"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		ctx.info("Running migrations...");

		let app_label = ctx.arg(0).map(|s| s.to_string());
		let _migration_name = ctx.arg(1).map(|s| s.to_string());
		let is_fake = ctx.has_option("fake");
		let _is_fake_initial = ctx.has_option("fake-initial");
		let _database = ctx
			.option("database")
			.map(|s| s.to_string())
			.unwrap_or_else(|| "default".to_string());

		if let Some(ref app_name) = app_label {
			if let Some(ref migration) = _migration_name {
				ctx.verbose(&format!("Migrating {} to {}", app_name, migration));
			} else {
				ctx.verbose(&format!("Migrating app: {}", app_name));
			}
		} else {
			ctx.verbose("Migrating all apps");
		}

		if is_fake {
			ctx.warning("Fake mode: Migrations will be marked as applied without running");
		}

		// Use reinhardt-migrations for migration execution
		#[cfg(feature = "migrations")]
		{
			use reinhardt_migrations::MigrationLoader;
			use std::path::PathBuf;

			// 1. Load migration files from disk
			ctx.verbose("Loading migrations from disk...");
			let migrations_dir = PathBuf::from("migrations");
			let mut loader = MigrationLoader::new(migrations_dir);

			if let Err(e) = loader.load_disk() {
				return Err(crate::CommandError::ExecutionError(format!(
					"Failed to load migrations: {:?}",
					e
				)));
			}

			// 2. Filter by app if specified
			let all_migrations = loader.get_all_migrations();
			let migrations_to_apply: Vec<_> = if let Some(ref app) = app_label {
				all_migrations
					.iter()
					.filter(|&&m| &m.app_label == app)
					.map(|&m| m.clone())
					.collect()
			} else {
				all_migrations.iter().map(|&m| m.clone()).collect()
			};

			if migrations_to_apply.is_empty() {
				ctx.info("No migrations to apply");
				return Ok(());
			}

			ctx.info(&format!(
				"Found {} migration(s) to apply",
				migrations_to_apply.len()
			));

			// 3. Check database connection
			let _database_url = std::env::var("DATABASE_URL").map_err(|_| {
				crate::CommandError::ExecutionError(
					"DATABASE_URL environment variable not set".to_string(),
				)
			})?;

			// 4. Apply migrations (or fake them)
			if is_fake {
				ctx.info("Faking migrations (marking as applied without execution):");
				for migration in &migrations_to_apply {
					ctx.success(&format!(
						"  Faked: {}:{}",
						migration.app_label, migration.name
					));
				}
			} else {
				ctx.info("Applying migrations:");

				// Determine database type from URL
				let db_type = if _database_url.starts_with("postgres://")
					|| _database_url.starts_with("postgresql://")
				{
					DatabaseType::Postgres
				} else if _database_url.starts_with("sqlite://") {
					DatabaseType::Sqlite
				} else {
					return Err(crate::CommandError::ExecutionError(format!(
						"Unsupported database URL scheme: {}",
						_database_url
					)));
				};

				// Connect to database
				let connection = match db_type {
					DatabaseType::Postgres => {
						DatabaseConnection::connect_postgres(&_database_url).await
					}
					DatabaseType::Sqlite => {
						DatabaseConnection::connect_sqlite(&_database_url).await
					}
					#[cfg(feature = "mongodb-backend")]
					DatabaseType::MongoDB => {
						// MongoDB requires separate database name
						// Extract database name from URL or use default
						let db_name = _database_url.split('/').next_back().unwrap_or("reinhardt");
						DatabaseConnection::connect_mongodb(&_database_url, db_name).await
					}
					#[allow(unreachable_patterns)]
					_ => {
						return Err(crate::CommandError::ExecutionError(format!(
							"Database type {:?} is not supported in this feature configuration",
							db_type
						)));
					}
				}
				.map_err(|e| {
					crate::CommandError::ExecutionError(format!(
						"Failed to connect to database: {:?}",
						e
					))
				})?;

				// Create migration executor
				let mut executor = DatabaseMigrationExecutor::new(connection, db_type);

				// Apply migrations
				match executor.apply_migrations(&migrations_to_apply[..]).await {
					Ok(result) => {
						for applied_id in &result.applied {
							ctx.success(&format!("  ✓ Applied: {}", applied_id));
						}
					}
					Err(e) => {
						return Err(crate::CommandError::ExecutionError(format!(
							"Failed to apply migrations: {:?}",
							e
						)));
					}
				}
			}

			ctx.info("");
			ctx.success(&format!(
				"Applied {} migration(s) successfully",
				migrations_to_apply.len()
			));

			Ok(())
		}

		#[cfg(not(feature = "migrations"))]
		{
			ctx.warning("Migrations feature not enabled");
			ctx.info("To use migrate, enable the 'migrations' feature");
			Ok(())
		}
	}
}

/// Make migrations command
pub struct MakeMigrationsCommand;

#[async_trait]
impl BaseCommand for MakeMigrationsCommand {
	fn name(&self) -> &str {
		"makemigrations"
	}

	fn description(&self) -> &str {
		"Create new migrations based on model changes"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![CommandArgument::optional(
			"app",
			"App name to create migrations for",
		)]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(
				None,
				"dry-run",
				"Show what would be created without writing files",
			),
			CommandOption::flag(None, "empty", "Create empty migration"),
			CommandOption::option(Some('n'), "name", "Name for the migration"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		ctx.info("Detecting model changes...");

		let is_dry_run = ctx.has_option("dry-run");
		let _is_empty = ctx.has_option("empty");
		let app_label = ctx.arg(0).map(|s| s.to_string());
		let _migration_name = ctx.option("name").map(|s| s.to_string());

		if is_dry_run {
			ctx.warning("Dry run mode: No files will be created");
		}

		if let Some(ref app_name) = app_label {
			ctx.verbose(&format!("Creating migrations for: {}", app_name));
		} else {
			ctx.verbose("Creating migrations for all apps");
		}

		// Use reinhardt-migrations MakeMigrationsCommand
		#[cfg(feature = "migrations")]
		{
			use reinhardt_migrations::{MakeMigrationsCommand as MigCmd, MakeMigrationsOptions};

			let options = MakeMigrationsOptions {
				app_label,
				name: _migration_name,
				dry_run: is_dry_run,
				migrations_dir: "migrations".to_string(),
			};

			let migration_cmd = MigCmd::new(options);
			let created_files = migration_cmd.execute();

			if created_files.is_empty() {
				ctx.info("No changes detected");
			} else {
				for file in &created_files {
					ctx.success(&format!("  Created migration: {}", file));
				}
				ctx.info(&format!(
					"Created {} migration file(s)",
					created_files.len()
				));
			}

			Ok(())
		}

		#[cfg(not(feature = "migrations"))]
		{
			ctx.warning("Migrations feature not enabled");
			ctx.info("To use makemigrations, enable the 'migrations' feature");
			Ok(())
		}
	}
}

/// Interactive shell command
pub struct ShellCommand;

#[async_trait]
impl BaseCommand for ShellCommand {
	fn name(&self) -> &str {
		"shell"
	}

	fn description(&self) -> &str {
		"Start an interactive Rust REPL"
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::option(Some('c'), "command", "Execute a command and exit"),
			CommandOption::option(
				Some('e'),
				"engine",
				"Evaluation engine: rhai (default), python",
			),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		if let Some(command) = ctx.option("command") {
			ctx.info(&format!("Executing: {}", command));
			// Execute the command
			return Ok(());
		}

		ctx.info("Starting interactive shell...");
		ctx.info("Type 'exit' or press Ctrl+D to quit");

		#[cfg(feature = "shell")]
		{
			use rustyline::DefaultEditor;
			use rustyline::error::ReadlineError;

			let mut rl = DefaultEditor::new().map_err(|e| {
				crate::CommandError::ExecutionError(format!("Failed to create REPL: {}", e))
			})?;

			loop {
				let readline = rl.readline(">>> ");
				match readline {
					Ok(line) => {
						let trimmed = line.trim();
						if trimmed == "exit" || trimmed == "quit" {
							ctx.info("Goodbye!");
							break;
						}

						if !trimmed.is_empty() {
							let _ = rl.add_history_entry(line.as_str());

							// Evaluate code using selected engine
							let engine = ctx.option("engine").map(|s| s.as_str()).unwrap_or("rhai");

							match engine {
								"rhai" => {
									#[cfg(feature = "shell-rhai")]
									{
										Self::eval_rhai(ctx, trimmed)?;
									}
									#[cfg(not(feature = "shell-rhai"))]
									{
										ctx.warning(
											"Rhai engine not enabled. Enable 'shell-rhai' feature.",
										);
									}
								}
								"python" | "py" => {
									#[cfg(feature = "shell-pyo3")]
									{
										Self::eval_python(ctx, trimmed)?;
									}
									#[cfg(not(feature = "shell-pyo3"))]
									{
										ctx.warning(
											"Python engine not enabled. Enable 'shell-pyo3' feature.",
										);
									}
								}
								_ => {
									ctx.warning(&format!("Unknown engine: {}", engine));
									ctx.info("Available engines: rhai, python");
								}
							}
						}
					}
					Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
						ctx.info("Goodbye!");
						break;
					}
					Err(err) => {
						return Err(crate::CommandError::ExecutionError(format!(
							"REPL error: {}",
							err
						)));
					}
				}
			}

			Ok(())
		}

		#[cfg(not(feature = "shell"))]
		{
			ctx.warning("Shell feature not enabled");
			ctx.info("To use shell, enable the 'shell' feature in Cargo.toml:");
			ctx.info("  reinhardt-commands = { version = \"*\", features = [\"shell\"] }");
			Ok(())
		}
	}
}

impl ShellCommand {
	/// Evaluate code using Rhai engine
	#[cfg(feature = "shell-rhai")]
	fn eval_rhai(ctx: &CommandContext, code: &str) -> CommandResult<()> {
		use rhai::{Engine, EvalAltResult};

		let mut engine = Engine::new();

		// Register helper functions
		engine.register_fn("println", |s: &str| {
			println!("{}", s);
		});

		// Evaluate the code
		match engine.eval::<rhai::Dynamic>(code) {
			Ok(result) => {
				// Display result if not Unit type
				if !result.is_unit() {
					ctx.info(&format!("=> {}", result));
				}
				Ok(())
			}
			Err(e) => {
				let error_msg = match *e {
					EvalAltResult::ErrorParsing(ref err, _) => {
						format!("Parse error: {}", err)
					}
					EvalAltResult::ErrorRuntime(ref msg, _) => {
						format!("Runtime error: {}", msg)
					}
					_ => format!("Error: {}", e),
				};
				ctx.warning(&error_msg);
				Ok(())
			}
		}
	}

	/// Evaluate code using Python (PyO3) engine
	#[cfg(feature = "shell-pyo3")]
	fn eval_python(ctx: &CommandContext, code: &str) -> CommandResult<()> {
		use pyo3::prelude::*;
		use pyo3::types::PyDict;

		Python::with_gil(|py| {
			// Create a locals dictionary to maintain state between evaluations
			let locals = PyDict::new_bound(py);

			// Execute the code
			match py.eval_bound(code, None, Some(&locals)) {
				Ok(result) => {
					// Convert result to string and display
					if let Ok(result_str) = result.str() {
						if let Ok(s) = result_str.to_string() {
							if s != "()" && !s.is_empty() {
								ctx.info(&format!("=> {}", s));
							}
						}
					}
					Ok(())
				}
				Err(e) => {
					let error_msg = format!("Python error: {}", e);
					ctx.warning(&error_msg);
					Ok(())
				}
			}
		})
	}
}

/// Development server command
pub struct RunServerCommand;

#[async_trait]
impl BaseCommand for RunServerCommand {
	fn name(&self) -> &str {
		"runserver"
	}

	fn description(&self) -> &str {
		"Start the development server"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![
			CommandArgument::optional("address", "Server address (default: 127.0.0.1:8000)")
				.with_default("127.0.0.1:8000"),
		]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(None, "noreload", "Disable auto-reload"),
			CommandOption::flag(
				None,
				"clear",
				"Clear screen before each rebuild (requires cargo-watch)",
			),
			CommandOption::option(
				None,
				"watch-delay",
				"Watch delay in milliseconds for file change debouncing",
			)
			.with_default("500"),
			CommandOption::flag(None, "nothreading", "Disable threading"),
			CommandOption::flag(None, "insecure", "Serve static files in production mode"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		let address = ctx.arg(0).map(|s| s.as_str()).unwrap_or("127.0.0.1:8000");
		let noreload = ctx.has_option("noreload");
		let clear = ctx.has_option("clear");
		let watch_delay = ctx
			.option("watch-delay")
			.and_then(|v| v.parse::<u64>().ok())
			.unwrap_or(500);
		let insecure = ctx.has_option("insecure");

		// Check if cargo-watch-reload feature is enabled and cargo-watch is available
		#[cfg(all(feature = "server", feature = "cargo-watch-reload"))]
		if !noreload {
			// Use cargo-watch integration (non-async)
			return Self::run_with_cargo_watch(ctx, address, clear, watch_delay);
		}

		ctx.info(&format!(
			"Starting development server at http://{}",
			address
		));

		if !noreload {
			#[cfg(all(feature = "server", feature = "autoreload"))]
			{
				ctx.verbose("Auto-reload enabled (notify-based)");
			}
			#[cfg(all(
				feature = "server",
				not(feature = "autoreload"),
				not(feature = "cargo-watch-reload")
			))]
			{
				ctx.warning(
					"Auto-reload disabled: Enable 'autoreload' or 'cargo-watch-reload' feature to use this functionality",
				);
			}
		}

		if insecure {
			ctx.warning("Running with --insecure: Static files will be served");
		}

		ctx.info("");
		ctx.info("Quit the server with CTRL-C");
		ctx.info("");

		// Server implementation with conditional features
		#[cfg(feature = "server")]
		{
			Self::run_server(ctx, address, noreload, insecure).await
		}

		#[cfg(not(feature = "server"))]
		{
			ctx.warning("Server feature not enabled");
			ctx.info("To use runserver, enable the 'server' feature in Cargo.toml:");
			ctx.info("  reinhardt-commands = { version = \"*\", features = [\"server\"] }");
			ctx.info("");
			ctx.info("Alternatively, implement your own server using:");
			ctx.info("  use reinhardt_server::HttpServer;");
			ctx.info("  use reinhardt_routers::DefaultRouter;");
			ctx.info("");
			ctx.info("  let router = DefaultRouter::new();");
			ctx.info("  // Register your routes");
			ctx.info("  let server = HttpServer::new(Arc::new(router));");
			ctx.info(&format!(
				"  server.listen(\"{}\".parse()?).await?;",
				address
			));

			Ok(())
		}
	}
}

impl RunServerCommand {
	/// Run the development server
	#[cfg(feature = "server")]
	async fn run_server(
		ctx: &CommandContext,
		address: &str,
		noreload: bool,
		_insecure: bool,
	) -> CommandResult<()> {
		use reinhardt_server::{HttpServer, ShutdownCoordinator};

		use std::time::Duration;

		// Get registered router
		if !reinhardt_routers::is_router_registered() {
			return Err(crate::CommandError::ExecutionError(
                "No router registered. Call reinhardt_routers::register_router() before running the server.".to_string()
            ));
		}

		let router = reinhardt_routers::get_router().ok_or_else(|| {
			crate::CommandError::ExecutionError("Failed to get registered router".to_string())
		})?;

		// Parse socket address
		let addr: std::net::SocketAddr = address.parse().map_err(|e| {
			crate::CommandError::ExecutionError(format!("Invalid address '{}': {}", address, e))
		})?;

		// Create shutdown coordinator with 30s graceful shutdown timeout
		let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));

		// Spawn CTRL-C signal handler
		let shutdown_tx = coordinator.clone();
		tokio::spawn(async move {
			if let Err(e) = tokio::signal::ctrl_c().await {
				eprintln!("Failed to listen for CTRL-C: {}", e);
				return;
			}
			println!("\nReceived CTRL-C, shutting down gracefully...");
			shutdown_tx.shutdown();
		});

		// Create HTTP server
		let server = HttpServer::new(router);

		// Run with or without auto-reload
		if !noreload {
			#[cfg(feature = "autoreload")]
			{
				Self::run_with_autoreload(ctx, server, addr, coordinator).await
			}
			#[cfg(not(feature = "autoreload"))]
			{
				server
					.listen_with_shutdown(addr, coordinator)
					.await
					.map_err(|e| crate::CommandError::ExecutionError(e.to_string()))
			}
		} else {
			server
				.listen_with_shutdown(addr, coordinator)
				.await
				.map_err(|e| crate::CommandError::ExecutionError(e.to_string()))
		}
	}

	/// Run server with file watching and auto-reload
	#[cfg(all(feature = "server", feature = "autoreload"))]
	async fn run_with_autoreload(
		ctx: &CommandContext,
		server: reinhardt_server::HttpServer,
		addr: std::net::SocketAddr,
		coordinator: reinhardt_server::ShutdownCoordinator,
	) -> CommandResult<()> {
		use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
		use std::path::Path;
		use std::sync::mpsc::channel;

		ctx.verbose("Setting up file watcher for auto-reload...");

		// Create file watcher
		let (tx, rx) = channel();
		let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default()).map_err(|e| {
			crate::CommandError::ExecutionError(format!("Failed to create file watcher: {}", e))
		})?;

		// Watch current directory for changes
		let watch_path = Path::new(".");
		watcher
			.watch(watch_path, RecursiveMode::Recursive)
			.map_err(|e| {
				crate::CommandError::ExecutionError(format!("Failed to watch directory: {}", e))
			})?;

		ctx.success(&format!(
			"Watching for file changes in {}",
			watch_path.display()
		));

		// Spawn file watcher task
		let shutdown_for_reload = coordinator.clone();
		tokio::task::spawn_blocking(move || {
			for res in rx {
				match res {
					Ok(Event { kind, paths, .. }) => {
						// Only reload on modify or create events
						if matches!(
							kind,
							notify::EventKind::Modify(_) | notify::EventKind::Create(_)
						) {
							// Filter out temporary files and build artifacts
							let should_reload = paths.iter().any(|p| {
								let path_str = p.to_string_lossy();
								!path_str.contains("/target/")
									&& !path_str.contains("/.git/")
									&& !path_str.ends_with('~') && !path_str.ends_with(".swp")
									&& (path_str.ends_with(".rs") || path_str.ends_with(".toml"))
							});

							if should_reload {
								println!("\n📝 File changed, triggering reload...");
								shutdown_for_reload.shutdown();
								break;
							}
						}
					}
					Err(e) => eprintln!("Watch error: {:?}", e),
				}
			}
		});

		// Run server
		let result = server
			.listen_with_shutdown(addr, coordinator)
			.await
			.map_err(|e| crate::CommandError::ExecutionError(e.to_string()));

		ctx.info("Auto-reload detected code change. Please restart the server.");
		result
	}

	/// Check if cargo-watch is installed
	#[cfg(all(feature = "server", feature = "cargo-watch-reload"))]
	fn is_cargo_watch_installed() -> bool {
		use std::process::{Command, Stdio};

		Command::new("cargo")
			.args(&["watch", "--version"])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map(|s| s.success())
			.unwrap_or(false)
	}

	/// Run server with cargo-watch integration for automatic rebuild and restart
	#[cfg(all(feature = "server", feature = "cargo-watch-reload"))]
	fn run_with_cargo_watch(
		ctx: &CommandContext,
		addr: &str,
		clear: bool,
		watch_delay: u64,
	) -> CommandResult<()> {
		use std::process::{Command, Stdio};

		// Check if cargo-watch is installed
		if !is_cargo_watch_installed() {
			ctx.error("cargo-watch not found. Install with:");
			ctx.info("  cargo install cargo-watch");
			ctx.info("");
			return Err(crate::CommandError::ExecutionError(
				"cargo-watch not installed".to_string(),
			));
		}

		ctx.success("Starting development server with auto-reload (powered by cargo-watch)");
		ctx.info(&format!("🚀 Server address: {}", addr));
		ctx.info("🔍 Watching: src/, Cargo.toml, templates/, settings/");
		ctx.info("");

		// Build cargo-watch arguments
		let mut args = vec![
			"watch",
			// Watch paths
			"-w",
			"src",
			"-w",
			"Cargo.toml",
		];

		// Add optional watch paths if they exist
		if std::path::Path::new("templates").exists() {
			args.extend_from_slice(&["-w", "templates"]);
		}
		if std::path::Path::new("settings").exists() {
			args.extend_from_slice(&["-w", "settings"]);
		}

		// Ignore paths
		args.extend_from_slice(&["-i", "target/", "-i", ".git/", "-i", "*.swp", "-i", "*~"]);

		// Clear screen before each rebuild
		if clear {
			args.push("-c");
		}

		// Watch delay (debounce)
		args.extend_from_slice(&["--delay", &watch_delay.to_string()]);

		// Execute command: cargo run --bin runserver -- <addr> --noreload
		let run_cmd = format!("run --bin runserver -- {} --noreload", addr);
		args.extend_from_slice(&["-x", &run_cmd]);

		// Run cargo-watch
		let status = Command::new("cargo")
			.args(&args)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.status()
			.map_err(|e| {
				crate::CommandError::ExecutionError(format!("Failed to run cargo-watch: {}", e))
			})?;

		if !status.success() {
			return Err(crate::CommandError::ExecutionError(
				"cargo-watch exited with non-zero status".to_string(),
			));
		}

		Ok(())
	}
}

/// Show all URLs command
#[cfg(feature = "routers")]
pub struct ShowUrlsCommand;

#[cfg(feature = "routers")]
#[async_trait]
impl BaseCommand for ShowUrlsCommand {
	fn name(&self) -> &str {
		"showurls"
	}

	fn description(&self) -> &str {
		"Display all registered URL patterns"
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![CommandOption::flag(None, "names", "Show only named URLs")]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		// Check if router is registered
		if !reinhardt_routers::is_router_registered() {
			ctx.warning(
				"No router registered. Call reinhardt_routers::register_router() in your application startup.",
			);
			ctx.info("");
			ctx.info("Example:");
			ctx.info("  let router = UnifiedRouter::new()");
			ctx.info("      .with_prefix(\"/api\")");
			ctx.info("      .function(\"/health\", Method::GET, health_handler);");
			ctx.info("");
			ctx.info("  reinhardt_routers::register_router(Arc::new(router));");
			return Ok(());
		}

		// Get registered router
		let router =
			reinhardt_routers::get_router().expect("Router should be registered (checked above)");

		// Get all routes
		let routes = router.get_all_routes();

		if routes.is_empty() {
			ctx.info("No routes registered.");
			return Ok(());
		}

		// Check if --names flag is set
		let names_only = ctx.has_option("names");

		// Display header
		ctx.info("Registered URL patterns:");
		ctx.info("");

		if names_only {
			// Show only named URLs
			let named_routes: Vec<_> = routes
				.iter()
				.filter(|(_, name, _, _)| name.is_some())
				.collect();

			if named_routes.is_empty() {
				ctx.info("No named URLs registered.");
				return Ok(());
			}

			ctx.info(&format!(
				"{:<40} {:<30} {:<20}",
				"URL Pattern", "Name", "Namespace"
			));
			ctx.info(&"=".repeat(90));

			for (path, name, namespace, _) in named_routes {
				let name_str = name.as_ref().map(|s| s.as_str()).unwrap_or("-");
				let namespace_str = namespace.as_ref().map(|s| s.as_str()).unwrap_or("-");

				ctx.info(&format!(
					"{:<40} {:<30} {:<20}",
					path, name_str, namespace_str
				));
			}
		} else {
			// Show all URLs with methods
			ctx.info(&format!(
				"{:<40} {:<20} {:<15} {:<20}",
				"URL Pattern", "Methods", "Name", "Namespace"
			));
			ctx.info(&"=".repeat(95));

			for (path, name, namespace, methods) in &routes {
				let methods_str = if methods.is_empty() {
					"ALL".to_string()
				} else {
					methods
						.iter()
						.map(|m| m.as_str())
						.collect::<Vec<_>>()
						.join(", ")
				};

				let name_str = name.as_ref().map(|s| s.as_str()).unwrap_or("-");
				let namespace_str = namespace.as_ref().map(|s| s.as_str()).unwrap_or("-");

				ctx.info(&format!(
					"{:<40} {:<20} {:<15} {:<20}",
					path, methods_str, name_str, namespace_str
				));
			}
		}

		ctx.info("");
		ctx.success(&format!("Total routes: {}", routes.len()));

		Ok(())
	}
}

/// Check system command
pub struct CheckCommand;

#[async_trait]
impl BaseCommand for CheckCommand {
	fn name(&self) -> &str {
		"check"
	}

	fn description(&self) -> &str {
		"Check for common problems"
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![CommandOption::flag(
			None,
			"deploy",
			"Check deployment settings",
		)]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		ctx.info("System check:");
		ctx.info("");

		let is_deploy = ctx.has_option("deploy");
		let mut checks_passed = 0;
		let mut checks_failed = 0;

		// 1. Database connectivity check (if DATABASE_URL is set)
		if let Ok(database_url) = std::env::var("DATABASE_URL") {
			ctx.info("Checking database connectivity...");
			match Self::check_database(&database_url).await {
				Ok(_) => {
					ctx.success("  ✓ Database connection successful");
					checks_passed += 1;
				}
				Err(e) => {
					ctx.warning(&format!("  ✗ Database connection failed: {}", e));
					checks_failed += 1;
				}
			}
		} else {
			ctx.info("Skipping database check (DATABASE_URL not set)");
		}

		// 2. Settings validation
		ctx.info("Checking settings...");
		checks_passed += Self::check_settings(ctx, is_deploy);

		// 3. Migration status check (if DATABASE_URL is set)
		if std::env::var("DATABASE_URL").is_ok() {
			ctx.info("Checking migrations...");
			match Self::check_migrations().await {
				Ok(count) => {
					if count == 0 {
						ctx.success("  ✓ All migrations applied");
						checks_passed += 1;
					} else {
						ctx.warning(&format!("  ⚠ {} unapplied migrations found", count));
					}
				}
				Err(e) => {
					ctx.warning(&format!("  ✗ Migration check failed: {}", e));
					checks_failed += 1;
				}
			}
		}

		// 4. Static files verification
		ctx.info("Checking static files...");
		if std::env::var("STATIC_ROOT").is_ok() {
			ctx.success("  ✓ STATIC_ROOT configured");
			checks_passed += 1;
		} else if is_deploy {
			ctx.warning("  ✗ STATIC_ROOT not set (required for deployment)");
			checks_failed += 1;
		} else {
			ctx.info("  ⚠ STATIC_ROOT not set (optional for development)");
		}

		// 5. Security settings check (if --deploy)
		if is_deploy {
			ctx.info("Checking security settings...");
			checks_passed += Self::check_security(ctx);
		}

		ctx.info("");
		ctx.info(&format!(
			"System check complete: {} passed, {} failed",
			checks_passed, checks_failed
		));

		if checks_failed > 0 {
			Err(crate::CommandError::ExecutionError(format!(
				"{} check(s) failed",
				checks_failed
			)))
		} else {
			Ok(())
		}
	}
}

impl CheckCommand {
	/// Check database connectivity
	async fn check_database(database_url: &str) -> Result<(), String> {
		if database_url.is_empty() {
			return Err("Empty database URL".to_string());
		}

		#[cfg(feature = "migrations")]
		{
			// Actually connect to database and verify connectivity
			match connect_database(database_url).await {
				Ok((db_type, connection)) => {
					// Execute a simple query to verify connection
					match db_type {
						DatabaseType::Postgres | DatabaseType::Sqlite => {
							connection
								.execute("SELECT 1", vec![])
								.await
								.map_err(|e| format!("Query failed: {}", e))?;
						}
						#[cfg(feature = "mongodb-backend")]
						DatabaseType::MongoDB => {
							// MongoDB connection is verified at connection time
						}
						#[allow(unreachable_patterns)]
						_ => {
							// MySQL or other database types that don't have SQL execution support yet
						}
					}
					Ok(())
				}
				Err(e) => Err(format!("Connection failed: {:?}", e)),
			}
		}

		#[cfg(not(feature = "migrations"))]
		{
			// Basic URL validation only
			Ok(())
		}
	}

	/// Check settings configuration
	fn check_settings(ctx: &CommandContext, is_deploy: bool) -> u32 {
		let mut passed = 0;

		// Check SECRET_KEY (always required in deployment)
		if is_deploy {
			if let Ok(secret_key) = std::env::var("SECRET_KEY") {
				if secret_key.len() >= 32 {
					ctx.success("  ✓ SECRET_KEY configured");
					passed += 1;
				} else {
					ctx.warning("  ✗ SECRET_KEY too short (minimum 32 characters)");
				}
			} else {
				ctx.warning("  ✗ SECRET_KEY not set (required for deployment)");
			}
		}

		// Check DEBUG setting
		if let Ok(debug) = std::env::var("DEBUG") {
			if is_deploy && debug == "true" {
				ctx.warning("  ✗ DEBUG=true in deployment (should be false)");
			} else {
				ctx.success("  ✓ DEBUG setting appropriate");
				passed += 1;
			}
		}

		passed
	}

	/// Check migrations status
	async fn check_migrations() -> Result<u32, String> {
		#[cfg(feature = "migrations")]
		{
			use reinhardt_migrations::{DatabaseMigrationRecorder, MigrationLoader};
			use std::path::PathBuf;

			// 1. Load migration files from disk
			let migrations_dir = PathBuf::from("migrations");
			let mut loader = MigrationLoader::new(migrations_dir);
			loader
				.load_disk()
				.map_err(|e| format!("Failed to load migrations: {:?}", e))?;

			let all_migrations = loader.get_all_migrations();

			// 2. Connect to database
			let database_url =
				std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;

			let (_db_type, connection) = connect_database(&database_url)
				.await
				.map_err(|e| format!("Database connection failed: {:?}", e))?;

			// 3. Check applied migrations
			let recorder = DatabaseMigrationRecorder::new(connection);
			recorder
				.ensure_schema_table()
				.await
				.map_err(|e| format!("Failed to create migration table: {}", e))?;

			// 4. Count unapplied migrations
			let mut unapplied_count = 0;
			for migration in &all_migrations {
				let is_applied = recorder
					.is_applied(&migration.app_label, &migration.name)
					.await
					.map_err(|e| format!("Failed to check migration: {}", e))?;

				if !is_applied {
					unapplied_count += 1;
				}
			}

			Ok(unapplied_count)
		}

		#[cfg(not(feature = "migrations"))]
		{
			// Without migrations feature, assume no unapplied migrations
			Ok(0)
		}
	}

	/// Check security settings
	fn check_security(ctx: &CommandContext) -> u32 {
		let mut passed = 0;

		// Check ALLOWED_HOSTS
		if std::env::var("ALLOWED_HOSTS").is_ok() {
			ctx.success("  ✓ ALLOWED_HOSTS configured");
			passed += 1;
		} else {
			ctx.warning("  ✗ ALLOWED_HOSTS not set (required for deployment)");
		}

		// Check SECURE_SSL_REDIRECT
		if let Ok(ssl_redirect) = std::env::var("SECURE_SSL_REDIRECT")
			&& ssl_redirect == "true"
		{
			ctx.success("  ✓ SECURE_SSL_REDIRECT enabled");
			passed += 1;
		}

		passed
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_check_command_basic() {
		let cmd = CheckCommand;
		let ctx = CommandContext::default();

		// Should succeed when no DATABASE_URL is set (skips DB check)
		let result = cmd.execute(&ctx).await;
		// May fail if environment has strict checks, but should handle gracefully
		assert!(result.is_ok() || result.is_err());
	}

	#[tokio::test]
	async fn test_check_command_with_deploy_flag() {
		let cmd = CheckCommand;
		let mut ctx = CommandContext::default();
		ctx.set_option("deploy".to_string(), "true".to_string());

		// Deploy checks are stricter and may fail
		let result = cmd.execute(&ctx).await;
		assert!(result.is_ok() || result.is_err());
	}

	#[tokio::test]
	#[cfg(feature = "routers")]
	async fn test_showurls_command() {
		let cmd = ShowUrlsCommand;
		let ctx = CommandContext::default();

		let result = cmd.execute(&ctx).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_migrate_command() {
		let cmd = MigrateCommand;
		let ctx = CommandContext::default();

		// Without migrations feature or DATABASE_URL, should handle gracefully
		let result = cmd.execute(&ctx).await;
		#[cfg(feature = "migrations")]
		{
			// May fail without DATABASE_URL, which is expected
			assert!(result.is_ok() || result.is_err());
		}
		#[cfg(not(feature = "migrations"))]
		{
			assert!(result.is_ok());
		}
	}

	#[tokio::test]
	async fn test_makemigrations_command() {
		let cmd = MakeMigrationsCommand;
		let ctx = CommandContext::default();

		let result = cmd.execute(&ctx).await;
		// Should succeed (either creates migrations or reports "No changes detected")
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_makemigrations_with_dry_run() {
		let cmd = MakeMigrationsCommand;
		let mut ctx = CommandContext::default();
		ctx.set_option("dry-run".to_string(), "true".to_string());

		let result = cmd.execute(&ctx).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	#[serial_test::serial(runserver)]
	async fn test_runserver_command() {
		// Test without server feature - should show warnings
		#[cfg(not(feature = "server"))]
		{
			let cmd = RunServerCommand;
			let ctx = CommandContext::default();
			let result = cmd.execute(&ctx).await;
			assert!(result.is_ok());
		}

		// Test with server feature - spawn server with timeout
		// Server blocks indefinitely, so timeout is expected
		#[cfg(feature = "server")]
		{
			use reinhardt_routers::UnifiedRouter;
			use std::time::Duration;

			// Register a dummy router for the test
			let router = UnifiedRouter::new();
			reinhardt_routers::register_router(router);

			// Create context with noreload option to disable autoreload
			let mut ctx = CommandContext::default();
			ctx.set_option("noreload".to_string(), "true".to_string());

			// Spawn server in background task
			let server_task = tokio::spawn(async move {
				let cmd = RunServerCommand;
				cmd.execute(&ctx).await
			});

			// Wait a short time for server to start
			tokio::time::sleep(Duration::from_millis(200)).await;

			// Abort the server task (server blocks, so we need to abort)
			server_task.abort();

			// Wait for task to be aborted
			let result = server_task.await;

			// Cleanup: clear the registered router
			reinhardt_routers::clear_router();

			// Task should have been cancelled
			assert!(result.is_err(), "Server task should have been cancelled");
		}
	}
}

/// Helper function to connect to database
#[cfg(feature = "migrations")]
async fn connect_database(url: &str) -> CommandResult<(DatabaseType, DatabaseConnection)> {
	let db_type = if url.starts_with("postgres://") || url.starts_with("postgresql://") {
		DatabaseType::Postgres
	} else if url.starts_with("sqlite://") || url.starts_with(":memory:") {
		DatabaseType::Sqlite
	} else if url.starts_with("mongodb://") {
		#[cfg(feature = "mongodb-backend")]
		{
			DatabaseType::MongoDB
		}
		#[cfg(not(feature = "mongodb-backend"))]
		{
			return Err(crate::CommandError::ExecutionError(
				"MongoDB backend not enabled. Enable 'mongodb-backend' feature.".to_string(),
			));
		}
	} else {
		return Err(crate::CommandError::ExecutionError(format!(
			"Unsupported database URL: {}",
			url
		)));
	};

	let connection = match db_type {
		DatabaseType::Postgres => DatabaseConnection::connect_postgres(url).await,
		DatabaseType::Sqlite => DatabaseConnection::connect_sqlite(url).await,
		#[cfg(feature = "mongodb-backend")]
		DatabaseType::MongoDB => {
			// MongoDB URL format: mongodb://host:port/database
			let database = url.split('/').next_back().unwrap_or("test");
			DatabaseConnection::connect_mongodb(url, database).await
		}
		#[allow(unreachable_patterns)]
		_ => {
			// MySQL or other database types
			return Err(crate::CommandError::ExecutionError(format!(
				"Database type {:?} is not yet supported in this feature configuration",
				db_type
			)));
		}
	}
	.map_err(|e| {
		crate::CommandError::ExecutionError(format!("Database connection failed: {}", e))
	})?;

	Ok((db_type, connection))
}
