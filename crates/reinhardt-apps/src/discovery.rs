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
    /// Returns the related_name if specified, otherwise generates a default name.
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
    /// let relation = RelationMetadata::new(
    ///     "Post",
    ///     "User",
    ///     "author",
    ///     None,
    ///     RelationType::OneToMany,
    /// );
    /// // Default would be "{from_model}_set" but this is just a placeholder
    /// // In real implementation, this would be "post_set"
    /// ```
    pub fn reverse_name(&self) -> &str {
        self.related_name.unwrap_or_else(|| {
            // In real implementation, this would generate a default name
            // like "{from_model}_set" (e.g., "post_set")
            self.field_name
        })
    }
}

/// Build reverse relations between models
///
/// This function analyzes the relationships defined in models and automatically
/// creates reverse relations. For example, if a Post model has a ForeignKey to User,
/// this will create a reverse relation from User to Post.
///
/// **Note**: This is a placeholder implementation. Full reverse relation building
/// requires integration with the ORM's relationship system, which is not yet
/// fully implemented.
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
    todo!(
        "Implement reverse relation building - requires ORM relationship system. \
        This requires: \
        1. Analyzing model metadata to find ForeignKey/ManyToMany fields, \
        2. Creating reverse relation descriptors, \
        3. Registering reverse relations in the model registry"
    )
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
/// **Note**: This is a placeholder implementation. Full migration discovery
/// requires integration with the `reinhardt-migrations` crate, which is not yet
/// implemented.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::discovery::discover_migrations;
///
/// let migrations = discover_migrations("myapp");
/// // Note: This function will panic with todo!() as migration system is not yet implemented
/// ```
pub fn discover_migrations(_app_label: &str) -> Vec<MigrationMetadata> {
    todo!(
        "Implement migration discovery - requires reinhardt-migrations integration. \
        This requires: \
        1. Integration with reinhardt-migrations crate, \
        2. Scanning for migration files or registered migrations, \
        3. Parsing migration dependencies"
    )
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
        // Without related_name, defaults to field_name (placeholder behavior)
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
    #[should_panic(expected = "not yet implemented")]
    fn test_discover_migrations_not_implemented() {
        discover_migrations("myapp");
    }

    #[test]
    #[should_panic(expected = "not yet implemented")]
    fn test_build_reverse_relations_not_implemented() {
        build_reverse_relations();
    }
}
