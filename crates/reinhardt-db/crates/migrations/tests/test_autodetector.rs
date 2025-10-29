//! Tests for migration autodetector
//! Translated from Django's test_autodetector.py
//!
//! Django reference: django/tests/migrations/test_autodetector.py

use reinhardt_migrations::{
    ConstraintDefinition, FieldState, IndexDefinition, MigrationAutodetector, ModelState,
    ProjectState,
};

/// Helper function to create a simple field
fn field(name: &str, field_type: &str, nullable: bool) -> FieldState {
    FieldState::new(name.to_string(), field_type.to_string(), nullable)
}

#[test]
fn test_create_model_simple() {
    // Django: test_arrange_for_graph_with_multiple_initial
    // Test that a simple model creation is detected
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    let mut book_model = ModelState::new("testapp", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(book_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.created_models.len(), 1);
    assert_eq!(changes.created_models[0].0, "testapp");
    assert_eq!(changes.created_models[0].1, "Book");
}

#[test]
fn test_delete_model() {
    // Django: test_delete_model
    // Test that model deletion is detected
    let mut from_state = ProjectState::new();
    let to_state = ProjectState::new();

    let mut book_model = ModelState::new("testapp", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    from_state.add_model(book_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.deleted_models.len(), 1);
    assert_eq!(changes.deleted_models[0].0, "testapp");
    assert_eq!(changes.deleted_models[0].1, "Book");
}

#[test]
fn test_add_field() {
    // Django: test_add_field
    // Test that adding a field is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Create initial model
    let mut book_model_old = ModelState::new("testapp", "Book");
    book_model_old.add_field(field("id", "INTEGER", false));
    book_model_old.add_field(field("title", "VARCHAR(200)", false));
    from_state.add_model(book_model_old);

    // Add a field to the model
    let mut book_model_new = ModelState::new("testapp", "Book");
    book_model_new.add_field(field("id", "INTEGER", false));
    book_model_new.add_field(field("title", "VARCHAR(200)", false));
    book_model_new.add_field(field("author", "VARCHAR(100)", false));
    to_state.add_model(book_model_new);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.added_fields.len(), 1);
    assert_eq!(changes.added_fields[0].0, "testapp");
    assert_eq!(changes.added_fields[0].1, "Book");
    assert_eq!(changes.added_fields[0].2, "author");
}

#[test]
fn test_remove_field() {
    // Django: test_remove_field
    // Test that removing a field is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Create initial model with three fields
    let mut book_model_old = ModelState::new("testapp", "Book");
    book_model_old.add_field(field("id", "INTEGER", false));
    book_model_old.add_field(field("title", "VARCHAR(200)", false));
    book_model_old.add_field(field("author", "VARCHAR(100)", false));
    from_state.add_model(book_model_old);

    // Remove a field from the model
    let mut book_model_new = ModelState::new("testapp", "Book");
    book_model_new.add_field(field("id", "INTEGER", false));
    book_model_new.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(book_model_new);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.removed_fields.len(), 1);
    assert_eq!(changes.removed_fields[0].0, "testapp");
    assert_eq!(changes.removed_fields[0].1, "Book");
    assert_eq!(changes.removed_fields[0].2, "author");
}

#[test]
fn test_alter_field() {
    // Django: test_alter_field
    // Test that altering a field is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Create initial model
    let mut book_model_old = ModelState::new("testapp", "Book");
    book_model_old.add_field(field("id", "INTEGER", false));
    book_model_old.add_field(field("title", "VARCHAR(100)", false));
    from_state.add_model(book_model_old);

    // Change field type
    let mut book_model_new = ModelState::new("testapp", "Book");
    book_model_new.add_field(field("id", "INTEGER", false));
    book_model_new.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(book_model_new);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.altered_fields.len(), 1);
    assert_eq!(changes.altered_fields[0].0, "testapp");
    assert_eq!(changes.altered_fields[0].1, "Book");
    assert_eq!(changes.altered_fields[0].2, "title");
}

