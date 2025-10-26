//! makemigrations CLI command
//!
//! Creates new migration files based on model changes.

use clap::Parser;
use console::style;
use reinhardt_migrations::{
    autodetector::ProjectState, MakeMigrationsCommand, MakeMigrationsOptions,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "makemigrations")]
#[command(about = "Creates new migration(s) for apps", long_about = None)]
struct Args {
    /// Specify the app label(s) to create migrations for
    #[arg(value_name = "APP_LABEL")]
    app_labels: Vec<String>,

    /// Just show what migrations would be made; don't actually write them
    #[arg(long)]
    dry_run: bool,

    /// Use this name for migration file(s)
    #[arg(short = 'n', long)]
    name: Option<String>,

    /// Exit with a non-zero status if model changes are missing migrations
    #[arg(long)]
    check: bool,

    /// Migration directory (default: ./migrations)
    #[arg(long, default_value = "./migrations")]
    migration_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("{}", style("Checking for model changes...").cyan().bold());

    let _current_state = load_project_state();

    let options = MakeMigrationsOptions {
        app_label: args.app_labels.first().cloned(),
        dry_run: args.dry_run,
        name: args.name,
        migrations_dir: "migrations".to_string(),
    };

    let cmd = MakeMigrationsCommand::new(options);

    let files = cmd.execute();
    {
        let files = &files;
        if files.is_empty() {
            println!("{}", style("No changes detected").green());
        } else {
            println!(
                "\n{}",
                style(format!("Created {} migration(s)", files.len()))
                    .green()
                    .bold()
            );
            for file in files {
                println!("  {}", style(file).dim());
            }
        }
    }

    Ok(())
}

/// Load the current project state from application models
fn load_project_state() -> ProjectState {
    let mut state = ProjectState::new();

    // Load models from the global registry
    // Models are automatically registered via #[derive(Model)] macro using ctor
    use reinhardt_migrations::model_registry::global_registry;

    for model_metadata in global_registry().get_models() {
        let model_state = model_metadata.to_model_state();
        state.add_model(model_state);
    }

    state
}
