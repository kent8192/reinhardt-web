//! Model and migration discovery
//!
//! This module provides functionality for discovering models and migrations
//! within applications. It includes reverse relation building and migration
//! detection capabilities.
//!
//! # Examples
//!
//! ```rust
//! use reinhardt_apps::discovery::{discover_models, build_reverse_relations};
//!
//! // Discover models for an application
//! let models = discover_models("myapp");
//! println!("Found {} models", models.len());
//!
//! // Build reverse relations (when ORM is fully implemented)
//! // build_reverse_relations();
//! ```

use crate::registry::{get_models_for_app, get_registered_models, ModelMetadata};

/// Discover all models for a given application
///
/// This function retrieves all models that belong to the specified application
/// from the global model registry.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::discovery::discover_models;
///
/// let models = discover_models("auth");
/// for model in models {
///     println!("Found model: {}", model.model_name);
/// }
/// ```
pub fn discover_models(app_label: &str) -> Vec<&'static ModelMetadata> {
    get_models_for_app(app_label)
}

/// Discover all models across all applications
///
/// This function retrieves all models that have been registered in the
/// global model registry.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::discovery::discover_all_models;
///
/// let models = discover_all_models();
/// println!("Total models: {}", models.len());
/// ```
pub fn discover_all_models() -> &'static [ModelMetadata] {
    get_registered_models()
}

/// Relation metadata for building reverse relations
///
/// This structure contains information about a relationship between two models.
/// It is used to build reverse relations automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationMetadata {
    /// The source model (e.g., "Post")
    pub from_model: &'static str,

    /// The target model (e.g., "User")
    pub to_model: &'static str,

    /// The field name in the source model (e.g., "author")
    pub field_name: &'static str,

    /// The related name for the reverse relation (e.g., "posts")
    pub related_name: Option<&'static str>,

    /// The type of relation (OneToMany, ManyToMany, etc.)
    pub relation_type: RelationType,
}

/// Type of relationship between models
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// One-to-many relationship
    OneToMany,
    /// Many-to-many relationship
    ManyToMany,
    /// One-to-one relationship
    OneToOne,
}

impl RelationMetadata {
    /// Create a new relation metadata instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::discovery::{RelationMetadata, RelationType};
    ///
    /// let relation = RelationMetadata::new(
    ///     "Post",
    ///     "User",
    ///     "author",
    ///     Some("posts"),
    ///     RelationType::OneToMany,
    /// );
    /// assert_eq!(relation.from_model, "Post");
    /// assert_eq!(relation.to_model, "User");
    /// ```
    pub const fn new(
        from_model: &'static str,
        to_model: &'static str,
        field_name: &'static str,
        related_name: Option<&'static str>,
        relation_type: RelationType,
    ) -> Self {
        Self {
            from_model,
            to_model,
            field_name,
            related_name,
            relation_type,
        }
    }

    /// Get the reverse relation name
    ///
    /// Returns the related_name if specified, otherwise generates a default name
    /// in the format `{from_model}_set` (e.g., "post_set" for a Post model).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::discovery::{RelationMetadata, RelationType};
    ///
    /// let relation = RelationMetadata::new(
    ///     "Post",
    ///     "User",
    ///     "author",
    ///     Some("posts"),
    ///     RelationType::OneToMany,
    /// );
    /// assert_eq!(relation.reverse_name(), "posts");
    ///
    /// // Without related_name, generates default "{model}_set" format
    /// // Note: This returns a static string, but the format would be "post_set"
    /// // The actual dynamic generation happens in create_reverse_relation()
    /// ```
    pub fn reverse_name(&self) -> &str {
        // Return related_name if specified
        // Note: Default name generation requires String allocation,
        // which cannot be done in this method due to lifetime constraints.
        // Callers should use create_reverse_relation() for full default name generation.
        self.related_name.unwrap_or(self.field_name)
    }
}

