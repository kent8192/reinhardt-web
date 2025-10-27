#[path = "migrations/0001_initial.rs"]
mod _0001_initial;

use reinhardt_migrations::Migration;

/// Returns all migrations in order
pub fn all_migrations() -> Vec<Migration> {
    vec![_0001_initial::migration()]
}
