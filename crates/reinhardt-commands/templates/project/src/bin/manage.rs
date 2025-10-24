//! Reinhardt Project Management CLI
//!
//! This is the project-specific management command interface (equivalent to Django's manage.py).
//! It provides commands for database migrations, user management, development server, and more.

use clap::{Parser, Subcommand};
use colored::Colorize;
use reinhardt_commands::{
    CheckCommand, CollectStaticCommand, CommandContext, CommandRegistry, MakeMigrationsCommand,
    MigrateCommand, RunServerCommand, ShellCommand,
};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "manage")]
#[command(about = "Reinhardt project management interface", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level (can be repeated for more output)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbosity: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Create new migrations based on model changes
    Makemigrations {
        /// App labels to create migrations for
        #[arg(value_name = "APP_LABEL")]
        app_labels: Vec<String>,

        /// Dry run - don't actually write files
        #[arg(long)]
        dry_run: bool,

        /// Migration name
        #[arg(short = 'n', long, value_name = "NAME")]
        name: Option<String>,

        /// Check if migrations are missing
        #[arg(long)]
        check: bool,

        /// Create empty migration
        #[arg(long)]
        empty: bool,

        /// Migration directory
        #[arg(long, default_value = "./migrations")]
        migration_dir: PathBuf,
    },

    /// Apply database migrations
    Migrate {
        /// App label to migrate
        #[arg(value_name = "APP_LABEL")]
        app_label: Option<String>,

        /// Migration name to migrate to
        #[arg(value_name = "MIGRATION_NAME")]
        migration_name: Option<String>,

        /// Database connection string
        #[arg(long, value_name = "DATABASE")]
        database: Option<String>,

        /// Fake migration (mark as applied without running)
        #[arg(long)]
        fake: bool,

        /// Fake initial migration only
        #[arg(long)]
        fake_initial: bool,

        /// Show migration plan without applying
        #[arg(long)]
        plan: bool,

        /// Migration directory
        #[arg(long, default_value = "./migrations")]
        migration_dir: PathBuf,
    },

    /// Start the development server
    Runserver {
        /// Server address (default: 127.0.0.1:8000)
        #[arg(value_name = "ADDRESS", default_value = "127.0.0.1:8000")]
        address: String,

        /// Disable auto-reload
        #[arg(long)]
        noreload: bool,

        /// Serve static files in development mode
        #[arg(long)]
        insecure: bool,
    },

    /// Run an interactive Rust shell (REPL)
    Shell {
        /// Execute a command and exit
        #[arg(short = 'c', long, value_name = "COMMAND")]
        command: Option<String>,
    },

    /// Check the project for common issues
    Check {
        /// Check specific app
        #[arg(value_name = "APP_LABEL")]
        app_label: Option<String>,

        /// Deploy check (stricter checks)
        #[arg(long)]
        deploy: bool,
    },

    /// Collect static files into STATIC_ROOT
    Collectstatic {
        /// Clear existing files before collecting
        #[arg(long)]
        clear: bool,

        /// Do not prompt for confirmation
        #[arg(long)]
        no_input: bool,

        /// Do not actually collect, just show what would be collected
        #[arg(long)]
        dry_run: bool,

        /// Create symbolic links instead of copying files
        #[arg(long)]
        link: bool,

        /// Ignore file patterns (glob)
        #[arg(long, value_name = "PATTERN")]
        ignore: Vec<String>,
    },

    /// Display all registered URL patterns
    Showurls {
        /// Show only named URLs
        #[arg(long)]
        names: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Makemigrations {
            app_labels,
            dry_run,
            name,
            check,
            empty,
            migration_dir,
        } => {
            run_makemigrations(
                app_labels,
                dry_run,
                name,
                check,
                empty,
                migration_dir,
                cli.verbosity,
            )
            .await
        }
        Commands::Migrate {
            app_label,
            migration_name,
            database,
            fake,
            fake_initial,
            plan,
            migration_dir,
        } => {
            run_migrate(
                app_label,
                migration_name,
                database,
                fake,
                fake_initial,
                plan,
                migration_dir,
                cli.verbosity,
            )
            .await
        }
        Commands::Runserver {
            address,
            noreload,
            insecure,
        } => run_runserver(address, noreload, insecure, cli.verbosity).await,
        Commands::Shell { command } => run_shell(command, cli.verbosity).await,
        Commands::Check { app_label, deploy } => run_check(app_label, deploy, cli.verbosity).await,
        Commands::Collectstatic {
            clear,
            no_input,
            dry_run,
            link,
            ignore,
        } => run_collectstatic(clear, no_input, dry_run, link, ignore, cli.verbosity).await,
        Commands::Showurls { names } => run_showurls(names, cli.verbosity).await,
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

async fn run_makemigrations(
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

    // Set arguments
    if !app_labels.is_empty() {
        for label in app_labels {
            ctx.add_arg(label);
        }
    }

    // Set options
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

async fn run_migrate(
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

    // Set arguments
    if let Some(app) = app_label {
        ctx.add_arg(app);
        if let Some(migration) = migration_name {
            ctx.add_arg(migration);
        }
    }

    // Set options
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

async fn run_runserver(
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

async fn run_shell(
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

async fn run_check(
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

async fn run_collectstatic(
    clear: bool,
    no_input: bool,
    dry_run: bool,
    link: bool,
    ignore: Vec<String>,
    verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = CommandContext::default();
    ctx.set_verbosity(verbosity);

    if clear {
        ctx.set_option("clear".to_string(), "true".to_string());
    }
    if no_input {
        ctx.set_option("no-input".to_string(), "true".to_string());
    }
    if dry_run {
        ctx.set_option("dry-run".to_string(), "true".to_string());
    }
    if link {
        ctx.set_option("link".to_string(), "true".to_string());
    }
    if !ignore.is_empty() {
        ctx.set_option_multi("ignore".to_string(), ignore);
    }

    let cmd = CollectStaticCommand;
    cmd.execute(&ctx).await.map_err(|e| e.into())
}

async fn run_showurls(names: bool, verbosity: u8) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "routers")]
    {
        use reinhardt_commands::builtin::ShowUrlsCommand;

        let mut ctx = CommandContext::default();
        ctx.set_verbosity(verbosity);

        if names {
            ctx.set_option("names".to_string(), "true".to_string());
        }

        let cmd = ShowUrlsCommand;
        cmd.execute(&ctx).await.map_err(|e| e.into())
    }

    #[cfg(not(feature = "routers"))]
    {
        eprintln!(
            "{}",
            "showurls command requires 'routers' feature".red().bold()
        );
        eprintln!("Enable it in your Cargo.toml:");
        eprintln!("  [dependencies]");
        eprintln!("  reinhardt-commands = {{ version = \"0.1.0\", features = [\"routers\"] }}");
        std::process::exit(1);
    }
}
