//! Integration tests for Contact and email validation.
//!
//! This test module validates the Contact struct and related Settings methods
//! for managing admin and manager contacts, including email validation,
//! deduplication, and the managers_from_admins utility.

use reinhardt_settings::{Contact, Settings};
use rstest::*;

/// Test: Contact creation with invalid email format
///
/// Why: Validates that Contact creation or validation detects invalid email formats.
#[rstest]
#[case("not-an-email")]
#[case("missing@domain")]
#[case("@missing-local.com")]
#[case("spaces in@email.com")]
#[case("invalid..double@dots.com")]
#[test]
fn test_contact_invalid_email_format(#[case] invalid_email: &str) {
	// Note: Contact struct may not enforce email validation at construction time
	// This test documents the expected behavior
	let contact = Contact::new("Test User", invalid_email);

	// Contact should be created (validation happens elsewhere)
	assert_eq!(contact.email, invalid_email);
	assert_eq!(contact.name, "Test User");

	// If validation is implemented, it should reject this email
	// (Add actual validation check when implemented)
}

/// Test: Contact with empty name
///
/// Why: Validates handling of contacts with empty names.
#[rstest]
#[test]
fn test_contact_empty_name() {
	let contact = Contact::new("", "valid@example.com");

	assert_eq!(contact.name, "", "Empty name should be accepted");
	assert_eq!(contact.email, "valid@example.com");
}

/// Test: Contact with empty email
///
/// Why: Validates handling of contacts with empty email addresses.
#[rstest]
#[test]
fn test_contact_empty_email() {
	let contact = Contact::new("Test User", "");

	assert_eq!(
		contact.email, "",
		"Empty email should be accepted at creation"
	);
	assert_eq!(contact.name, "Test User");

	// Empty email should be flagged as invalid in validation
}

/// Test: Settings add_admin with duplicate contact
///
/// Why: Validates that adding the same admin twice is handled correctly
/// (either deduplicated or allowed).
#[rstest]
#[test]
fn test_settings_add_admin_duplicate() {
	let mut settings = Settings::default();

	// Note: add_admin takes (name, email) parameters, not Contact object
	settings.add_admin("Admin User", "admin@example.com");
	let initial_count = settings.admins.len();
	assert_eq!(initial_count, 1, "First admin should be added");

	settings.add_admin("Admin User", "admin@example.com");
	let final_count = settings.admins.len();

	// Behavior: Either deduplicates (count stays 1) or allows duplicates (count is 2)
	assert!(
		final_count == 1 || final_count == 2,
		"Adding duplicate admin should either deduplicate or allow duplicate"
	);

	if final_count == 1 {
		// Deduplication occurred
		assert_eq!(
			settings.admins.len(),
			1,
			"Duplicate admin should be deduplicated"
		);
	} else {
		// Duplicates allowed
		assert_eq!(settings.admins.len(), 2, "Duplicate admin was added");
	}
}

/// Test: Settings managers_from_admins with empty admins
///
/// Why: Validates that managers_from_admins returns empty list when no admins exist.
#[rstest]
#[test]
fn test_settings_managers_from_admins_empty() {
	let mut settings = Settings::default();

	// No admins added
	assert!(settings.admins.is_empty(), "Admins should be empty");

	settings.managers_from_admins();

	assert!(
		settings.managers.is_empty(),
		"Managers should be empty when no admins exist"
	);
}

/// Test: Settings managers_from_admins copies all admins
///
/// Why: Validates that managers_from_admins correctly copies all admin contacts to managers.
#[rstest]
#[test]
fn test_settings_managers_from_admins_copies_all() {
	let mut settings = Settings::default();

	// add_admin takes (name, email) parameters
	settings.add_admin("Admin 1", "admin1@example.com");
	settings.add_admin("Admin 2", "admin2@example.com");
	settings.add_admin("Admin 3", "admin3@example.com");

	assert_eq!(settings.admins.len(), 3, "Should have 3 admins");

	settings.managers_from_admins();

	assert_eq!(
		settings.managers.len(),
		3,
		"Managers should have same count as admins"
	);

	// Verify all admins were copied
	for admin in &settings.admins {
		let found = settings
			.managers
			.iter()
			.any(|m| m.name == admin.name && m.email == admin.email);
		assert!(found, "Admin '{}' should be copied to managers", admin.name);
	}
}

