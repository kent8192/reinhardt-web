//! Tests for composite primary key support in migrations

use reinhardt_migrations::operations::models::CreateModel;
use reinhardt_migrations::operations::FieldDefinition;
use reinhardt_migrations::schema_editor::BaseDatabaseSchemaEditor;

/// Mock schema editor for testing SQL generation
struct MockSchemaEditor;

impl BaseDatabaseSchemaEditor for MockSchemaEditor {
    fn create_table_sql(&self, table_name: &str, columns: &[(&str, &str)]) -> String {
        let column_defs: Vec<String> = columns
            .iter()
            .map(|(name, type_def)| format!("{} {}", name, type_def))
            .collect();
        format!("CREATE TABLE {} ({})", table_name, column_defs.join(", "))
    }

    fn drop_table_sql(&self, _table_name: &str) -> String {
        // TODO: Implement drop_table_sql for complete schema editor mock
        unimplemented!()
    }

    fn add_column_sql(&self, _table_name: &str, _column_name: &str, _column_type: &str) -> String {
        // TODO: Implement add_column_sql for complete schema editor mock
        unimplemented!()
    }

    fn drop_column_sql(&self, _table_name: &str, _column_name: &str) -> String {
        // TODO: Implement drop_column_sql for complete schema editor mock
        unimplemented!()
    }

    fn alter_column_sql(&self, _table_name: &str, _column_name: &str, _new_type: &str) -> String {
        // TODO: Implement alter_column_sql for complete schema editor mock
        unimplemented!()
    }

    fn rename_column_sql(&self, _table_name: &str, _old_name: &str, _new_name: &str) -> String {
        // TODO: Implement rename_column_sql for complete schema editor mock
        unimplemented!()
    }

    fn add_constraint_sql(&self, _table_name: &str, _constraint: &str) -> String {
        // TODO: Implement add_constraint_sql for complete schema editor mock
        unimplemented!()
    }

    fn drop_constraint_sql(&self, _table_name: &str, _constraint_name: &str) -> String {
        // TODO: Implement drop_constraint_sql for complete schema editor mock
        unimplemented!()
    }
}

#[test]
fn test_create_model_with_single_primary_key() {
    let create = CreateModel::new(
        "users",
        vec![
            FieldDefinition::new("id", "INTEGER", true, false, None),
            FieldDefinition::new("name", "VARCHAR(100)", false, false, None),
        ],
    );

    let schema_editor = MockSchemaEditor;
    let sql_statements = create.database_forwards(&schema_editor);

    assert_eq!(sql_statements.len(), 1);
    let sql = &sql_statements[0];

    assert!(sql.contains("CREATE TABLE users"));
    assert!(sql.contains("id INTEGER PRIMARY KEY"));
    assert!(sql.contains("name VARCHAR(100)"));
}

#[test]
fn test_create_model_with_composite_primary_key() {
    let create = CreateModel::new(
        "post_tags",
        vec![
            FieldDefinition::new("post_id", "INTEGER", true, false, None),
            FieldDefinition::new("tag_id", "INTEGER", true, false, None),
            FieldDefinition::new("description", "VARCHAR(200)", false, false, None),
        ],
    )
    .with_composite_primary_key(vec!["post_id".to_string(), "tag_id".to_string()]);

    let schema_editor = MockSchemaEditor;
    let sql_statements = create.database_forwards(&schema_editor);

    assert_eq!(sql_statements.len(), 1);
    let sql = &sql_statements[0];

    assert!(sql.contains("CREATE TABLE post_tags"));
    // Individual fields should NOT have PRIMARY KEY
    assert!(!sql.contains("post_id INTEGER PRIMARY KEY"));
    assert!(!sql.contains("tag_id INTEGER PRIMARY KEY"));
    // Should have NOT NULL for composite PK fields
    assert!(sql.contains("post_id INTEGER NOT NULL"));
    assert!(sql.contains("tag_id INTEGER NOT NULL"));
    // Should have table-level PRIMARY KEY constraint
    assert!(sql.contains("PRIMARY KEY (post_id, tag_id)"));
}

#[test]
fn test_create_model_composite_pk_three_fields() {
    let create = CreateModel::new(
        "user_role_permission",
        vec![
            FieldDefinition::new("user_id", "INTEGER", true, false, None),
            FieldDefinition::new("role_id", "INTEGER", true, false, None),
            FieldDefinition::new("permission_id", "INTEGER", true, false, None),
        ],
    )
    .with_composite_primary_key(vec![
        "user_id".to_string(),
        "role_id".to_string(),
        "permission_id".to_string(),
    ]);

    let schema_editor = MockSchemaEditor;
    let sql_statements = create.database_forwards(&schema_editor);

    let sql = &sql_statements[0];
    assert!(sql.contains("PRIMARY KEY (user_id, role_id, permission_id)"));
}

#[test]
fn test_create_model_composite_pk_with_additional_fields() {
    let create = CreateModel::new(
        "order_items",
        vec![
            FieldDefinition::new("order_id", "INTEGER", true, false, None),
            FieldDefinition::new("item_id", "INTEGER", true, false, None),
            FieldDefinition::new("quantity", "INTEGER", false, false, Some("1")),
            FieldDefinition::new("price", "DECIMAL(10, 2)", false, false, None),
        ],
    )
    .with_composite_primary_key(vec!["order_id".to_string(), "item_id".to_string()]);

    let schema_editor = MockSchemaEditor;
    let sql_statements = create.database_forwards(&schema_editor);

    let sql = &sql_statements[0];
    assert!(sql.contains("PRIMARY KEY (order_id, item_id)"));
    assert!(sql.contains("quantity INTEGER NOT NULL DEFAULT 1"));
    assert!(sql.contains("price DECIMAL(10, 2) NOT NULL"));
}
