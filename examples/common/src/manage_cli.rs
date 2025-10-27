//! 共通のmanage CLI実装
//!
//! examples間で共有されるmanage.rsのロジックを提供します。

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub use available::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod available {
    use reinhardt_commands::{
        CheckCommand, CollectStaticCommand, CommandContext, MakeMigrationsCommand,
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

    pub async fn run_showurls(names: bool, verbosity: u8) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(feature = "routers")]
        {
            use console::style;
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
            use console::style;

            eprintln!(
                "{}",
                style("showurls command requires 'routers' feature")
                    .red()
                    .bold()
            );
            eprintln!("Enable it in your Cargo.toml:");
            eprintln!("  [dependencies]");
            eprintln!(
                "  reinhardt-commands = {{ version = \"0.1.0\", features = [\"routers\"] }}"
            );
            std::process::exit(1);
        }
    }
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub use unavailable::*;

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
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

    pub async fn run_showurls(_names: bool, _verbosity: u8) -> Result<(), Box<dyn std::error::Error>> {
        Err("reinhardt is not available".into())
    }
}
