//! Two-Phase Commit Demo
//!
//! This example demonstrates how to use PostgreSQL and MySQL two-phase commit
//! implementations for distributed transactions.
//!
//! # Setup
//!
//! ## PostgreSQL
//! ```sql
//! -- In postgresql.conf, set:
//! max_prepared_transactions = 100
//!
//! -- Create test database:
//! CREATE DATABASE demo_2pc;
//! ```
//!
//! ## MySQL
//! ```sql
//! -- XA transactions are enabled by default
//! CREATE DATABASE demo_2pc;
//! ```
//!
//! # Running
//!
//! ```bash
//! # PostgreSQL
//! export DATABASE_URL="postgresql://localhost/demo_2pc"
//! cargo run --example two_phase_commit_demo postgres
//!
//! # MySQL
//! export DATABASE_URL="mysql://root@localhost/demo_2pc"
//! cargo run --example two_phase_commit_demo mysql
//! ```

use std::env;

#[cfg(feature = "postgres")]
use reinhardt_db_backends::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;

#[cfg(feature = "mysql")]
use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <postgres|mysql>", args[0]);
        std::process::exit(1);
    }

    let db_type = &args[1];
    match db_type.as_str() {
        #[cfg(feature = "postgres")]
        "postgres" => run_postgres_demo().await,
        #[cfg(feature = "mysql")]
        "mysql" => run_mysql_demo().await,
        _ => {
            eprintln!("Unknown database type: {}", db_type);
            eprintln!("Supported types: postgres, mysql");
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "postgres")]
async fn run_postgres_demo() {
    use sqlx::PgPool;

    println!("=== PostgreSQL Two-Phase Commit Demo ===\n");

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/demo_2pc".to_string());
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Setup
    println!("1. Setting up test table...");
    sqlx::query("DROP TABLE IF EXISTS accounts")
        .execute(&pool)
        .await
        .expect("Failed to drop table");
    sqlx::query("CREATE TABLE accounts (id SERIAL PRIMARY KEY, name TEXT, balance INT)")
        .execute(&pool)
        .await
        .expect("Failed to create table");
    sqlx::query("INSERT INTO accounts (name, balance) VALUES ('Alice', 1000), ('Bob', 1000)")
        .execute(&pool)
        .await
        .expect("Failed to insert initial data");
    println!("   ✓ Table created and initialized\n");

    // Create participant
    let participant = PostgresTwoPhaseParticipant::new(pool.clone());
    let xid = "demo_transfer_001";

    // Begin transaction
    println!("2. Beginning transaction...");
    participant
        .begin(xid)
        .await
        .expect("Failed to begin transaction");
    println!("   ✓ Transaction started\n");

    // Transfer money from Alice to Bob
    println!("3. Transferring $100 from Alice to Bob...");
    sqlx::query("UPDATE accounts SET balance = balance - 100 WHERE name = 'Alice'")
        .execute(&pool)
        .await
        .expect("Failed to debit Alice");
    sqlx::query("UPDATE accounts SET balance = balance + 100 WHERE name = 'Bob'")
        .execute(&pool)
        .await
        .expect("Failed to credit Bob");
    println!("   ✓ Transfer operations executed\n");

    // Prepare phase
    println!("4. Preparing transaction...");
    participant
        .prepare(xid)
        .await
        .expect("Failed to prepare transaction");
    println!("   ✓ Transaction prepared\n");

    // Check prepared transactions
    println!("5. Querying prepared transactions...");
    let prepared_list = participant
        .list_prepared_transactions()
        .await
        .expect("Failed to list prepared transactions");
    for txn in &prepared_list {
        println!("   - Transaction: {}", txn.gid);
        println!("     Prepared at: {}", txn.prepared);
        println!("     Owner: {}", txn.owner);
    }
    println!();

    // Commit phase
    println!("6. Committing transaction...");
    participant
        .commit(xid)
        .await
        .expect("Failed to commit transaction");
    println!("   ✓ Transaction committed\n");

    // Verify final balances
    println!("7. Verifying final balances...");
    let rows = sqlx::query("SELECT name, balance FROM accounts ORDER BY name")
        .fetch_all(&pool)
        .await
        .expect("Failed to query accounts");
    for row in rows {
        let name: String = row.get("name");
        let balance: i32 = row.get("balance");
        println!("   - {}: ${}", name, balance);
    }
    println!();

    // Cleanup
    println!("8. Cleaning up...");
    sqlx::query("DROP TABLE accounts")
        .execute(&pool)
        .await
        .expect("Failed to drop table");
    println!("   ✓ Cleanup complete\n");

    println!("=== Demo Complete ===");
}