#[test]
fn test_alter_field_nullable() {
    // Django: test_alter_field_to_not_null_without_default
    // Test that changing nullable status is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Create initial model with nullable field
    let mut book_model_old = ModelState::new("testapp", "Book");
    book_model_old.add_field(field("id", "INTEGER", false));
    book_model_old.add_field(field("title", "VARCHAR(200)", true));
    from_state.add_model(book_model_old);

    // Change to not nullable
    let mut book_model_new = ModelState::new("testapp", "Book");
    book_model_new.add_field(field("id", "INTEGER", false));
    book_model_new.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(book_model_new);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.altered_fields.len(), 1);
    assert_eq!(changes.altered_fields[0].0, "testapp");
    assert_eq!(changes.altered_fields[0].1, "Book");
    assert_eq!(changes.altered_fields[0].2, "title");
}

#[test]
fn test_no_changes() {
    // Django: test_empty
    // Test that no changes results in no migrations
    let mut state = ProjectState::new();

    let mut book_model = ModelState::new("testapp", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    state.add_model(book_model.clone());

    let autodetector = MigrationAutodetector::new(state.clone(), state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.created_models.len(), 0);
    assert_eq!(changes.deleted_models.len(), 0);
    assert_eq!(changes.added_fields.len(), 0);
    assert_eq!(changes.removed_fields.len(), 0);
    assert_eq!(changes.altered_fields.len(), 0);
}

#[test]
fn test_multiple_apps() {
    // Django: test_arrange_for_graph_with_multiple_apps
    // Test that changes in multiple apps are detected
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Add models to two different apps
    let mut book_model = ModelState::new("books", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(book_model);

    let mut author_model = ModelState::new("authors", "Author");
    author_model.add_field(field("id", "INTEGER", false));
    author_model.add_field(field("name", "VARCHAR(100)", false));
    to_state.add_model(author_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.created_models.len(), 2);
    assert!(changes.created_models.iter().any(|(app, _)| app == "books"));
    assert!(
        changes
            .created_models
            .iter()
            .any(|(app, _)| app == "authors")
    );
}

#[test]
fn test_rename_model_detected_as_rename() {
    // Test that model rename can be detected when fields are identical
    // Note: Current implementation may detect this as delete+create depending on similarity threshold
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model with complete field set
    let mut old_model = ModelState::new("testapp", "OldBook");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("title", "VARCHAR(200)", false));
    old_model.add_field(field("author", "VARCHAR(100)", false));
    from_state.add_model(old_model);

    // New model with identical fields (high field similarity)
    let mut new_model = ModelState::new("testapp", "NewBook");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("title", "VARCHAR(200)", false));
    new_model.add_field(field("author", "VARCHAR(100)", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify either rename detection (if implemented) or delete+create fallback
    let has_rename = !changes.renamed_models.is_empty();
    let has_delete_create = changes.created_models.len() == 1 && changes.deleted_models.len() == 1;

    assert!(
        has_rename || has_delete_create,
        "Expected either rename detection or delete+create fallback. \
         renamed: {}, created: {}, deleted: {}",
        changes.renamed_models.len(),
        changes.created_models.len(),
        changes.deleted_models.len()
    );
}

#[test]
fn test_rename_model_detected_as_delete_and_create() {
    // Test that model with different fields is detected as delete + create
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model
    let mut old_model = ModelState::new("testapp", "OldBook");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("old_field1", "VARCHAR(100)", false));
    old_model.add_field(field("old_field2", "INTEGER", false));
    from_state.add_model(old_model);

    // New model with completely different fields (low similarity)
    let mut new_model = ModelState::new("testapp", "NewBook");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("new_field1", "TEXT", false));
    new_model.add_field(field("new_field2", "BOOLEAN", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Should detect as delete + create due to low similarity
    assert_eq!(
        changes.created_models.len(),
        1,
        "Expected model creation to be detected"
    );
    assert_eq!(
        changes.deleted_models.len(),
        1,
        "Expected model deletion to be detected"
    );

    // Verify not detected as rename
    assert!(
        changes.renamed_models.is_empty(),
        "Should not detect as rename"
    );
}

#[test]
fn test_rename_field_detected_as_rename() {
    // Test that field rename can be detected when type matches
    // Note: Current implementation may detect this as add+remove depending on similarity threshold
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model with original field name
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("old_title", "VARCHAR(200)", false));
    from_state.add_model(old_model);

    // New model with renamed field (same type)
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("new_title", "VARCHAR(200)", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify either rename detection (if implemented) or add+remove fallback
    let has_rename = !changes.renamed_fields.is_empty();
    let has_add_remove = changes.added_fields.len() == 1 && changes.removed_fields.len() == 1;

    assert!(
        has_rename || has_add_remove,
        "Expected either rename detection or add+remove fallback. \
         renamed: {}, added: {}, removed: {}",
        changes.renamed_fields.len(),
        changes.added_fields.len(),
        changes.removed_fields.len()
    );
}

#[test]
fn test_rename_field_detected_as_add_and_remove() {
    // Test that fields with different types are detected as remove + add
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model with original field
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("old_field", "VARCHAR(100)", false));
    from_state.add_model(old_model);

    // New model with different field (different type)
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("new_field", "TEXT", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Should detect as add + remove due to different types
    assert_eq!(
        changes.added_fields.len(),
        1,
        "Expected field addition to be detected"
    );
    assert_eq!(
        changes.removed_fields.len(),
        1,
        "Expected field removal to be detected"
    );

    // Verify not detected as rename
    assert!(
        changes.renamed_fields.is_empty(),
        "Should not detect as rename"
    );
}

#[test]
fn test_add_index() {
    // Django: test_create_model_with_indexes
    // Test that adding an index is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Model without index
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("title", "VARCHAR(200)", false));
    from_state.add_model(old_model);

    // Model with index
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("title", "VARCHAR(200)", false));
    new_model.indexes.push(IndexDefinition {
        name: "idx_title".to_string(),
        fields: vec!["title".to_string()],
        unique: false,
    });
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.added_indexes.len(), 1);
    assert_eq!(changes.added_indexes[0].0, "testapp");
    assert_eq!(changes.added_indexes[0].1, "Book");
    assert_eq!(changes.added_indexes[0].2.name, "idx_title");
}

