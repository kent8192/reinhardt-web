//! Tests for migration writer
//! Translated from Django's test_writer.py

use reinhardt_migrations::{ColumnDefinition, Migration, MigrationWriter, Operation};

#[test]
fn test_write_simple_migration() {
    // Test writing a simple migration with one operation
    let migration =
        Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
                ColumnDefinition::new("username", "VARCHAR(150) NOT NULL"),
            ],
            constraints: vec![],
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    // Verify generated content
    assert!(
        content.contains("0001_initial"),
        "マイグレーション名 '0001_initial' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("testapp"),
        "アプリ名 'testapp' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("CreateTable"),
        "Operation::CreateTable が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("users"),
        "テーブル名 'users' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("id"),
        "カラム名 'id' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("username"),
        "カラム名 'username' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("INTEGER PRIMARY KEY"),
        "カラム定義 'INTEGER PRIMARY KEY' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("VARCHAR(150) NOT NULL"),
        "カラム定義 'VARCHAR(150) NOT NULL' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_add_column_migration() {
    // Test writing a migration that adds a column
    let migration =
        Migration::new("0002_add_email", "testapp").add_operation(Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition::new("email", "VARCHAR(255)"),
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(
        content.contains("0002_add_email"),
        "マイグレーション名 '0002_add_email' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("AddColumn"),
        "Operation::AddColumn が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("email"),
        "カラム名 'email' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("VARCHAR(255)"),
        "カラム定義 'VARCHAR(255)' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_drop_column_migration() {
    // Test writing a migration that drops a column
    let migration =
        Migration::new("0003_remove_email", "testapp").add_operation(Operation::DropColumn {
            table: "users".to_string(),
            column: "email".to_string(),
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(
        content.contains("0003_remove_email"),
        "マイグレーション名 '0003_remove_email' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("DropColumn"),
        "Operation::DropColumn が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("email"),
        "カラム名 'email' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_alter_column_migration() {
    // Test writing a migration that alters a column
    let migration =
        Migration::new("0004_alter_username", "testapp").add_operation(Operation::AlterColumn {
            table: "users".to_string(),
            column: "username".to_string(),
            new_definition: ColumnDefinition::new("username", "VARCHAR(200) NOT NULL"),
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(
        content.contains("0004_alter_username"),
        "マイグレーション名 '0004_alter_username' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("AlterColumn"),
        "Operation::AlterColumn が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("username"),
        "カラム名 'username' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("VARCHAR(200) NOT NULL"),
        "カラム定義 'VARCHAR(200) NOT NULL' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_drop_table_migration() {
    // Test writing a migration that drops a table
    let migration =
        Migration::new("0005_delete_users", "testapp").add_operation(Operation::DropTable {
            name: "users".to_string(),
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(
        content.contains("0005_delete_users"),
        "マイグレーション名 '0005_delete_users' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("DropTable"),
        "Operation::DropTable が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("users"),
        "テーブル名 'users' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_migration_with_dependencies() {
    // Test writing a migration with dependencies
    let migration = Migration::new("0002_add_profile", "users")
        .add_dependency("auth", "0001_initial")
        .add_operation(Operation::CreateTable {
            name: "profile".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
                ColumnDefinition::new("user_id", "INTEGER NOT NULL"),
            ],
            constraints: vec![],
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(
        content.contains("0002_add_profile"),
        "マイグレーション名 '0002_add_profile' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("add_dependency"),
        "依存関係メソッド 'add_dependency' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("auth"),
        "依存先アプリ 'auth' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("0001_initial"),
        "依存先マイグレーション '0001_initial' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_migration_with_multiple_operations() {
    // Test writing a migration with multiple operations
    let migration = Migration::new("0006_complex", "testapp")
        .add_operation(Operation::CreateTable {
            name: "categories".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
                ColumnDefinition::new("name", "VARCHAR(100) NOT NULL"),
            ],
            constraints: vec![],
        })
        .add_operation(Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition::new("category_id", "INTEGER"),
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(
        content.contains("0006_complex"),
        "マイグレーション名 '0006_complex' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("CreateTable"),
        "Operation::CreateTable が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("categories"),
        "テーブル名 'categories' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("AddColumn"),
        "Operation::AddColumn が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("category_id"),
        "カラム名 'category_id' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_migration_file_format() {
    // Test that the generated migration file has correct format
    let migration = Migration::new("0001_initial", "myapp").add_operation(Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![ColumnDefinition::new("id", "INTEGER")],
        constraints: vec![],
    });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    // Check file header
    assert!(
        content.contains("//! Auto-generated migration"),
        "ファイルヘッダー '//! Auto-generated migration' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("//! Name: 0001_initial"),
        "マイグレーション名ヘッダー '//! Name: 0001_initial' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("//! App: myapp"),
        "アプリ名ヘッダー '//! App: myapp' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );

    // Check imports
    assert!(
        content.contains("use reinhardt_migrations"),
        "インポート文 'use reinhardt_migrations' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );

    // Check function definition
    assert!(
        content.contains("pub fn migration_0001_initial() -> Migration"),
        "関数定義 'pub fn migration_0001_initial() -> Migration' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("Migration::new(\"0001_initial\", \"myapp\")"),
        "マイグレーション初期化コード 'Migration::new(\"0001_initial\", \"myapp\")' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}

#[test]
fn test_write_to_file() {
    // Test writing migration to actual file
    let migration =
        Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            constraints: vec![],
        });

    let temp_dir = std::env::temp_dir().join("reinhardt_test_migrations");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let writer = MigrationWriter::new(migration);
    let filepath = writer.write_to_file(&temp_dir).unwrap();

    // Verify file was created
    assert!(
        std::path::Path::new(&filepath).exists(),
        "マイグレーションファイルが作成されませんでした: {}",
        filepath
    );

    // Verify file content
    let content = std::fs::read_to_string(&filepath).unwrap();
    assert!(
        content.contains("0001_initial"),
        "ファイル内容にマイグレーション名 '0001_initial' が含まれていません。\nファイルパス: {}\n内容:\n{}",
        filepath,
        content
    );
    assert!(
        content.contains("testapp"),
        "ファイル内容にアプリ名 'testapp' が含まれていません。\nファイルパス: {}\n内容:\n{}",
        filepath,
        content
    );

    // Cleanup
    std::fs::remove_file(&filepath).unwrap();
}

#[test]
fn test_serialization_indentation() {
    // Test that the serialization maintains proper indentation
    let migration =
        Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("name", "VARCHAR(100)"),
            ],
            constraints: vec![],
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    // Check that proper indentation is maintained
    assert!(
        content.contains("    .add_operation"),
        "インデント '    .add_operation' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("        name:"),
        "インデント '        name:' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
    assert!(
        content.contains("        columns: vec!["),
        "インデント '        columns: vec![' が生成コードに含まれていません。\n生成されたコード:\n{}",
        content
    );
}