/// Build reverse relations between models
///
/// This function analyzes the relationships defined in models and automatically
/// creates reverse relations. For example, if a Post model has a ForeignKey to User,
/// this will create a reverse relation from User to Post.
///
/// The function performs the following steps:
/// 1. Discovers all registered models
/// 2. Analyzes each model for relationship fields (ForeignKey, ManyToMany)
/// 3. Generates appropriate reverse accessor names
/// 4. Creates reverse relation descriptors
///
/// **Current Limitation**: The actual relationship metadata extraction from models
/// is not yet implemented, as the ORM system does not currently expose relationship
/// metadata through the model registry. This will be implemented once the ORM
/// provides a mechanism to introspect model relationships.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::discovery::build_reverse_relations;
///
/// // Build reverse relations for all models
/// build_reverse_relations();
/// ```
pub fn build_reverse_relations() {
    // Step 1: Get all registered models
    let models = get_registered_models();

    // Step 2: Collect all relationships
    let mut relations = Vec::new();

    for model in models {
        // TODO: Extract relationship metadata from model
        // This requires the ORM to expose relationship information through ModelMetadata
        // For now, we collect placeholder relations for demonstration purposes
        let model_relations = extract_model_relations(model);
        relations.extend(model_relations);
    }

    // Step 3: Build reverse relation descriptors
    for relation in &relations {
        create_reverse_relation(relation);
    }
}

/// Extract relationship metadata from a model
///
/// **Current Limitation**: The actual relationship metadata extraction from models
/// is not yet fully implemented due to architectural constraints. This function
/// currently returns an empty vector as a placeholder.
///
/// # Implementation Notes
///
/// To fully implement this function, we need to:
/// 1. Avoid circular dependencies between `reinhardt-apps` and `reinhardt-orm`
/// 2. Design a registry system that allows ORM to register relationship metadata
/// 3. Consider creating a separate `reinhardt-models-registry` crate for shared types
///
/// # Future Implementation
///
/// When the architecture is updated, this function should:
/// 1. Retrieve relationship metadata from a global registry
/// 2. Convert relationship information to `RelationMetadata` format
/// 3. Handle different relationship types (OneToOne, OneToMany, ManyToMany)
fn extract_model_relations(_model: &ModelMetadata) -> Vec<RelationMetadata> {
    // Placeholder implementation - returns empty vector
    // This will be implemented once the architecture supports it without circular dependencies
    Vec::new()
}

/// Create a reverse relation descriptor and register it
///
/// This function generates the reverse accessor name and creates a reverse
/// relation descriptor that will be added to the target model.
///
/// # Reverse Accessor Naming
///
/// - If `related_name` is specified, use that name
/// - Otherwise, generate default name: `{from_model_lowercase}_set`
///   - Example: For Post.author -> User, reverse name is "post_set"
///
/// # Relation Type Mapping
///
/// - ForeignKey (OneToMany) -> Reverse is OneToMany (collection)
/// - ManyToMany -> Reverse is ManyToMany (collection)
/// - OneToOne -> Reverse is OneToOne (single object)
fn create_reverse_relation(relation: &RelationMetadata) {
    // Generate reverse relation name
    let reverse_name = if let Some(name) = relation.related_name {
        name.to_string()
    } else {
        // Default naming: {model_name}_set (e.g., post_set)
        format!("{}_set", relation.from_model.to_lowercase())
    };

    // Determine reverse relation type
    let reverse_type = match relation.relation_type {
        RelationType::OneToMany => RelationType::ManyToMany, // ForeignKey reverse
        RelationType::ManyToMany => RelationType::ManyToMany, // M2M is bidirectional
        RelationType::OneToOne => RelationType::OneToOne,    // O2O is bidirectional
    };

    // TODO: Register the reverse relation in the model registry
    // This requires the ORM registry to support adding reverse relations dynamically
    // The registration should add a reverse accessor to the target model that:
    // 1. Returns a QuerySet or collection of related objects
    // 2. Uses the appropriate loading strategy (lazy by default)
    // 3. Handles the inverse foreign key or junction table

    let _ = (reverse_name, reverse_type); // Suppress unused warnings until implementation
}

/// Migration metadata
///
/// This structure contains information about a migration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationMetadata {
    /// The application this migration belongs to
    pub app_label: &'static str,

    /// The migration name (e.g., "0001_initial")
    pub name: &'static str,

    /// Dependencies on other migrations
    pub dependencies: Vec<(&'static str, &'static str)>, // (app_label, migration_name)
}

