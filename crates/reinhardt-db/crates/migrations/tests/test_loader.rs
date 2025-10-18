//! Tests for migration loader
//! Adapted from Django's test_loader.py

use reinhardt_migrations::{MigrationLoader, MigrationRecorder};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_loader_creation() {
    // Test creating a MigrationLoader
    let _loader = MigrationLoader::new("/tmp/migrations".into());
    // MigrationLoader is created successfully if this doesn't panic
}

// ===== MigrationRecorder Tests =====

#[test]
fn test_recorder_creation() {
    let recorder = MigrationRecorder::new();
    // Recorder is created successfully
    assert!(recorder.get_applied_migrations().is_empty());
}

#[test]
fn test_recorder_record_applied() {
    let mut recorder = MigrationRecorder::new();

    recorder.record_applied("testapp".to_string(), "0001_initial".to_string());
    assert_eq!(recorder.get_applied_migrations().len(), 1);
}

#[test]
fn test_recorder_is_applied() {
    let mut recorder = MigrationRecorder::new();

    // Record a migration
    recorder.record_applied("testapp".to_string(), "0001_initial".to_string());

    // Check if it's applied
    assert!(recorder.is_applied("testapp", "0001_initial"));

    // Check a non-existent migration
    assert!(!recorder.is_applied("testapp", "0002_next"));
}

#[test]
fn test_recorder_applied_migrations() {
    let mut recorder = MigrationRecorder::new();

    // Record multiple migrations
    recorder.record_applied("app1".to_string(), "0001_initial".to_string());
    recorder.record_applied("app1".to_string(), "0002_add_field".to_string());
    recorder.record_applied("app2".to_string(), "0001_initial".to_string());

    // Get all applied migrations
    let migrations = recorder.get_applied_migrations();
    assert_eq!(migrations.len(), 3);

    // Check first migration
    assert_eq!(migrations[0].app, "app1");
    assert_eq!(migrations[0].name, "0001_initial");
}

#[test]
fn test_recorder_multiple_apps() {
    let mut recorder = MigrationRecorder::new();

    // Record migrations for different apps
    recorder.record_applied("app1".to_string(), "0001_initial".to_string());
    recorder.record_applied("app2".to_string(), "0001_initial".to_string());
    recorder.record_applied("app3".to_string(), "0001_initial".to_string());

    let migrations = recorder.get_applied_migrations();
    assert_eq!(migrations.len(), 3);

    // Check app names
    let app_names: Vec<&str> = migrations.iter().map(|m| m.app.as_str()).collect();
    assert!(app_names.contains(&"app1"));
    assert!(app_names.contains(&"app2"));
    assert!(app_names.contains(&"app3"));
}

#[test]
fn test_recorder_migration_order() {
    let mut recorder = MigrationRecorder::new();

    // Record migrations in order
    recorder.record_applied("app1".to_string(), "0001_initial".to_string());
    recorder.record_applied("app1".to_string(), "0002_add_field".to_string());
    recorder.record_applied("app1".to_string(), "0003_alter_field".to_string());

    let migrations = recorder.get_applied_migrations();
    assert_eq!(migrations.len(), 3);

    // Verify order is preserved
    assert_eq!(migrations[0].name, "0001_initial");
    assert_eq!(migrations[1].name, "0002_add_field");
    assert_eq!(migrations[2].name, "0003_alter_field");
}

#[test]
fn test_recorder_duplicate_record() {
    let mut recorder = MigrationRecorder::new();

    // Record same migration twice
    recorder.record_applied("testapp".to_string(), "0001_initial".to_string());
    recorder.record_applied("testapp".to_string(), "0001_initial".to_string());

    // Both records will be added (no uniqueness constraint)
    let migrations = recorder.get_applied_migrations();
    assert_eq!(migrations.len(), 2);

    // Verify it's still recorded as applied
    assert!(recorder.is_applied("testapp", "0001_initial"));
}

// ===== MigrationLoader Tests =====

#[test]
fn test_loader_load_disk_empty() {
    let temp_dir = create_temp_dir("test_load_disk_empty");
    let mut loader = MigrationLoader::new(temp_dir.clone());

    let result = loader.load_disk();
    assert!(result.is_ok());
    assert_eq!(loader.get_all_migrations().len(), 0);

    cleanup_temp_dir(&temp_dir);
}