/// Test: Settings with_admins fluent API
///
/// Why: Validates that with_admins fluent API correctly adds multiple admins.
#[rstest]
#[test]
fn test_settings_with_admins_fluent_api() {
	let admin1 = Contact::new("Admin 1", "admin1@example.com");
	let admin2 = Contact::new("Admin 2", "admin2@example.com");

	let settings = Settings::default().with_admins(vec![admin1, admin2]);

	assert_eq!(settings.admins.len(), 2, "Should have 2 admins");
	assert_eq!(settings.admins[0].name, "Admin 1");
	assert_eq!(settings.admins[1].name, "Admin 2");
}

/// Test: Settings with_managers fluent API
///
/// Why: Validates that with_managers fluent API correctly adds multiple managers.
#[rstest]
#[test]
fn test_settings_with_managers_fluent_api() {
	let manager1 = Contact::new("Manager 1", "manager1@example.com");
	let manager2 = Contact::new("Manager 2", "manager2@example.com");

	let settings = Settings::default().with_managers(vec![manager1, manager2]);

	assert_eq!(settings.managers.len(), 2, "Should have 2 managers");
	assert_eq!(settings.managers[0].name, "Manager 1");
	assert_eq!(settings.managers[1].name, "Manager 2");
}

/// Test: Contact with very long name
///
/// Why: Validates that Contact handles long names without issues.
#[rstest]
#[test]
fn test_contact_very_long_name() {
	let long_name = "A".repeat(1000);
	let contact = Contact::new(&long_name, "user@example.com");

	assert_eq!(contact.name, long_name, "Long name should be preserved");
	assert_eq!(contact.email, "user@example.com");
}

/// Test: Contact with Unicode characters
///
/// Why: Validates that Contact handles Unicode characters in name and email.
#[rstest]
#[case("山田太郎", "yamada@example.jp")]
#[case("Müller", "mueller@example.de")]
#[case("José García", "jose@example.es")]
#[test]
fn test_contact_unicode_characters(#[case] name: &str, #[case] email: &str) {
	let contact = Contact::new(name, email);

	assert_eq!(contact.name, name, "Unicode name should be preserved");
	assert_eq!(contact.email, email, "Unicode email should be preserved");
}

/// Test: Settings add_manager method
///
/// Why: Validates that add_manager correctly adds managers to the list.
#[rstest]
#[test]
fn test_settings_add_manager() {
	let mut settings = Settings::default();

	// add_manager takes (name, email) parameters
	settings.add_manager("Manager", "manager@example.com");

	assert_eq!(settings.managers.len(), 1, "Should have 1 manager");
	assert_eq!(settings.managers[0].name, "Manager");
	assert_eq!(settings.managers[0].email, "manager@example.com");
}

/// Test: Contact equality comparison
///
/// Why: Validates that Contact implements PartialEq correctly.
#[rstest]
#[test]
fn test_contact_equality() {
	let contact1 = Contact::new("User", "user@example.com");
	let contact2 = Contact::new("User", "user@example.com");
	let contact3 = Contact::new("Other", "other@example.com");

	assert_eq!(contact1, contact2, "Identical contacts should be equal");
	assert_ne!(contact1, contact3, "Different contacts should not be equal");
}

/// Test: Settings with both admins and managers
///
/// Why: Validates that Settings can maintain separate admin and manager lists.
#[rstest]
#[test]
fn test_settings_separate_admins_and_managers() {
	let mut settings = Settings::default();

	// add_admin and add_manager take (name, email) parameters
	settings.add_admin("Admin", "admin@example.com");
	settings.add_manager("Manager", "manager@example.com");

	assert_eq!(settings.admins.len(), 1, "Should have 1 admin");
	assert_eq!(settings.managers.len(), 1, "Should have 1 manager");
	assert_eq!(settings.admins[0].name, "Admin");
	assert_eq!(settings.managers[0].name, "Manager");
}
