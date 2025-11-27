pub mod _0001_initial;

pub use _0001_initial::Migration as Migration0001Initial;

use reinhardt::db::migrations::{Migration, MigrationProvider, Operation};

/// Polls application migration provider
///
/// Provides all migrations for the polls app in dependency order.
/// Used with `postgres_with_migrations_from::<PollsMigrations>()` fixture.
pub struct PollsMigrations;

impl MigrationProvider for PollsMigrations {
	fn migrations() -> Vec<Migration> {
		vec![Migration {
			name: "0001_initial".to_string(),
			app_label: "polls".to_string(),
			operations: vec![Operation::RunSQL {
				sql: Migration0001Initial::up(),
				reverse_sql: Some(Migration0001Initial::down()),
			}],
			dependencies: vec![],
			replaces: vec![],
			atomic: true,
		}]
	}
}
