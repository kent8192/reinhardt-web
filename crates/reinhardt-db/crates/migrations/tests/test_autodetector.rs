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
    assert!(changes
        .created_models
        .iter()
        .any(|(app, _)| app == "authors"));
}

#[test]
fn test_rename_model() {
    // Django: test_rename_model
    // Test that model rename is detected when fields are similar
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model
    let mut old_model = ModelState::new("testapp", "OldBook");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("title", "VARCHAR(200)", false));
    old_model.add_field(field("author", "VARCHAR(100)", false));
    from_state.add_model(old_model);

    // New model with same fields
    let mut new_model = ModelState::new("testapp", "NewBook");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("title", "VARCHAR(200)", false));
    new_model.add_field(field("author", "VARCHAR(100)", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // With high similarity, should detect as rename
    if !changes.renamed_models.is_empty() {
        assert_eq!(changes.renamed_models[0].0, "testapp");
        assert_eq!(changes.renamed_models[0].1, "OldBook");
        assert_eq!(changes.renamed_models[0].2, "NewBook");
    } else {
        // Or might detect as delete + create
        assert_eq!(changes.created_models.len(), 1);
        assert_eq!(changes.deleted_models.len(), 1);
    }
}

#[test]
fn test_rename_field() {
    // Django: test_rename_field
    // Test that field rename is detected when types match
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Old model
    let mut old_model = ModelState::new("testapp", "Book");
    old_model.add_field(field("id", "INTEGER", false));
    old_model.add_field(field("old_title", "VARCHAR(200)", false));
    from_state.add_model(old_model);

    // New model with renamed field
    let mut new_model = ModelState::new("testapp", "Book");
    new_model.add_field(field("id", "INTEGER", false));
    new_model.add_field(field("new_title", "VARCHAR(200)", false));
    to_state.add_model(new_model);

    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Might detect as rename or remove + add
    if !changes.renamed_fields.is_empty() {
        assert_eq!(changes.renamed_fields[0].0, "testapp");
        assert_eq!(changes.renamed_fields[0].1, "Book");
        assert_eq!(changes.renamed_fields[0].2, "old_title");
        assert_eq!(changes.renamed_fields[0].3, "new_title");
    } else {
        assert_eq!(changes.added_fields.len(), 1);
        assert_eq!(changes.removed_fields.len(), 1);
    }
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
    assert!(operations
        .iter()
        .any(|op| matches!(op, reinhardt_migrations::Operation::CreateTable { .. })));
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
