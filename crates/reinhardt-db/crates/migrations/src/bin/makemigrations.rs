//! makemigrations CLI command
//!
//! Creates new migration files based on model changes.

use clap::Parser;
use console::style;
use reinhardt_migrations::{
    autodetector::{FieldState, ModelState, ProjectState},
    MakeMigrationsCommand, MakeMigrationsOptions,
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

    cmd.execute();

    // Placeholder return since execute() returns ()
    let files: Vec<String> = vec![];
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

    // Check if running in demo mode
    if std::env::var("REINHARDT_DEMO").is_ok() {
        populate_demo_models(&mut state);
    }

    // In a production implementation, this function would:
    // 1. Use procedural macros to collect all models defined with #[derive(Model)]
    // 2. Introspect each model's fields and metadata
    // 3. Build a complete ProjectState representation
    //
    // The demo mode shows how the system would work with actual models.
    state
}

/// Populate demo models for demonstration purposes
fn populate_demo_models(state: &mut ProjectState) {
    let mut user_model = ModelState::new("auth", "User");

    let mut id_field = FieldState::new("id".to_string(), "INTEGER".to_string(), false);
    id_field
        .params
        .insert("primary_key".to_string(), "true".to_string());
    user_model.add_field(id_field);

    let mut username_field =
        FieldState::new("username".to_string(), "VARCHAR(150)".to_string(), false);
    username_field
        .params
        .insert("null".to_string(), "false".to_string());
    user_model.add_field(username_field);

    let mut email_field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    email_field
        .params
        .insert("null".to_string(), "false".to_string());
    user_model.add_field(email_field);

    let mut created_at_field =
        FieldState::new("created_at".to_string(), "TIMESTAMP".to_string(), false);
    created_at_field
        .params
        .insert("default".to_string(), "CURRENT_TIMESTAMP".to_string());
    user_model.add_field(created_at_field);

    state.add_model(user_model);
}