#[cfg(feature = "mysql")]
async fn run_mysql_demo() {
    use sqlx::MySqlPool;

    println!("=== MySQL Two-Phase Commit Demo ===\n");

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "mysql://root@localhost/demo_2pc".to_string());
    let pool = MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to MySQL");

    // Setup
    println!("1. Setting up test table...");
    sqlx::query("DROP TABLE IF EXISTS accounts")
        .execute(&pool)
        .await
        .expect("Failed to drop table");
    sqlx::query("CREATE TABLE accounts (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(255), balance INT)")
        .execute(&pool)
        .await
        .expect("Failed to create table");
    sqlx::query("INSERT INTO accounts (name, balance) VALUES ('Alice', 1000), ('Bob', 1000)")
        .execute(&pool)
        .await
        .expect("Failed to insert initial data");
    println!("   ✓ Table created and initialized\n");

    // Create participant
    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "demo_transfer_001";

    // Begin XA transaction
    println!("2. Starting XA transaction...");
    participant
        .begin(xid)
        .await
        .expect("Failed to begin XA transaction");
    println!("   ✓ XA transaction started\n");

    // Transfer money from Alice to Bob
    println!("3. Transferring $100 from Alice to Bob...");
    sqlx::query("UPDATE accounts SET balance = balance - 100 WHERE name = 'Alice'")
        .execute(&pool)
        .await
        .expect("Failed to debit Alice");
    sqlx::query("UPDATE accounts SET balance = balance + 100 WHERE name = 'Bob'")
        .execute(&pool)
        .await
        .expect("Failed to credit Bob");
    println!("   ✓ Transfer operations executed\n");

    // End XA transaction
    println!("4. Ending XA transaction...");
    participant.end(xid).await.expect("Failed to end XA");
    println!("   ✓ XA transaction ended\n");

    // Prepare phase
    println!("5. Preparing XA transaction...");
    participant
        .prepare(xid)
        .await
        .expect("Failed to prepare XA transaction");
    println!("   ✓ XA transaction prepared\n");

    // Check prepared transactions
    println!("6. Querying prepared XA transactions...");
    let prepared_list = participant
        .list_prepared_transactions()
        .await
        .expect("Failed to list XA transactions");
    for txn in &prepared_list {
        println!("   - XA Transaction: {}", txn.xid);
        println!("     Format ID: {}", txn.format_id);
        println!("     GTRID Length: {}", txn.gtrid_length);
    }
    println!();

    // Commit phase
    println!("7. Committing XA transaction...");
    participant
        .commit(xid)
        .await
        .expect("Failed to commit XA transaction");
    println!("   ✓ XA transaction committed\n");

    // Verify final balances
    println!("8. Verifying final balances...");
    let rows = sqlx::query("SELECT name, balance FROM accounts ORDER BY name")
        .fetch_all(&pool)
        .await
        .expect("Failed to query accounts");
    for row in rows {
        let name: String = row.get("name");
        let balance: i32 = row.get("balance");
        println!("   - {}: ${}", name, balance);
    }
    println!();

    // Cleanup
    println!("9. Cleaning up...");
    sqlx::query("DROP TABLE accounts")
        .execute(&pool)
        .await
        .expect("Failed to drop table");
    println!("   ✓ Cleanup complete\n");

    println!("=== Demo Complete ===");
}

#[cfg(not(any(feature = "postgres", feature = "mysql")))]
async fn run_postgres_demo() {
    eprintln!("PostgreSQL feature not enabled. Rebuild with --features postgres");
}

#[cfg(not(any(feature = "postgres", feature = "mysql")))]
async fn run_mysql_demo() {
    eprintln!("MySQL feature not enabled. Rebuild with --features mysql");
}
