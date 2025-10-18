//! Flatpages integration tests
//!
//! Based on Django's flatpages tests from:
//! - django/tests/flatpages_tests/test_models.py

use reinhardt_contrib::{FlatPage, FlatPageError};

#[test]
fn test_flatpage_url_validation() {
    let mut page = FlatPage::new(
        "/valid-url/".to_string(),
        "Test Page".to_string(),
        "Content".to_string(),
    );

    assert!(page.validate_url().is_ok());
}

#[test]
fn test_flatpage_url_must_start_with_slash() {
    let mut page = FlatPage::new(
        "invalid-url".to_string(),
        "Test Page".to_string(),
        "Content".to_string(),
    );

    let result = page.validate_url();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), FlatPageError::InvalidUrl(_)));
}

#[test]
fn test_flatpage_url_cannot_be_empty() {
    let mut page = FlatPage::new(
        "".to_string(),
        "Test Page".to_string(),
        "Content".to_string(),
    );

    let result = page.validate_url();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), FlatPageError::InvalidUrl(_)));
}

#[test]
fn test_flatpage_creation_with_defaults() {
    let page = FlatPage::new(
        "/about/".to_string(),
        "About Us".to_string(),
        "Welcome to our site".to_string(),
    );

    assert_eq!(page.url, "/about/");
    assert_eq!(page.title, "About Us");
    assert_eq!(page.content, "Welcome to our site");
    assert!(!page.enable_comments);
    assert!(!page.registration_required);
    assert!(page.template_name.is_none());
}

#[test]
fn test_flatpage_with_custom_template() {
    let mut page = FlatPage::new(
        "/custom/".to_string(),
        "Custom".to_string(),
        "Content".to_string(),
    );
    page.template_name = Some("custom_template.html".to_string());

    assert_eq!(page.template_name, Some("custom_template.html".to_string()));
}

#[test]
fn test_flatpage_enable_comments() {
    let mut page = FlatPage::new(
        "/comments/".to_string(),
        "Comments Page".to_string(),
        "Content".to_string(),
    );
    page.enable_comments = true;

    assert!(page.enable_comments);
}

#[test]
fn test_flatpage_registration_required() {
    let mut page = FlatPage::new(
        "/members-only/".to_string(),
        "Members Only".to_string(),
        "Private content".to_string(),
    );
    page.registration_required = true;

    assert!(page.registration_required);
}
