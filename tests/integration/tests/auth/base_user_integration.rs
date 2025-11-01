use reinhardt_auth::{BaseUser, BaseUserManager, DefaultUser, DefaultUserManager};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::test]
async fn test_default_user_with_password() {
    let mut user = DefaultUser {
        id: Uuid::new_v4(),
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        first_name: "Alice".to_string(),
        last_name: "Smith".to_string(),
        password_hash: None,
        last_login: None,
        is_active: true,
        is_staff: false,
        is_superuser: false,
        date_joined: chrono::Utc::now(),
        user_permissions: Vec::new(),
        groups: Vec::new(),
    };

    // Test password hashing (uses Argon2id by default)
    user.set_password("securepass123").unwrap();
    assert!(user.password_hash().is_some());
    assert!(user.has_usable_password());

    // Test password verification
    assert!(user.check_password("securepass123").unwrap());
    assert!(!user.check_password("wrongpass").unwrap());

    // Test session auth hash
    let hash1 = user.get_session_auth_hash();
    user.set_password("newpassword").unwrap();
    let hash2 = user.get_session_auth_hash();
    assert_ne!(hash1, hash2);
}

#[tokio::test]
async fn test_default_user_manager_create_user() {
    let mut manager = DefaultUserManager::new();

    let mut extra = HashMap::new();
    extra.insert("email".to_string(), serde_json::json!("bob@example.com"));
    extra.insert(
        "first_name".to_string(),
        serde_json::json!("Bob"),
    );
    extra.insert(
        "last_name".to_string(),
        serde_json::json!("Johnson"),
    );

    let user = manager
        .create_user("bob", Some("password123"), extra)
        .await
        .unwrap();

    assert_eq!(user.username, "bob");
    assert_eq!(user.email, "bob@example.com");
    assert_eq!(user.first_name, "Bob");
    assert_eq!(user.last_name, "Johnson");
    assert!(user.is_active);
    assert!(!user.is_staff);
    assert!(!user.is_superuser);
    assert!(user.check_password("password123").unwrap());
}

#[tokio::test]
async fn test_default_user_manager_create_superuser() {
    let mut manager = DefaultUserManager::new();

    let admin = manager
        .create_superuser("admin", Some("adminsecret"), HashMap::new())
        .await
        .unwrap();

    assert_eq!(admin.username, "admin");
    assert!(admin.is_active);
    assert!(admin.is_staff);
    assert!(admin.is_superuser);
    assert!(admin.check_password("adminsecret").unwrap());
}

#[tokio::test]
async fn test_username_already_exists() {
    let mut manager = DefaultUserManager::new();

    manager
        .create_user("alice", Some("pass1"), HashMap::new())
        .await
        .unwrap();

    let result = manager
        .create_user("alice", Some("pass2"), HashMap::new())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_email_normalization() {
    let mut manager = DefaultUserManager::new();

    let mut extra = HashMap::new();
    extra.insert(
        "email".to_string(),
        serde_json::json!("Alice@EXAMPLE.COM"),
    );

    let user = manager
        .create_user("alice", Some("password"), extra)
        .await
        .unwrap();

    // Email domain should be lowercased
    assert_eq!(user.email, "Alice@example.com");
}

#[tokio::test]
async fn test_unusable_password() {
    let mut user = DefaultUser {
        id: Uuid::new_v4(),
        username: "oauth_user".to_string(),
        email: "oauth@example.com".to_string(),
        first_name: String::new(),
        last_name: String::new(),
        password_hash: None,
        last_login: None,
        is_active: true,
        is_staff: false,
        is_superuser: false,
        date_joined: chrono::Utc::now(),
        user_permissions: Vec::new(),
        groups: Vec::new(),
    };

    assert!(!user.has_usable_password());

    user.set_unusable_password();
    assert!(!user.has_usable_password());
    assert!(user.password_hash().is_some());
    assert_eq!(user.password_hash().unwrap(), "!");

    // Cannot login with unusable password
    assert!(!user.check_password("anypassword").unwrap());
}