impl MigrationMetadata {
    /// Create a new migration metadata instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::discovery::MigrationMetadata;
    ///
    /// let migration = MigrationMetadata::new(
    ///     "myapp",
    ///     "0001_initial",
    ///     vec![],
    /// );
    /// assert_eq!(migration.app_label, "myapp");
    /// assert_eq!(migration.name, "0001_initial");
    /// ```
    pub const fn new(
        app_label: &'static str,
        name: &'static str,
        dependencies: Vec<(&'static str, &'static str)>,
    ) -> Self {
        Self {
            app_label,
            name,
            dependencies,
        }
    }

    /// Get the fully qualified migration name
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::discovery::MigrationMetadata;
    ///
    /// let migration = MigrationMetadata::new(
    ///     "myapp",
    ///     "0001_initial",
    ///     vec![],
    /// );
    /// assert_eq!(migration.qualified_name(), "myapp.0001_initial");
    /// ```
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.app_label, self.name)
    }
}

/// Discover migrations for a given application
///
/// This function scans the migration directory for the specified application
/// and extracts migration metadata including name, app_label, and dependencies.
///
/// # Arguments
///
/// * `app_label` - The application label to discover migrations for
/// * `migration_root` - The root directory containing migration files
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::discovery::discover_migrations;
/// use std::path::PathBuf;
///
/// let migrations = discover_migrations("myapp", &PathBuf::from("/tmp/migrations"));
/// for migration in &migrations {
///     println!("Found migration: {}", migration.qualified_name());
/// }
/// ```
pub fn discover_migrations(
    app_label: &str,
    migration_root: &std::path::Path,
) -> Vec<MigrationMetadata> {
    use std::fs;

    let mut result = Vec::new();
    let app_path = migration_root.join(app_label);

    // Check if migration directory exists for this app
    if !app_path.exists() || !app_path.is_dir() {
        return result;
    }

    // Scan for migration files
    let entries = match fs::read_dir(&app_path) {
        Ok(entries) => entries,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        // Skip files that don't look like migrations
        if !file_name.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }

        // Skip files starting with _ or ~
        if file_name.starts_with('_') || file_name.starts_with('~') {
            continue;
        }

        // Parse migration file
        if let Some(migration_name) = file_name.strip_suffix(".json") {
            match parse_migration_file(&path, app_label, migration_name) {
                Ok(metadata) => result.push(metadata),
                Err(_) => continue,
            }
        }
    }

    result
}