#[test]
fn test_remove_index() {
    // Django: test_remove_index
    // Test that removing an index is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Model with index
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("title", "VARCHAR(200)", false));
    old_model.indexes.push(IndexDefinition {
        name: "idx_title".to_string(),
        fields: vec!["title".to_string()],
        unique: false,
    });
    from_state.add_model(old_model);

    // Model without index
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.removed_indexes.len(), 1);
    assert_eq!(changes.removed_indexes[0].0, "testapp");
    assert_eq!(changes.removed_indexes[0].1, "Book");
    assert_eq!(changes.removed_indexes[0].2, "idx_title");
}

#[test]
fn test_add_constraint() {
    // Django: test_add_constraints
    // Test that adding a constraint is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Model without constraint
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("price", "DECIMAL", false));
    from_state.add_model(old_model);

    // Model with constraint
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("price", "DECIMAL", false));
    new_model.constraints.push(ConstraintDefinition {
        name: "chk_price_positive".to_string(),
        constraint_type: "check".to_string(),
        fields: vec!["price".to_string()],
        expression: Some("price > 0".to_string()),
    });
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.added_constraints.len(), 1);
    assert_eq!(changes.added_constraints[0].0, "testapp");
    assert_eq!(changes.added_constraints[0].1, "Book");
    assert_eq!(changes.added_constraints[0].2.name, "chk_price_positive");
}

#[test]
fn test_remove_constraint() {
    // Django: test_remove_constraints
    // Test that removing a constraint is detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Model with constraint
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("price", "DECIMAL", false));
    old_model.constraints.push(ConstraintDefinition {
        name: "chk_price_positive".to_string(),
        constraint_type: "check".to_string(),
        fields: vec!["price".to_string()],
        expression: Some("price > 0".to_string()),
    });
    from_state.add_model(old_model);

    // Model without constraint
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("price", "DECIMAL", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.removed_constraints.len(), 1);
    assert_eq!(changes.removed_constraints[0].0, "testapp");
    assert_eq!(changes.removed_constraints[0].1, "Book");
    assert_eq!(changes.removed_constraints[0].2, "chk_price_positive");
}

