//! Schema Construction Tests
//!
//! Tests for basic OpenAPI schema structure, info objects, and empty handling.

use reinhardt_openapi::{Info, OpenApiSchema};

#[test]
fn test_schema_construction() {
    // Test basic schema structure with openapi version and paths
    let schema = OpenApiSchema::new("Test API", "1.0.0");

    assert_eq!(schema.openapi, "3.0.3");
    assert_eq!(schema.info.title, "Test API");
    assert_eq!(schema.info.version, "1.0.0");
    assert!(schema.paths.is_empty());
}

#[test]
fn test_schema_information() {
    // Test info object with title, version, and description
    let mut schema = OpenApiSchema::new("My API", "1.2.3");
    schema.info.description = Some("My description".to_string());

    assert_eq!(schema.info.title, "My API");
    assert_eq!(schema.info.version, "1.2.3");
    assert_eq!(schema.info.description, Some("My description".to_string()));
}

#[test]
fn test_schema_information_empty() {
    // Test empty info defaults
    let schema = OpenApiSchema::new("", "");

    assert_eq!(schema.info.title, "");
    assert_eq!(schema.info.version, "");
    assert_eq!(schema.info.description, None);
}

#[test]
fn test_schema_with_no_paths() {
    // Test empty paths handling
    let schema = OpenApiSchema::new("Test API", "1.0.0");

    assert!(schema.paths.is_empty());
}

#[test]
fn test_schema_info_contact() {
    // Test contact information
    use reinhardt_openapi::openapi::Contact;

    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    schema.info.contact = Some(Contact {
        name: Some("Support Team".to_string()),
        url: Some("https://example.com".to_string()),
        email: Some("support@example.com".to_string()),
    });

    let contact = schema.info.contact.as_ref().unwrap();
    assert_eq!(contact.name, Some("Support Team".to_string()));
    assert_eq!(contact.url, Some("https://example.com".to_string()));
    assert_eq!(contact.email, Some("support@example.com".to_string()));
}

#[test]
fn test_schema_info_license() {
    // Test license information
    use reinhardt_openapi::openapi::License;

    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    schema.info.license = Some(License {
        name: "MIT".to_string(),
        url: Some("https://opensource.org/licenses/MIT".to_string()),
    });

    let license = schema.info.license.as_ref().unwrap();
    assert_eq!(license.name, "MIT");
    assert_eq!(
        license.url,
        Some("https://opensource.org/licenses/MIT".to_string())
    );
}

#[test]
fn test_schema_openapi_version() {
    // Test that OpenAPI version is correctly set
    let schema = OpenApiSchema::new("Test API", "1.0.0");

    // Should be OpenAPI 3.0.3
    assert!(schema.openapi.starts_with("3.0"));
    assert_eq!(schema.openapi, "3.0.3");
}