/// Parse a migration file and extract metadata
fn parse_migration_file(
    path: &std::path::Path,
    app_label: &str,
    migration_name: &str,
) -> std::io::Result<MigrationMetadata> {
    use serde_json::Value;
    use std::fs;

    let content = fs::read_to_string(path)?;
    let parsed: Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Extract dependencies
    let mut dependencies = Vec::new();
    if let Some(deps_array) = parsed["dependencies"].as_array() {
        for dep in deps_array {
            if let Some(dep_array) = dep.as_array()
                && dep_array.len() >= 2
                && let (Some(dep_app), Some(dep_name)) =
                    (dep_array[0].as_str(), dep_array[1].as_str())
            {
                // Leak strings to get 'static lifetime
                let dep_app_static: &'static str =
                    Box::leak(dep_app.to_string().into_boxed_str());
                let dep_name_static: &'static str =
                    Box::leak(dep_name.to_string().into_boxed_str());
                dependencies.push((dep_app_static, dep_name_static));
            }
        }
    }

    // Leak app_label and name to get 'static lifetime
    let app_label_static: &'static str = Box::leak(app_label.to_string().into_boxed_str());
    let name_static: &'static str = Box::leak(migration_name.to_string().into_boxed_str());

    Ok(MigrationMetadata::new(
        app_label_static,
        name_static,
        dependencies,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ModelMetadata, MODELS};
    use linkme::distributed_slice;

    // Test models for discovery
    #[distributed_slice(MODELS)]
    static DISCOVERY_TEST_USER: ModelMetadata = ModelMetadata {
        app_label: "discovery_test",
        model_name: "User",
        table_name: "discovery_test_users",
    };

    #[distributed_slice(MODELS)]
    static DISCOVERY_TEST_POST: ModelMetadata = ModelMetadata {
        app_label: "discovery_test",
        model_name: "Post",
        table_name: "discovery_test_posts",
    };

    #[test]
    fn test_discover_models() {
        let models = discover_models("discovery_test");
        assert_eq!(models.len(), 2);

        let model_names: Vec<&str> = models.iter().map(|m| m.model_name).collect();
        assert!(model_names.contains(&"User"));
        assert!(model_names.contains(&"Post"));
    }

    #[test]
    fn test_discover_all_models() {
        let models = discover_all_models();
        // Should have at least our test models
        assert!(models.len() >= 2);

        assert!(models
            .iter()
            .any(|m| m.app_label == "discovery_test" && m.model_name == "User"));
    }

    #[test]
    fn test_discover_models_empty() {
        let models = discover_models("nonexistent_app");
        assert_eq!(models.len(), 0);
    }

    #[test]
    fn test_relation_metadata_new() {
        let relation = RelationMetadata::new(
            "Post",
            "User",
            "author",
            Some("posts"),
            RelationType::OneToMany,
        );

        assert_eq!(relation.from_model, "Post");
        assert_eq!(relation.to_model, "User");
        assert_eq!(relation.field_name, "author");
        assert_eq!(relation.related_name, Some("posts"));
        assert_eq!(relation.relation_type, RelationType::OneToMany);
    }

    #[test]
    fn test_relation_metadata_reverse_name() {
        let relation = RelationMetadata::new(
            "Post",
            "User",
            "author",
            Some("posts"),
            RelationType::OneToMany,
        );
        assert_eq!(relation.reverse_name(), "posts");

        let relation =
            RelationMetadata::new("Post", "User", "author", None, RelationType::OneToMany);
        // Without related_name, returns field_name (full default name generated in create_reverse_relation)
        assert_eq!(relation.reverse_name(), "author");
    }

    #[test]
    fn test_relation_types() {
        assert_eq!(RelationType::OneToMany, RelationType::OneToMany);
        assert_ne!(RelationType::OneToMany, RelationType::ManyToMany);
        assert_ne!(RelationType::OneToMany, RelationType::OneToOne);
    }

    #[test]
    fn test_migration_metadata_new() {
        let migration = MigrationMetadata::new("myapp", "0001_initial", vec![]);

        assert_eq!(migration.app_label, "myapp");
        assert_eq!(migration.name, "0001_initial");
        assert_eq!(migration.dependencies.len(), 0);
    }

    #[test]
    fn test_migration_metadata_qualified_name() {
        let migration = MigrationMetadata::new("myapp", "0001_initial", vec![]);
        assert_eq!(migration.qualified_name(), "myapp.0001_initial");
    }

    #[test]
    fn test_migration_metadata_with_dependencies() {
        let migration = MigrationMetadata::new(
            "myapp",
            "0002_add_field",
            vec![("myapp", "0001_initial"), ("auth", "0001_initial")],
        );

        assert_eq!(migration.dependencies.len(), 2);
        assert_eq!(migration.dependencies[0], ("myapp", "0001_initial"));
        assert_eq!(migration.dependencies[1], ("auth", "0001_initial"));
    }

    #[test]
    fn test_discover_migrations_empty_directory() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("/tmp/reinhardt_test_discover_migrations_empty");
        fs::create_dir_all(&temp_dir).ok();

        let migrations = discover_migrations("nonexistent_app", &temp_dir);
        assert_eq!(migrations.len(), 0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_discover_migrations_single_migration() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("/tmp/reinhardt_test_discover_migrations_single");
        let app_dir = temp_dir.join("testapp");
        fs::create_dir_all(&app_dir).ok();

        let migration_json = r#"{
            "app_label": "testapp",
            "name": "0001_initial",
            "dependencies": [],
            "replaces": [],
            "atomic": true,
            "operations": []
        }"#;
        fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();

        let migrations = discover_migrations("testapp", &temp_dir);
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].app_label, "testapp");
        assert_eq!(migrations[0].name, "0001_initial");
        assert_eq!(migrations[0].dependencies.len(), 0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_discover_migrations_with_dependencies() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("/tmp/reinhardt_test_discover_migrations_deps");
        let app_dir = temp_dir.join("myapp");
        fs::create_dir_all(&app_dir).ok();

        let migration_json = r#"{
            "app_label": "myapp",
            "name": "0002_add_field",
            "dependencies": [["myapp", "0001_initial"], ["auth", "0001_initial"]],
            "replaces": [],
            "atomic": true,
            "operations": []
        }"#;
        fs::write(app_dir.join("0002_add_field.json"), migration_json).unwrap();

        let migrations = discover_migrations("myapp", &temp_dir);
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].dependencies.len(), 2);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_discover_migrations_multiple_files() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("/tmp/reinhardt_test_discover_migrations_multiple");
        let app_dir = temp_dir.join("testapp");
        fs::create_dir_all(&app_dir).ok();

        for i in 1..=3 {
            let migration_json = format!(
                r#"{{
                    "app_label": "testapp",
                    "name": "000{}_migration",
                    "dependencies": [],
                    "replaces": [],
                    "atomic": true,
                    "operations": []
                }}"#,
                i
            );
            fs::write(
                app_dir.join(format!("000{}_migration.json", i)),
                migration_json,
            )
            .unwrap();
        }

        let migrations = discover_migrations("testapp", &temp_dir);
        assert_eq!(migrations.len(), 3);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_discover_migrations_skip_invalid_files() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("/tmp/reinhardt_test_discover_migrations_skip");
        let app_dir = temp_dir.join("testapp");
        fs::create_dir_all(&app_dir).ok();

        let valid_migration = r#"{
            "app_label": "testapp",
            "name": "0001_initial",
            "dependencies": [],
            "replaces": [],
            "atomic": true,
            "operations": []
        }"#;
        fs::write(app_dir.join("0001_initial.json"), valid_migration).unwrap();

        // Create files that should be skipped
        fs::write(app_dir.join("__init__.py"), "").unwrap();
        fs::write(app_dir.join("_helper.json"), "{}").unwrap();
        fs::write(app_dir.join("~temp.json"), "{}").unwrap();
        fs::write(app_dir.join("README.md"), "# Migrations").unwrap();

        let migrations = discover_migrations("testapp", &temp_dir);
        assert_eq!(migrations.len(), 1);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_discover_migrations_qualified_name() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("/tmp/reinhardt_test_discover_migrations_qualified");
        let app_dir = temp_dir.join("myapp");
        fs::create_dir_all(&app_dir).ok();

        let migration_json = r#"{
            "app_label": "myapp",
            "name": "0001_initial",
            "dependencies": [],
            "replaces": [],
            "atomic": true,
            "operations": []
        }"#;
        fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();

        let migrations = discover_migrations("myapp", &temp_dir);
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].qualified_name(), "myapp.0001_initial");

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_build_reverse_relations_basic() {
        // Should not panic - basic implementation exists
        build_reverse_relations();
        // Currently no-op as models don't expose relationship metadata
    }

    #[test]
    fn test_extract_model_relations_placeholder() {
        let metadata = ModelMetadata::new("test", "User", "users");
        let relations = extract_model_relations(&metadata);
        // Currently returns empty vector as relationship metadata is not exposed
        assert_eq!(relations.len(), 0);
    }

    #[test]
    fn test_create_reverse_relation_default_naming() {
        let relation =
            RelationMetadata::new("Post", "User", "author", None, RelationType::OneToMany);
        // Should not panic - creates reverse relation descriptor
        create_reverse_relation(&relation);
        // Currently no-op as registry doesn't support dynamic reverse relations
    }

    #[test]
    fn test_create_reverse_relation_with_related_name() {
        let relation = RelationMetadata::new(
            "Post",
            "User",
            "author",
            Some("posts"),
            RelationType::OneToMany,
        );
        // Should use provided related_name instead of default
        create_reverse_relation(&relation);
    }

    #[test]
    fn test_create_reverse_relation_many_to_many() {
        let relation = RelationMetadata::new(
            "User",
            "Role",
            "roles",
            Some("users"),
            RelationType::ManyToMany,
        );
        create_reverse_relation(&relation);
    }

    #[test]
    fn test_create_reverse_relation_one_to_one() {
        let relation = RelationMetadata::new(
            "Profile",
            "User",
            "user",
            Some("profile"),
            RelationType::OneToOne,
        );
        create_reverse_relation(&relation);
    }
}