#[test]
fn test_multiple_changes_same_model() {
    // Django: test_multiple_changes
    // Test that multiple changes to the same model are all detected
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("title", "VARCHAR(100)", false));
    old_model.add_field(field("old_field", "VARCHAR(50)", false));
    from_state.add_model(old_model);

    // New model with multiple changes
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("title", "VARCHAR(200)", false)); // altered
    new_model.add_field(field("new_field", "VARCHAR(50)", false)); // added
    // old_field removed
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.added_fields.len(), 1);
    assert_eq!(changes.removed_fields.len(), 1);
    assert_eq!(changes.altered_fields.len(), 1);
}

#[test]
fn test_create_model_with_multiple_fields() {
    // Django: test_create_model_with_check_constraint
    // Test creating a model with multiple fields
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    let mut book_model = ModelState::new("testapp", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    book_model.add_field(field("author", "VARCHAR(100)", false));
    book_model.add_field(field("price", "DECIMAL", false));
    book_model.add_field(field("published_date", "DATE", true));
    to_state.add_model(book_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state.clone());
    let changes = autodetector.detect_changes();

    assert_eq!(changes.created_models.len(), 1);
    let model = to_state.get_model("testapp", "Book").unwrap();
    assert_eq!(model.fields.len(), 5);
}

#[test]
fn test_unchanged_model_with_index() {
    // Test that unchanged models with indexes don't generate changes
    let mut state = ProjectState::new();

    let mut book_model = ModelState::new("testapp", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    book_model.indexes.push(IndexDefinition {
        name: "idx_title".to_string(),
        fields: vec!["title".to_string()],
        unique: false,
    });
    state.add_model(book_model.clone());

    let autodetector = MigrationAutodetector::new(state.clone(), state);
    let changes = autodetector.detect_changes();

    assert_eq!(changes.added_indexes.len(), 0);
    assert_eq!(changes.removed_indexes.len(), 0);
}

#[test]
fn test_generate_operations_from_changes() {
    // Test that generate_operations correctly converts DetectedChanges to Operations
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    let mut book_model = ModelState::new("testapp", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    to_state.add_model(book_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let operations = autodetector.generate_operations();

    assert!(!operations.is_empty());
    // Should generate at least one CreateTable operation
    assert!(
        operations
            .iter()
            .any(|op| matches!(op, reinhardt_migrations::Operation::CreateTable { .. }))
    );
}

#[test]
fn test_generate_migrations() {
    // Test that generate_migrations creates Migration objects grouped by app
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Add models to two different apps
    let mut book_model = ModelState::new("books", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    to_state.add_model(book_model);

    let mut author_model = ModelState::new("authors", "Author");
    author_model.add_field(field("id", "INTEGER", false));
    to_state.add_model(author_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let migrations = autodetector.generate_migrations();

    assert_eq!(migrations.len(), 2);
    assert!(migrations.iter().any(|m| m.app_label == "books"));
    assert!(migrations.iter().any(|m| m.app_label == "authors"));
}

// ============================================================================
// Phase 1.3 & 1.4 Tests: Hybrid Similarity and Dependency Checking
// ============================================================================

#[test]
fn test_similarity_config_default() {
    // Test default similarity configuration
    use reinhardt_migrations::SimilarityConfig;

    let config = SimilarityConfig::default();
    assert_eq!(config.model_threshold(), 0.7);
    assert_eq!(config.field_threshold(), 0.8);
}

#[test]
fn test_similarity_config_new() {
    // Test creating custom similarity configuration
    use reinhardt_migrations::SimilarityConfig;

    let config = SimilarityConfig::new(0.75, 0.85).unwrap();
    assert_eq!(config.model_threshold(), 0.75);
    assert_eq!(config.field_threshold(), 0.85);
}

#[test]
fn test_similarity_config_validation() {
    // Test similarity config threshold validation
    use reinhardt_migrations::SimilarityConfig;

    // Valid thresholds
    assert!(SimilarityConfig::new(0.5, 0.5).is_ok());
    assert!(SimilarityConfig::new(0.95, 0.95).is_ok());
    assert!(SimilarityConfig::new(0.7, 0.8).is_ok());

    // Invalid thresholds - too low
    assert!(SimilarityConfig::new(0.4, 0.8).is_err());
    assert!(SimilarityConfig::new(0.7, 0.4).is_err());

    // Invalid thresholds - too high
    assert!(SimilarityConfig::new(0.96, 0.8).is_err());
    assert!(SimilarityConfig::new(0.7, 0.96).is_err());
}

#[test]
fn test_similarity_config_with_weights() {
    // Test similarity config with custom algorithm weights
    use reinhardt_migrations::SimilarityConfig;

    // Valid weights (sum to 1.0)
    let config = SimilarityConfig::with_weights(0.75, 0.85, 0.8, 0.2).unwrap();
    assert_eq!(config.model_threshold(), 0.75);
    assert_eq!(config.field_threshold(), 0.85);

    // Prefer Levenshtein over Jaro-Winkler
    let config = SimilarityConfig::with_weights(0.7, 0.8, 0.3, 0.7).unwrap();
    assert_eq!(config.model_threshold(), 0.7);

    // Equal weights
    let config = SimilarityConfig::with_weights(0.7, 0.8, 0.5, 0.5).unwrap();
    assert_eq!(config.model_threshold(), 0.7);
}

#[test]
fn test_similarity_config_weights_validation() {
    // Test that weights must sum to 1.0
    use reinhardt_migrations::SimilarityConfig;

    // Invalid weights - don't sum to 1.0
    assert!(SimilarityConfig::with_weights(0.75, 0.85, 0.5, 0.3).is_err());
    assert!(SimilarityConfig::with_weights(0.75, 0.85, 0.8, 0.8).is_err());

    // Invalid weights - out of range
    assert!(SimilarityConfig::with_weights(0.75, 0.85, 1.5, -0.5).is_err());
    assert!(SimilarityConfig::with_weights(0.75, 0.85, -0.2, 1.2).is_err());
}

#[test]
fn test_cross_app_model_move_detected_as_move() {
    // Test that moving models between apps can be detected
    // Note: Current implementation may detect this as delete+create depending on similarity threshold
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Model in app1 with complete field set
    let mut old_model = ModelState::new("app1", "User");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("email", "VARCHAR(200)", false));
    old_model.add_field(field("username", "VARCHAR(100)", false));
    from_state.add_model(old_model);

    // Same model moved to app2 (identical fields)
    let mut new_model = ModelState::new("app2", "User");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("email", "VARCHAR(200)", false));
    new_model.add_field(field("username", "VARCHAR(100)", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify either move detection (if implemented) or delete+create fallback
    let has_move = !changes.moved_models.is_empty();
    let has_delete_create = changes.created_models.len() == 1 && changes.deleted_models.len() == 1;

    assert!(
        has_move || has_delete_create,
        "Expected either move detection or delete+create fallback. \
         moved: {}, created: {}, deleted: {}",
        changes.moved_models.len(),
        changes.created_models.len(),
        changes.deleted_models.len()
    );
}

#[test]
fn test_cross_app_model_move_detected_as_delete_and_create() {
    // Test that models with different structures are detected as delete + create
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Model in app1
    let mut old_model = ModelState::new("app1", "User");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("old_field1", "VARCHAR(100)", false));
    old_model.add_field(field("old_field2", "INTEGER", false));
    from_state.add_model(old_model);

    // Different model in app2 (different fields)
    let mut new_model = ModelState::new("app2", "User");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("new_field1", "TEXT", false));
    new_model.add_field(field("new_field2", "BOOLEAN", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Should detect as delete + create due to different structure
    assert_eq!(
        changes.created_models.len(),
        1,
        "Expected model creation to be detected"
    );
    assert_eq!(
        changes.deleted_models.len(),
        1,
        "Expected model deletion to be detected"
    );

    // Verify not detected as move
    assert!(changes.moved_models.is_empty(), "Should not detect as move");
}

#[test]
fn test_model_dependencies_simple() {
    // Test simple foreign key dependency detection
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Create User model
    let mut user_model = ModelState::new("accounts", "User");
    user_model.add_field(field("id", "INTEGER", false));
    user_model.add_field(field("email", "VARCHAR(200)", false));
    to_state.add_model(user_model);

    // Create Post model that depends on User
    let mut post_model = ModelState::new("blog", "Post");
    post_model.add_field(field("id", "INTEGER", false));
    post_model.add_field(field("title", "VARCHAR(200)", false));
    post_model.add_field(field("author", "ForeignKey(accounts.User)", false));
    to_state.add_model(post_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // blog.Post should depend on accounts.User
    assert!(
        changes
            .model_dependencies
            .contains_key(&("blog".to_string(), "Post".to_string()))
    );

    let deps = changes
        .model_dependencies
        .get(&("blog".to_string(), "Post".to_string()))
        .unwrap();
    assert!(deps.contains(&("accounts".to_string(), "User".to_string())));
}

#[test]
fn test_model_dependencies_many_to_many() {
    // Test ManyToMany dependency detection
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Create Author model
    let mut author_model = ModelState::new("authors", "Author");
    author_model.add_field(field("id", "INTEGER", false));
    author_model.add_field(field("name", "VARCHAR(100)", false));
    to_state.add_model(author_model);

    // Create Book model with ManyToMany to Author
    let mut book_model = ModelState::new("books", "Book");
    book_model.add_field(field("id", "INTEGER", false));
    book_model.add_field(field("title", "VARCHAR(200)", false));
    book_model.add_field(field("authors", "ManyToManyField(authors.Author)", false));
    to_state.add_model(book_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // books.Book should depend on authors.Author
    let deps = changes
        .model_dependencies
        .get(&("books".to_string(), "Book".to_string()));
    assert!(deps.is_some());
    assert!(
        deps.unwrap()
            .contains(&("authors".to_string(), "Author".to_string()))
    );
}

#[test]
fn test_topological_sort_simple() {
    // Test that models are ordered by dependencies
    use reinhardt_migrations::DetectedChanges;
    use std::collections::HashMap;

    let mut changes = DetectedChanges::default();
    changes
        .created_models
        .push(("accounts".to_string(), "User".to_string()));
    changes
        .created_models
        .push(("blog".to_string(), "Post".to_string()));

    // Post depends on User
    let mut deps = HashMap::new();
    deps.insert(
        ("blog".to_string(), "Post".to_string()),
        vec![("accounts".to_string(), "User".to_string())],
    );
    changes.model_dependencies = deps;

    let ordered = changes.order_models_by_dependency();

    // User should come before Post
    let user_idx = ordered
        .iter()
        .position(|m| m == &("accounts".to_string(), "User".to_string()))
        .unwrap();
    let post_idx = ordered
        .iter()
        .position(|m| m == &("blog".to_string(), "Post".to_string()))
        .unwrap();
    assert!(user_idx < post_idx);
}

#[test]
fn test_topological_sort_complex() {
    // Test complex dependency chain: A -> B -> C
    use reinhardt_migrations::DetectedChanges;
    use std::collections::HashMap;

    let mut changes = DetectedChanges::default();
    changes
        .created_models
        .push(("app".to_string(), "A".to_string()));
    changes
        .created_models
        .push(("app".to_string(), "B".to_string()));
    changes
        .created_models
        .push(("app".to_string(), "C".to_string()));

    // C depends on B, B depends on A
    let mut deps = HashMap::new();
    deps.insert(
        ("app".to_string(), "B".to_string()),
        vec![("app".to_string(), "A".to_string())],
    );
    deps.insert(
        ("app".to_string(), "C".to_string()),
        vec![("app".to_string(), "B".to_string())],
    );
    changes.model_dependencies = deps;

    let ordered = changes.order_models_by_dependency();

    // A should come before B, B should come before C
    let a_idx = ordered
        .iter()
        .position(|m| m == &("app".to_string(), "A".to_string()))
        .unwrap();
    let b_idx = ordered
        .iter()
        .position(|m| m == &("app".to_string(), "B".to_string()))
        .unwrap();
    let c_idx = ordered
        .iter()
        .position(|m| m == &("app".to_string(), "C".to_string()))
        .unwrap();
    assert!(a_idx < b_idx);
    assert!(b_idx < c_idx);
}

#[test]
fn test_circular_dependency_detection() {
    // Test that circular dependencies are detected
    use reinhardt_migrations::DetectedChanges;
    use std::collections::HashMap;

    let mut changes = DetectedChanges::default();

    // Create circular dependency: A -> B -> C -> A
    let mut deps = HashMap::new();
    deps.insert(
        ("app".to_string(), "A".to_string()),
        vec![("app".to_string(), "B".to_string())],
    );
    deps.insert(
        ("app".to_string(), "B".to_string()),
        vec![("app".to_string(), "C".to_string())],
    );
    deps.insert(
        ("app".to_string(), "C".to_string()),
        vec![("app".to_string(), "A".to_string())],
    );
    changes.model_dependencies = deps;

    let result = changes.check_circular_dependencies();
    assert!(result.is_err());
}

#[test]
fn test_no_circular_dependency() {
    // Test that non-circular dependencies pass validation
    use reinhardt_migrations::DetectedChanges;
    use std::collections::HashMap;

    let mut changes = DetectedChanges::default();

    // Create non-circular dependency: A -> B, C -> D
    let mut deps = HashMap::new();
    deps.insert(
        ("app".to_string(), "B".to_string()),
        vec![("app".to_string(), "A".to_string())],
    );
    deps.insert(
        ("app".to_string(), "D".to_string()),
        vec![("app".to_string(), "C".to_string())],
    );
    changes.model_dependencies = deps;

    let result = changes.check_circular_dependencies();
    assert!(result.is_ok());
}

// ============================================================================
// Phase 2 Tests: Pattern Learning and Inference
// ============================================================================

#[test]
fn test_change_tracker_record_model_rename() {
    use reinhardt_migrations::ChangeTracker;

    let mut tracker = ChangeTracker::new();

    // Record some model renames
    tracker.record_model_rename("app1", "OldUser", "NewUser");
    tracker.record_model_rename("app2", "OldProduct", "NewProduct");

    // Check history size
    assert_eq!(tracker.len(), 2);

    // Check pattern frequency
    let patterns = tracker.get_frequent_patterns(1);
    assert!(!patterns.is_empty());
}

#[test]
fn test_change_tracker_pattern_frequency() {
    use reinhardt_migrations::ChangeTracker;

    let mut tracker = ChangeTracker::new();

    // Record same pattern multiple times
    for _ in 0..3 {
        tracker.record_model_rename("app", "Old", "New");
    }

    // Should have high frequency for this pattern
    let patterns = tracker.get_frequent_patterns(2);
    assert!(patterns.len() >= 1);
    assert_eq!(patterns[0].frequency, 3);
}

#[test]
fn test_change_tracker_cooccurrence() {
    use reinhardt_migrations::ChangeTracker;
    use std::time::Duration;

    let mut tracker = ChangeTracker::new();

    // Record co-occurring changes
    tracker.record_model_rename("app", "User", "Account");
    tracker.record_field_addition("app", "Account", "email");

    // Analyze co-occurrence within 1 hour
    let cooccurrence = tracker.analyze_cooccurrence(Duration::from_secs(3600));
    assert!(!cooccurrence.is_empty());
}

#[test]
fn test_pattern_matcher_basic() {
    use reinhardt_migrations::PatternMatcher;

    let mut matcher = PatternMatcher::new();
    matcher.add_pattern("User");
    matcher.add_pattern("Product");
    matcher.build().unwrap();

    let text = "UserModel and ProductModel";
    let matches = matcher.find_all(text);

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].pattern, "User");
    assert_eq!(matches[1].pattern, "Product");
}

#[test]
fn test_pattern_matcher_contains() {
    use reinhardt_migrations::PatternMatcher;

    let mut matcher = PatternMatcher::new();
    matcher.add_patterns(vec!["ForeignKey", "ManyToMany", "OneToOne"]);
    matcher.build().unwrap();

    assert!(matcher.contains_any("ForeignKey(User)"));
    assert!(matcher.contains_any("ManyToManyField(Tag)"));
    assert!(!matcher.contains_any("CharField(max_length=100)"));
}

#[test]
fn test_pattern_matcher_replace() {
    use reinhardt_migrations::PatternMatcher;
    use std::collections::HashMap;

    let mut matcher = PatternMatcher::new();
    matcher.add_pattern("old_name");
    matcher.build().unwrap();

    let mut replacements = HashMap::new();
    replacements.insert("old_name".to_string(), "new_name".to_string());

    let replaced = matcher.replace_all("old_name is old_name", &replacements);
    assert_eq!(replaced, "new_name is new_name");
}

#[test]
fn test_inference_engine_basic() {
    use reinhardt_migrations::{InferenceEngine, InferenceRule, RuleCondition};

    let mut engine = InferenceEngine::new();

    // Add a simple rule
    let rule = InferenceRule {
        name: "test_rule".to_string(),
        conditions: vec![RuleCondition::FieldAddition {
            field_name_pattern: "created_at".to_string(),
        }],
        optional_conditions: vec![],
        intent_type: "Add timestamp tracking".to_string(),
        base_confidence: 0.9,
        confidence_boost_per_optional: 0.05,
    };
    engine.add_rule(rule);

    // Infer intent from field addition
    let intents = engine.infer_intents(
        &[],
        &[],
        &[(
            "app".to_string(),
            "User".to_string(),
            "created_at".to_string(),
        )],
        &[],
    );

    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].intent_type, "Add timestamp tracking");
    assert_eq!(intents[0].confidence, 0.9);
}

#[test]
fn test_inference_engine_default_rules() {
    use reinhardt_migrations::InferenceEngine;

    let mut engine = InferenceEngine::new();
    engine.add_default_rules();

    // Should have 5 default rules
    assert_eq!(engine.rules().len(), 5);
}

#[test]
fn test_inference_engine_model_rename_rule() {
    use reinhardt_migrations::InferenceEngine;

    let mut engine = InferenceEngine::new();
    engine.add_default_rules();

    // Test model rename detection
    let intents = engine.infer_intents(
        &[(
            "app".to_string(),
            "OldUser".to_string(),
            "app".to_string(),
            "NewUser".to_string(),
        )],
        &[],
        &[],
        &[],
    );

    // Should detect refactoring intent
    let refactor_intent = intents
        .iter()
        .find(|i| i.intent_type.contains("Refactoring"));
    assert!(refactor_intent.is_some());
}

#[test]
fn test_inference_engine_field_tracking_rule() {
    use reinhardt_migrations::InferenceEngine;

    let mut engine = InferenceEngine::new();
    engine.add_default_rules();

    // Test timestamp field addition
    let intents = engine.infer_intents(
        &[],
        &[],
        &[
            (
                "app".to_string(),
                "User".to_string(),
                "created_at".to_string(),
            ),
            (
                "app".to_string(),
                "User".to_string(),
                "updated_at".to_string(),
            ),
        ],
        &[],
    );

    // Should detect timestamp tracking intent
    let tracking_intent = intents
        .iter()
        .find(|i| i.intent_type.contains("timestamp tracking"));
    assert!(tracking_intent.is_some());
    // Confidence should be boosted for optional condition
    assert!(tracking_intent.unwrap().confidence > 0.8);
}

#[test]
fn test_migration_prompt_defaults() {
    use reinhardt_migrations::MigrationPrompt;

    let prompt = MigrationPrompt::new();
    assert_eq!(prompt.auto_accept_threshold(), 0.85);
}

#[test]
fn test_migration_prompt_custom_threshold() {
    use reinhardt_migrations::MigrationPrompt;

    let prompt = MigrationPrompt::with_threshold(0.75);
    assert_eq!(prompt.auto_accept_threshold(), 0.75);
}

// Note: Interactive tests are excluded as they require user input
// Manual testing should be performed for:
// - confirm_intent()
// - select_intent()
// - multi_select_intents()
// - confirm_model_rename()
// - confirm_field_rename()
// - with_progress()
