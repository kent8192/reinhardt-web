//! Integration tests for installed_apps with reinhardt-core

use reinhardt_apps::{init_apps_checked, Settings};

#[test]
fn test_init_apps_with_macro() {
    // Note: This test demonstrates the usage pattern
    // The actual macro validation happens at compile time

    let apps = vec![
        "reinhardt.contrib.auth".to_string(),
        "reinhardt.contrib.contenttypes".to_string(),
    ];

    let result = init_apps_checked(|| apps);
    assert!(result.is_ok());
}

#[test]
fn test_settings_with_validated_apps() {
    let settings = Settings::default().with_validated_apps(|| {
        vec![
            "reinhardt.contrib.auth".to_string(),
            "reinhardt.contrib.sessions".to_string(),
        ]
    });

    assert_eq!(settings.installed_apps.len(), 2);
    assert!(settings
        .installed_apps
        .contains(&"reinhardt.contrib.auth".to_string()));
}

#[test]
fn test_settings_builder_pattern() {
    use std::path::PathBuf;

    let settings = Settings::new(PathBuf::from("."), "secret".to_string())
        .with_validated_apps(|| {
            vec![
                "reinhardt.contrib.auth".to_string(),
                "reinhardt.contrib.contenttypes".to_string(),
                "reinhardt.contrib.sessions".to_string(),
            ]
        })
        .with_root_urlconf("config.urls");

    assert_eq!(settings.installed_apps.len(), 3);
    assert_eq!(settings.root_urlconf, "config.urls");
}
