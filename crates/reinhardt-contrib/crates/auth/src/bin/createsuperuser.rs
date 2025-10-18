//! createsuperuser CLI command
//!
//! Creates a superuser account interactively.

use clap::Parser;
use console::style;
use dialoguer::{Confirm, Input, Password};

#[cfg(feature = "database")]
use sqlx::{Pool, Sqlite, SqlitePool};

#[cfg(feature = "database")]
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

#[derive(Parser, Debug)]
#[command(name = "createsuperuser")]
#[command(about = "Creates a superuser account", long_about = None)]
struct Args {
    /// Username for the superuser
    #[arg(long, value_name = "USERNAME")]
    username: Option<String>,

    /// Email address for the superuser
    #[arg(long, value_name = "EMAIL")]
    email: Option<String>,

    /// Skip the password prompt (use with caution)
    #[arg(long)]
    no_password: bool,

    /// Non-interactive mode (requires --username and --email)
    #[arg(long)]
    noinput: bool,

    /// Database connection string
    #[arg(long, value_name = "DATABASE", default_value = "sqlite::memory:")]
    database: String,
}

fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

fn validate_username(username: &str) -> bool {
    !username.is_empty() && username.len() >= 3
}

#[cfg(feature = "database")]
async fn create_user_in_database(
    pool: &Pool<Sqlite>,
    username: &str,
    email: &str,
    password: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    /// // Create users table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS auth_user (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL,
            password_hash TEXT,
            is_staff BOOLEAN NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            is_superuser BOOLEAN NOT NULL DEFAULT 0,
            date_joined DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    /// // Hash the password if provided
    let password_hash = if let Some(pwd) = password {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(pwd.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {}", e))?;
        Some(hash.to_string())
    } else {
        None
    };

    /// // Insert the superuser
    sqlx::query(
        r#"
        INSERT INTO auth_user (username, email, password_hash, is_staff, is_superuser)
        VALUES (?, ?, ?, 1, 1)
        "#,
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("{}", style("Creating superuser account").cyan().bold());
    println!();

    /// // Get username
    let username = if let Some(username) = args.username {
        if args.noinput && !validate_username(&username) {
            eprintln!(
                "{}",
                style("Error: Username must be at least 3 characters").red()
            );
            std::process::exit(1);
        }
        username
    } else if args.noinput {
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

    /// // Get email
    let email = if let Some(email) = args.email {
        if args.noinput && !validate_email(&email) {
            eprintln!("{}", style("Error: Invalid email address").red());
            std::process::exit(1);
        }
        email
    } else if args.noinput {
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

    /// // Get password
    let password = if args.no_password {
        println!(
            "{}",
            style("Warning: Superuser created without password").yellow()
        );
        None
    } else if args.noinput {
        eprintln!(
            "{}",
            style("Error: Cannot set password in non-interactive mode without --no-password").red()
        );
        std::process::exit(1);
    } else {
        let password = Password::new()
            .with_prompt("Password")
            .with_confirmation("Password (again)", "Error: Passwords do not match")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.len() >= 8 {
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

    /// // Confirmation
    if !args.noinput {
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

    /// // Create the user in the database
    println!();
    println!("{}", style("Creating user in database...").cyan());

    #[cfg(feature = "database")]
    {
        match create_database_user(&args.database, &username, &email, password.as_deref()).await {
            Ok(_) => {
                println!(
                    "{}",
                    style("✓ Superuser created successfully!").green().bold()
                );
                println!();
                println!("  Database: {}", style(&args.database).dim());
                println!("  Username: {}", style(&username).yellow());
                println!("  Email:    {}", style(&email).yellow());
            }
            Err(e) => {
                eprintln!("{}", style(format!("Error: {}", e)).red().bold());
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(feature = "database"))]
    {
        println!();
        println!(
            "{}",
            style("✓ Superuser validation completed!").green().bold()
        );
        println!();
        println!("{}", style("Note: Database feature not enabled").yellow());
        println!(
            "{}",
            style("Rebuild with --features database to enable database integration").dim()
        );
        println!();
        println!("  Username: {}", style(&username).yellow());
        println!("  Email:    {}", style(&email).yellow());
    }

    Ok(())
}

#[cfg(feature = "database")]
async fn create_database_user(
    database_url: &str,
    username: &str,
    email: &str,
    password: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    /// // Parse database URL
    let options = SqliteConnectOptions::from_str(database_url)?;

    /// // Create connection pool
    let pool = SqlitePool::connect_with(options).await?;

    /// // Create user
    create_user_in_database(&pool, username, email, password).await?;

    /// // Close pool
    pool.close().await;

    Ok(())
}