#[test]
fn test_loader_load_single_migration() {
    let temp_dir = create_temp_dir("test_single_migration");
    let app_dir = temp_dir.join("testapp");
    fs::create_dir_all(&app_dir).unwrap();

    // Create a simple migration file
    let migration_json = r#"{
        "app_label": "testapp",
        "name": "0001_initial",
        "dependencies": [],
        "replaces": [],
        "atomic": true,
        "operations": []
    }"#;
    fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();

    let mut loader = MigrationLoader::new(temp_dir.clone());
    loader.load_disk().unwrap();

    assert_eq!(loader.get_all_migrations().len(), 1);
    assert!(loader.has_migrations("testapp"));

    let migration = loader.get_migration("testapp", "0001_initial");
    assert!(migration.is_some());

    cleanup_temp_dir(&temp_dir);
}

#[test]
fn test_loader_multiple_apps() {
    let temp_dir = create_temp_dir("test_multiple_apps");

    // Create migrations for multiple apps
    for app_name in ["app1", "app2", "app3"] {
        let app_dir = temp_dir.join(app_name);
        fs::create_dir_all(&app_dir).unwrap();

        let migration_json = format!(
            r#"{{
                "app_label": "{}",
                "name": "0001_initial",
                "dependencies": [],
                "replaces": [],
                "atomic": true,
                "operations": []
            }}"#,
            app_name
        );
        fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();
    }

    let mut loader = MigrationLoader::new(temp_dir.clone());
    loader.load_disk().unwrap();

    assert_eq!(loader.get_all_migrations().len(), 3);
    assert!(loader.has_migrations("app1"));
    assert!(loader.has_migrations("app2"));
    assert!(loader.has_migrations("app3"));

    cleanup_temp_dir(&temp_dir);
}

#[test]
fn test_loader_skip_non_migration_files() {
    let temp_dir = create_temp_dir("test_skip_files");
    let app_dir = temp_dir.join("testapp");
    fs::create_dir_all(&app_dir).unwrap();

    // Create valid migration
    let migration_json = r#"{
        "app_label": "testapp",
        "name": "0001_initial",
        "dependencies": [],
        "replaces": [],
        "atomic": true,
        "operations": []
    }"#;
    fs::write(app_dir.join("0001_initial.json"), migration_json).unwrap();

    // Create files that should be skipped
    fs::write(app_dir.join("__init__.py"), "").unwrap();
    fs::write(app_dir.join("_helper.json"), "{}").unwrap();
    fs::write(app_dir.join("~temp.json"), "{}").unwrap();
    fs::write(app_dir.join("README.md"), "# Migrations").unwrap();

    let mut loader = MigrationLoader::new(temp_dir.clone());
    loader.load_disk().unwrap();

    // Should only load the one valid migration
    assert_eq!(loader.get_all_migrations().len(), 1);

    cleanup_temp_dir(&temp_dir);
}

#[test]
fn test_loader_get_app_migrations() {
    let temp_dir = create_temp_dir("test_app_migrations");
    let app_dir = temp_dir.join("myapp");
    fs::create_dir_all(&app_dir).unwrap();

    // Create multiple migrations for one app
    for i in 1..=3 {
        let migration_json = format!(
            r#"{{
                "app_label": "myapp",
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

    let mut loader = MigrationLoader::new(temp_dir.clone());
    loader.load_disk().unwrap();

    let app_migrations = loader.get_app_migrations("myapp");
    assert_eq!(app_migrations.len(), 3);

    cleanup_temp_dir(&temp_dir);
}

#[test]
fn test_loader_get_migrations_by_prefix() {
    let temp_dir = create_temp_dir("test_prefix");
    let app_dir = temp_dir.join("testapp");
    fs::create_dir_all(&app_dir).unwrap();

    // Create migrations with different prefixes
    for (num, name) in [
        ("0001", "initial"),
        ("0002", "add_field"),
        ("0003", "alter_field"),
    ] {
        let migration_json = format!(
            r#"{{
                "app_label": "testapp",
                "name": "{}_{}",
                "dependencies": [],
                "replaces": [],
                "atomic": true,
                "operations": []
            }}"#,
            num, name
        );
        fs::write(
            app_dir.join(format!("{}_{}.json", num, name)),
            migration_json,
        )
        .unwrap();
    }

    let mut loader = MigrationLoader::new(temp_dir.clone());
    loader.load_disk().unwrap();

    let migrations = loader.get_migrations_by_prefix("testapp", "0001");
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].name, "0001_initial");

    cleanup_temp_dir(&temp_dir);
}

// ===== Helper Functions =====

fn create_temp_dir(name: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("reinhardt_test_{}", name));
    fs::create_dir_all(&temp_dir).ok();
    temp_dir
}

fn cleanup_temp_dir(path: &PathBuf) {
    fs::remove_dir_all(path).ok();
}
