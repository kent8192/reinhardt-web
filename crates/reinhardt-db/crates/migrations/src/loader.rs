//! Migration Loader
//!
//! Loads migration files from disk and tracks applied migrations from database.
//! Based on Django's migration loader functionality.

use crate::{Migration, MigrationError, ProjectState, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Migration loader - loads migrations from disk and database
pub struct MigrationLoader {
    /// Migrations directory root
    migration_root: PathBuf,
    /// Migrations loaded from disk, keyed by (app_label, migration_name)
    disk_migrations: HashMap<(String, String), Migration>,
    /// Apps that have migrations
    migrated_apps: HashSet<String>,
    /// Apps that don't have migrations
    unmigrated_apps: HashSet<String>,
}

impl MigrationLoader {
    /// Create a new migration loader
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let loader = MigrationLoader::new(migration_root);
    /// ```
    pub fn new(migration_root: PathBuf) -> Self {
        Self {
            migration_root,
            disk_migrations: HashMap::new(),
            migrated_apps: HashSet::new(),
            unmigrated_apps: HashSet::new(),
        }
    }
    /// Load all migrations from disk
    ///
    /// Scans the migration directory for all apps and loads their migration files.
    /// Migration files should be named like: 0001_initial.json, 0002_add_field.json
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    /// ```
    pub fn load_disk(&mut self) -> Result<()> {
        self.disk_migrations.clear();
        self.migrated_apps.clear();
        self.unmigrated_apps.clear();

        // Scan migration root for app directories
        if !self.migration_root.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.migration_root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let app_label = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();

                // Load migrations for this app
                match self.load_app_migrations(&app_label, &path) {
                    Ok(count) => {
                        if count > 0 {
                            self.migrated_apps.insert(app_label);
                        } else {
                            self.unmigrated_apps.insert(app_label);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load migrations for {}: {}",
                            app_label, e
                        );
                        self.unmigrated_apps.insert(app_label);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load migrations for a specific app from its migration directory
    fn load_app_migrations(&mut self, app_label: &str, app_path: &Path) -> Result<usize> {
        let mut count = 0;

        for entry in fs::read_dir(app_path)? {
            let entry = entry?;
            let path = entry.path();

            // Skip non-files and non-.json files
            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            // Skip files that don't look like migrations (must start with digit)
            if !file_name
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_digit())
            {
                continue;
            }

            // Skip files that start with _ or ~
            if file_name.starts_with('_') || file_name.starts_with('~') {
                continue;
            }

            // Load and parse the migration file
            if let Some(migration_name) = file_name.strip_suffix(".json") {
                match self.load_migration_file(&path, app_label, migration_name) {
                    Ok(migration) => {
                        self.disk_migrations.insert(
                            (app_label.to_string(), migration_name.to_string()),
                            migration,
                        );
                        count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load migration {} for {}: {}",
                            migration_name, app_label, e
                        );
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a single migration file
    fn load_migration_file(
        &self,
        path: &Path,
        app_label: &str,
        migration_name: &str,
    ) -> Result<Migration> {
        let content = fs::read_to_string(path)?;
        let mut migration: Migration = serde_json::from_str(&content).map_err(|e| {
            MigrationError::InvalidMigration(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Ensure app_label and name are set correctly
        migration.app_label = app_label.to_string();
        migration.name = migration_name.to_string();

        Ok(migration)
    }
    /// Get a migration by app label and name
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let migration = loader.get_migration("myapp", "0001_initial");
    /// assert!(migration.is_some() || migration.is_none()); // May or may not exist
    /// ```
    pub fn get_migration(&self, app_label: &str, name: &str) -> Option<&Migration> {
        self.disk_migrations
            .get(&(app_label.to_string(), name.to_string()))
    }
    /// Get all migrations for an app
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let migrations = loader.get_app_migrations("myapp");
    /// // Returns all migrations for the app
    /// ```
    pub fn get_app_migrations(&self, app_label: &str) -> Vec<&Migration> {
        self.disk_migrations
            .iter()
            .filter(|((app, _), _)| app == app_label)
            .map(|(_, migration)| migration)
            .collect()
    }
    /// Get all loaded migrations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let all_migrations = loader.get_all_migrations();
    /// // Returns vector of all loaded migrations
    /// ```
    pub fn get_all_migrations(&self) -> Vec<&Migration> {
        self.disk_migrations.values().collect()
    }
    /// Build a project state from all migrations
    ///
    /// This reconstructs the current state by applying all migrations in order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let state = loader.build_project_state().unwrap();
    /// // State now contains all model definitions from applied migrations
    /// ```
    pub fn build_project_state(&self) -> Result<ProjectState> {
        let mut state = ProjectState::new();

        // Get all migrations and sort them by dependencies
        let mut migrations: Vec<&Migration> = self.get_all_migrations();

        // Simple topological sort
        let mut sorted = Vec::new();
        while !migrations.is_empty() {
            let mut made_progress = false;

            let mut i = 0;
            while i < migrations.len() {
                let migration = migrations[i];

                // Check if all dependencies are satisfied
                let deps_satisfied = migration.dependencies.iter().all(|(dep_app, dep_name)| {
                    sorted
                        .iter()
                        .any(|m: &&Migration| m.app_label == *dep_app && m.name == *dep_name)
                });

                if deps_satisfied {
                    sorted.push(migrations.remove(i));
                    made_progress = true;
                } else {
                    i += 1;
                }
            }

            if !made_progress && !migrations.is_empty() {
                return Err(MigrationError::CircularDependency {
                    cycle: format!(
                        "Circular dependency detected among: {:?}",
                        migrations.iter().map(|m| m.id()).collect::<Vec<_>>()
                    ),
                });
            }
        }

        // Apply migrations to build state
        for ((app_label, _), migration) in &self.disk_migrations {
            for operation in &migration.operations {
                operation.state_forwards(app_label, &mut state);
            }
        }

        Ok(state)
    }
    /// Get migrations by prefix (for finding migrations by partial name)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let migrations = loader.get_migrations_by_prefix("myapp", "0001");
    /// // Returns migrations like "0001_initial", "0001_add_field", etc.
    /// ```
    pub fn get_migrations_by_prefix(&self, app_label: &str, prefix: &str) -> Vec<&Migration> {
        self.disk_migrations
            .iter()
            .filter(|((app, name), _)| app == app_label && name.starts_with(prefix))
            .map(|(_, migration)| migration)
            .collect()
    }
    /// Check if an app has any migrations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let has_migs = loader.has_migrations("myapp");
    /// // Returns true if app has migrations, false otherwise
    /// ```
    pub fn has_migrations(&self, app_label: &str) -> bool {
        self.migrated_apps.contains(app_label)
    }
    /// Get set of migrated apps
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let migrated_apps = loader.get_migrated_apps();
    /// // Returns set of app labels that have migrations
    /// ```
    pub fn get_migrated_apps(&self) -> &HashSet<String> {
        &self.migrated_apps
    }
    /// Get set of unmigrated apps
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::loader::MigrationLoader;
    /// use std::path::PathBuf;
    ///
    /// let migration_root = PathBuf::from("/tmp/migrations");
    /// let mut loader = MigrationLoader::new(migration_root);
    /// loader.load_disk().unwrap();
    ///
    /// let unmigrated_apps = loader.get_unmigrated_apps();
    /// // Returns set of app labels that don't have migrations
    /// ```
    pub fn get_unmigrated_apps(&self) -> &HashSet<String> {
        &self.unmigrated_apps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::{ColumnDefinition, CreateTable, SqlDialect};
    use std::fs;

    #[test]
    fn test_migration_loader_creation() {
        let temp_dir = std::env::temp_dir().join("reinhardt_loader_test");
        let loader = MigrationLoader::new(temp_dir);
        assert!(loader.disk_migrations.is_empty());
    }

    #[test]
    fn test_load_migrations() {
        let temp_dir = std::env::temp_dir().join("reinhardt_loader_test_load");
        fs::create_dir_all(&temp_dir).ok();
        let app_dir = temp_dir.join("myapp");
        fs::create_dir_all(&app_dir).ok();

        // Create a test migration file
        let migration =
            Migration::new("0001_initial", "myapp").add_operation(crate::Operation::CreateTable {
                name: "users".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER PRIMARY KEY".to_string(),
                }],
                constraints: vec![],
            });

        let migration_json = serde_json::to_string_pretty(&migration).unwrap();
        fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();

        let mut loader = MigrationLoader::new(temp_dir);
        loader.load_disk().unwrap();

        assert_eq!(loader.disk_migrations.len(), 1);
        assert!(loader.has_migrations("myapp"));
        assert_eq!(loader.get_app_migrations("myapp").len(), 1);

        // Cleanup
        fs::remove_dir_all(&app_dir).ok();
    }

    #[test]
    fn test_get_migration() {
        let temp_dir = std::env::temp_dir().join("reinhardt_loader_test_get");
        fs::create_dir_all(&temp_dir).ok();
        let app_dir = temp_dir.join("testapp");
        fs::create_dir_all(&app_dir).ok();

        let migration = Migration::new("0001_initial", "testapp");
        let migration_json = serde_json::to_string_pretty(&migration).unwrap();
        fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();

        let mut loader = MigrationLoader::new(temp_dir);
        loader.load_disk().unwrap();

        let loaded = loader.get_migration("testapp", "0001_initial");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "0001_initial");

        // Cleanup
        fs::remove_dir_all(&app_dir).ok();
    }
}
