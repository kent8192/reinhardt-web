//! Comprehensive test suite for migration autodetector
//!
//! This module tests the autodetector functionality including:
//! - Relation field detection (ForeignKey, OneToOne, ManyToMany)
//! - PostgreSQL-specific type detection
//! - Complex field type changes
//! - Edge cases and error handling
//! - Property-based testing
//!
//! Test organization follows the classification:
//! - Happy Path (normal operations)
//! - Error Path (invalid inputs)
//! - Edge Cases (boundary conditions)
//! - State Transitions (sequential operations)
//! - Use Cases (real-world scenarios)
//! - Fuzz/Property-based tests

use proptest::prelude::*;
use reinhardt_migrations::{
	ConstraintDefinition, DetectedChanges, FieldState, FieldType, ForeignKeyAction,
	IndexDefinition, MigrationAutodetector, ModelState, ProjectState, SimilarityConfig,
};
use rstest::*;
use std::collections::BTreeMap;

// =============================================================================
// Test Fixtures
// =============================================================================

/// Creates a basic field with the given parameters
fn field(name: &str, field_type: FieldType, nullable: bool) -> FieldState {
	FieldState::new(name.to_string(), field_type, nullable)
}

/// Creates a basic model with an id field
#[fixture]
fn basic_model() -> ModelState {
	let mut model = ModelState::new("testapp", "TestModel");
	model.add_field(field("id", FieldType::Integer, false));
	model
}

/// Creates an empty ProjectState
#[fixture]
fn empty_state() -> ProjectState {
	ProjectState::new()
}

/// Creates a ProjectState with a single basic model
#[fixture]
fn single_model_state(basic_model: ModelState) -> ProjectState {
	let mut state = ProjectState::new();
	state.add_model(basic_model);
	state
}

/// Creates a User model for relationship tests
#[fixture]
fn user_model() -> ModelState {
	let mut model = ModelState::new("auth", "User");
	model.add_field(field("id", FieldType::Integer, false));
	model.add_field(field("username", FieldType::VarChar(150), false));
	model.add_field(field("email", FieldType::VarChar(255), false));
	model
}

/// Creates a Post model with a foreign key to User
#[fixture]
fn post_model_with_fk() -> ModelState {
	let mut model = ModelState::new("blog", "Post");
	model.add_field(field("id", FieldType::Integer, false));
	model.add_field(field("title", FieldType::VarChar(200), false));
	model.add_field(field(
		"author_id",
		FieldType::ForeignKey {
			to_table: "auth_user".to_string(),
			to_field: "id".to_string(),
			on_delete: ForeignKeyAction::Cascade,
		},
		false,
	));
	model
}

// =============================================================================
// Phase 1: Happy Path Tests - Relation Field Detection
// =============================================================================

mod happy_path_relation_fields {
	use super::*;

	#[rstest]
	fn test_detect_foreign_key_field_addition(empty_state: ProjectState, user_model: ModelState) {
		// Arrange: Create from_state with User model only
		let mut from_state = empty_state.clone();
		from_state.add_model(user_model.clone());

		// Create to_state with User and Post (with FK)
		let mut to_state = from_state.clone();
		let mut post_model = ModelState::new("blog", "Post");
		post_model.add_field(field("id", FieldType::Integer, false));
		post_model.add_field(field("title", FieldType::VarChar(200), false));
		post_model.add_field(field(
			"author_id",
			FieldType::ForeignKey {
				to_table: "auth_user".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(post_model);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.created_models.len(),
			1,
			"Should detect one new model"
		);
		assert_eq!(changes.created_models[0].0, "blog");
		assert_eq!(changes.created_models[0].1, "Post");

		// Verify FK dependency is detected
		let deps = changes
			.model_dependencies
			.get(&("blog".to_string(), "Post".to_string()));
		assert!(
			deps.is_some(),
			"Post should have dependency on User via ForeignKey"
		);
	}

	#[rstest]
	fn test_detect_foreign_key_field_removal() {
		// Arrange: Post model with FK
		let mut from_state = ProjectState::new();
		let mut user = ModelState::new("auth", "User");
		user.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(user.clone());

		let mut post_with_fk = ModelState::new("blog", "Post");
		post_with_fk.add_field(field("id", FieldType::Integer, false));
		post_with_fk.add_field(field("title", FieldType::VarChar(200), false));
		post_with_fk.add_field(field(
			"author_id",
			FieldType::ForeignKey {
				to_table: "auth_user".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		from_state.add_model(post_with_fk);

		// To state: Post without FK
		let mut to_state = ProjectState::new();
		to_state.add_model(user);

		let mut post_without_fk = ModelState::new("blog", "Post");
		post_without_fk.add_field(field("id", FieldType::Integer, false));
		post_without_fk.add_field(field("title", FieldType::VarChar(200), false));
		to_state.add_model(post_without_fk);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.removed_fields.len(),
			1,
			"Should detect FK field removal"
		);
		assert_eq!(changes.removed_fields[0].0, "blog");
		assert_eq!(changes.removed_fields[0].1, "Post");
		assert_eq!(changes.removed_fields[0].2, "author_id");
	}

	#[rstest]
	fn test_detect_foreign_key_target_change() {
		// Arrange: Post with FK to User
		let mut from_state = ProjectState::new();

		let mut user = ModelState::new("auth", "User");
		user.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(user.clone());

		let mut admin = ModelState::new("auth", "Admin");
		admin.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(admin.clone());

		let mut post = ModelState::new("blog", "Post");
		post.add_field(field("id", FieldType::Integer, false));
		post.add_field(field(
			"author_id",
			FieldType::ForeignKey {
				to_table: "auth_user".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		from_state.add_model(post);

		// To state: Post with FK to Admin (changed target)
		let mut to_state = ProjectState::new();
		to_state.add_model(user);
		to_state.add_model(admin);

		let mut post_new = ModelState::new("blog", "Post");
		post_new.add_field(field("id", FieldType::Integer, false));
		post_new.add_field(field(
			"author_id",
			FieldType::ForeignKey {
				to_table: "auth_admin".to_string(), // Changed target
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(post_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect FK target change as field alteration"
		);
		assert_eq!(changes.altered_fields[0].0, "blog");
		assert_eq!(changes.altered_fields[0].1, "Post");
		assert_eq!(changes.altered_fields[0].2, "author_id");
	}

	#[rstest]
	fn test_detect_one_to_one_field_addition() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut user = ModelState::new("auth", "User");
		user.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(user.clone());

		let mut to_state = from_state.clone();
		let mut profile = ModelState::new("auth", "Profile");
		profile.add_field(field("id", FieldType::Integer, false));
		profile.add_field(field(
			"user",
			FieldType::OneToOne {
				to: "auth.User".to_string(),
				on_delete: ForeignKeyAction::Cascade,
				on_update: ForeignKeyAction::NoAction,
			},
			false,
		));
		to_state.add_model(profile);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.created_models.len(),
			1,
			"Should detect Profile model creation"
		);
		assert_eq!(changes.created_models[0].0, "auth");
		assert_eq!(changes.created_models[0].1, "Profile");
	}

	#[rstest]
	fn test_detect_one_to_one_field_removal() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut user = ModelState::new("auth", "User");
		user.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(user.clone());

		let mut profile = ModelState::new("auth", "Profile");
		profile.add_field(field("id", FieldType::Integer, false));
		profile.add_field(field(
			"user",
			FieldType::OneToOne {
				to: "auth.User".to_string(),
				on_delete: ForeignKeyAction::Cascade,
				on_update: ForeignKeyAction::NoAction,
			},
			false,
		));
		from_state.add_model(profile);

		// To state: Profile without OneToOne field
		let mut to_state = ProjectState::new();
		to_state.add_model(user);

		let mut profile_new = ModelState::new("auth", "Profile");
		profile_new.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(profile_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.removed_fields.len(),
			1,
			"Should detect OneToOne field removal"
		);
		assert_eq!(changes.removed_fields[0].2, "user");
	}

	#[rstest]
	fn test_detect_many_to_many_field_addition() {
		// Arrange
		let mut from_state = ProjectState::new();

		let mut tag = ModelState::new("blog", "Tag");
		tag.add_field(field("id", FieldType::Integer, false));
		tag.add_field(field("name", FieldType::VarChar(50), false));
		from_state.add_model(tag.clone());

		let mut post = ModelState::new("blog", "Post");
		post.add_field(field("id", FieldType::Integer, false));
		post.add_field(field("title", FieldType::VarChar(200), false));
		from_state.add_model(post);

		// To state: Post with M2M to Tag
		let mut to_state = ProjectState::new();
		to_state.add_model(tag);

		let mut post_with_m2m = ModelState::new("blog", "Post");
		post_with_m2m.add_field(field("id", FieldType::Integer, false));
		post_with_m2m.add_field(field("title", FieldType::VarChar(200), false));
		post_with_m2m.add_field(field(
			"tags",
			FieldType::ManyToMany {
				to: "blog.Tag".to_string(),
				through: None, // Auto-generated intermediate table
			},
			false,
		));
		to_state.add_model(post_with_m2m);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should detect M2M field addition"
		);
		assert_eq!(changes.added_fields[0].0, "blog");
		assert_eq!(changes.added_fields[0].1, "Post");
		assert_eq!(changes.added_fields[0].2, "tags");
	}

	#[rstest]
	fn test_detect_many_to_many_field_removal() {
		// Arrange
		let mut from_state = ProjectState::new();

		let mut tag = ModelState::new("blog", "Tag");
		tag.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(tag.clone());

		let mut post = ModelState::new("blog", "Post");
		post.add_field(field("id", FieldType::Integer, false));
		post.add_field(field(
			"tags",
			FieldType::ManyToMany {
				to: "blog.Tag".to_string(),
				through: None,
			},
			false,
		));
		from_state.add_model(post);

		// To state: Post without M2M field
		let mut to_state = ProjectState::new();
		to_state.add_model(tag);

		let mut post_without_m2m = ModelState::new("blog", "Post");
		post_without_m2m.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(post_without_m2m);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.removed_fields.len(),
			1,
			"Should detect M2M field removal"
		);
		assert_eq!(changes.removed_fields[0].2, "tags");
	}

	#[rstest]
	fn test_detect_many_to_many_through_change() {
		// Arrange: Post with auto-generated M2M table
		let mut from_state = ProjectState::new();

		let mut tag = ModelState::new("blog", "Tag");
		tag.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(tag.clone());

		let mut post = ModelState::new("blog", "Post");
		post.add_field(field("id", FieldType::Integer, false));
		post.add_field(field(
			"tags",
			FieldType::ManyToMany {
				to: "blog.Tag".to_string(),
				through: None, // Auto-generated
			},
			false,
		));
		from_state.add_model(post);

		// To state: Post with explicit through table
		let mut to_state = ProjectState::new();
		to_state.add_model(tag);

		let mut post_with_through = ModelState::new("blog", "Post");
		post_with_through.add_field(field("id", FieldType::Integer, false));
		post_with_through.add_field(field(
			"tags",
			FieldType::ManyToMany {
				to: "blog.Tag".to_string(),
				through: Some("blog_post_tags".to_string()), // Explicit table
			},
			false,
		));
		to_state.add_model(post_with_through);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect M2M through table change"
		);
		assert_eq!(changes.altered_fields[0].2, "tags");
	}
}

// =============================================================================
// Phase 1: Happy Path Tests - PostgreSQL-Specific Types
// =============================================================================

mod happy_path_postgres_types {
	use super::*;

	#[rstest]
	fn test_detect_json_to_jsonb_change() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Config");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("data", FieldType::Json, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Config");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("data", FieldType::JsonBinary, false)); // Changed to JSONB
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect JSON to JSONB change"
		);
		assert_eq!(changes.altered_fields[0].0, "app");
		assert_eq!(changes.altered_fields[0].1, "Config");
		assert_eq!(changes.altered_fields[0].2, "data");
	}

	#[rstest]
	fn test_detect_array_field_addition() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Product");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Product");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field(
			"tags",
			FieldType::Array(Box::new(FieldType::VarChar(50))),
			false,
		));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should detect array field addition"
		);
		assert_eq!(changes.added_fields[0].2, "tags");
	}

	#[rstest]
	fn test_detect_array_element_type_change() {
		// Arrange: Array of VarChar(50)
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Product");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field(
			"tags",
			FieldType::Array(Box::new(FieldType::VarChar(50))),
			false,
		));
		from_state.add_model(model);

		// To state: Array of Text (element type changed)
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Product");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field(
			"tags",
			FieldType::Array(Box::new(FieldType::Text)),
			false,
		));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect array element type change"
		);
		assert_eq!(changes.altered_fields[0].2, "tags");
	}

	#[rstest]
	fn test_detect_hstore_field_changes() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Product");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Product");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("metadata", FieldType::HStore, false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should detect HStore field addition"
		);
		assert_eq!(changes.added_fields[0].2, "metadata");
	}

	#[rstest]
	fn test_detect_uuid_field_changes() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Entity");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Entity");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("uuid", FieldType::Uuid, false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should detect UUID field addition"
		);
		assert_eq!(changes.added_fields[0].2, "uuid");
	}

	#[rstest]
	#[case(FieldType::Int4Range)]
	#[case(FieldType::Int8Range)]
	#[case(FieldType::NumRange)]
	#[case(FieldType::DateRange)]
	#[case(FieldType::TsRange)]
	#[case(FieldType::TsTzRange)]
	fn test_detect_range_type_addition(#[case] range_type: FieldType) {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Booking");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Booking");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("period", range_type.clone(), false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should detect range type field addition for {:?}",
			range_type
		);
		assert_eq!(changes.added_fields[0].2, "period");
	}

	#[rstest]
	fn test_detect_tsvector_field_changes() {
		// Arrange
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("search", "Document");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("content", FieldType::Text, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("search", "Document");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("content", FieldType::Text, false));
		model_new.add_field(field("search_vector", FieldType::TsVector, false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should detect TsVector field addition"
		);
		assert_eq!(changes.added_fields[0].2, "search_vector");
	}
}

// =============================================================================
// Phase 1: Happy Path Tests - Complex Field Type Changes
// =============================================================================

mod happy_path_complex_types {
	use super::*;

	#[rstest]
	fn test_detect_decimal_precision_change() {
		// Arrange: Decimal(10, 2)
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("finance", "Transaction");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field(
			"amount",
			FieldType::Decimal {
				precision: 10,
				scale: 2,
			},
			false,
		));
		from_state.add_model(model);

		// To state: Decimal(18, 2) - increased precision
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("finance", "Transaction");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field(
			"amount",
			FieldType::Decimal {
				precision: 18,
				scale: 2,
			},
			false,
		));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect decimal precision change"
		);
		assert_eq!(changes.altered_fields[0].2, "amount");
	}

	#[rstest]
	fn test_detect_decimal_scale_change() {
		// Arrange: Decimal(10, 2)
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("finance", "Transaction");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field(
			"amount",
			FieldType::Decimal {
				precision: 10,
				scale: 2,
			},
			false,
		));
		from_state.add_model(model);

		// To state: Decimal(10, 4) - increased scale
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("finance", "Transaction");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field(
			"amount",
			FieldType::Decimal {
				precision: 10,
				scale: 4,
			},
			false,
		));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect decimal scale change"
		);
		assert_eq!(changes.altered_fields[0].2, "amount");
	}

	#[rstest]
	fn test_detect_varchar_length_increase() {
		// Arrange: VarChar(100)
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "User");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("name", FieldType::VarChar(100), false));
		from_state.add_model(model);

		// To state: VarChar(200)
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "User");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("name", FieldType::VarChar(200), false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect varchar length increase"
		);
		assert_eq!(changes.altered_fields[0].2, "name");
	}

	#[rstest]
	fn test_detect_varchar_length_decrease() {
		// Arrange: VarChar(200)
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "User");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("name", FieldType::VarChar(200), false));
		from_state.add_model(model);

		// To state: VarChar(100) - decreased
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "User");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("name", FieldType::VarChar(100), false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect varchar length decrease"
		);
		assert_eq!(changes.altered_fields[0].2, "name");
	}

	#[rstest]
	fn test_detect_enum_values_change() {
		// Arrange: Enum with initial values
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Task");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field(
			"status",
			FieldType::Enum {
				values: vec!["pending".to_string(), "done".to_string()],
			},
			false,
		));
		from_state.add_model(model);

		// To state: Enum with additional value
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Task");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field(
			"status",
			FieldType::Enum {
				values: vec![
					"pending".to_string(),
					"in_progress".to_string(), // New value
					"done".to_string(),
				],
			},
			false,
		));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect enum values change"
		);
		assert_eq!(changes.altered_fields[0].2, "status");
	}

	#[rstest]
	fn test_detect_char_to_varchar_change() {
		// Arrange: Char(10)
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Code");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("code", FieldType::Char(10), false));
		from_state.add_model(model);

		// To state: VarChar(10)
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Code");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("code", FieldType::VarChar(10), false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.altered_fields.len(),
			1,
			"Should detect Char to VarChar change"
		);
		assert_eq!(changes.altered_fields[0].2, "code");
	}
}

// =============================================================================
// Phase 1: Happy Path Tests - Multiple Model Operations
// =============================================================================

mod happy_path_multiple_models {
	use super::*;

	#[rstest]
	fn test_detect_multiple_models_created() {
		// Arrange: Empty state
		let from_state = ProjectState::new();

		// To state: 5 new models
		let mut to_state = ProjectState::new();
		for i in 1..=5 {
			let mut model = ModelState::new("app", &format!("Model{}", i));
			model.add_field(field("id", FieldType::Integer, false));
			model.add_field(field("name", FieldType::VarChar(100), false));
			to_state.add_model(model);
		}

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.created_models.len(),
			5,
			"Should detect all 5 model creations"
		);
	}

	#[rstest]
	fn test_detect_multiple_models_deleted() {
		// Arrange: 5 models
		let mut from_state = ProjectState::new();
		for i in 1..=5 {
			let mut model = ModelState::new("app", &format!("Model{}", i));
			model.add_field(field("id", FieldType::Integer, false));
			from_state.add_model(model);
		}

		// To state: Empty
		let to_state = ProjectState::new();

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.deleted_models.len(),
			5,
			"Should detect all 5 model deletions"
		);
	}

	#[rstest]
	fn test_detect_models_with_dependencies() {
		// Arrange: Empty state
		let from_state = ProjectState::new();

		// To state: Models with FK chain: A <- B <- C
		let mut to_state = ProjectState::new();

		let mut model_a = ModelState::new("app", "ModelA");
		model_a.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model_a);

		let mut model_b = ModelState::new("app", "ModelB");
		model_b.add_field(field("id", FieldType::Integer, false));
		model_b.add_field(field(
			"a_id",
			FieldType::ForeignKey {
				to_table: "app_modela".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(model_b);

		let mut model_c = ModelState::new("app", "ModelC");
		model_c.add_field(field("id", FieldType::Integer, false));
		model_c.add_field(field(
			"b_id",
			FieldType::ForeignKey {
				to_table: "app_modelb".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(model_c);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.created_models.len(),
			3,
			"Should detect all 3 model creations"
		);

		// Verify dependencies are detected
		let b_deps = changes
			.model_dependencies
			.get(&("app".to_string(), "ModelB".to_string()));
		assert!(b_deps.is_some(), "ModelB should have dependencies");

		let c_deps = changes
			.model_dependencies
			.get(&("app".to_string(), "ModelC".to_string()));
		assert!(c_deps.is_some(), "ModelC should have dependencies");
	}

	#[rstest]
	fn test_operations_ordered_by_dependency() {
		// Arrange: Empty state
		let from_state = ProjectState::new();

		// To state: Models with FK dependencies
		let mut to_state = ProjectState::new();

		// Add in reverse order (C first, then B, then A)
		let mut model_c = ModelState::new("app", "ModelC");
		model_c.add_field(field("id", FieldType::Integer, false));
		model_c.add_field(field(
			"b_id",
			FieldType::ForeignKey {
				to_table: "app_modelb".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(model_c);

		let mut model_b = ModelState::new("app", "ModelB");
		model_b.add_field(field("id", FieldType::Integer, false));
		model_b.add_field(field(
			"a_id",
			FieldType::ForeignKey {
				to_table: "app_modela".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(model_b);

		let mut model_a = ModelState::new("app", "ModelA");
		model_a.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model_a);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Get ordered models
		let ordered = changes.order_models_by_dependency();

		// Assert: A should come before B, B should come before C
		let a_pos = ordered
			.iter()
			.position(|m| m.1 == "ModelA")
			.expect("ModelA should be in ordered list");
		let b_pos = ordered
			.iter()
			.position(|m| m.1 == "ModelB")
			.expect("ModelB should be in ordered list");
		let c_pos = ordered
			.iter()
			.position(|m| m.1 == "ModelC")
			.expect("ModelC should be in ordered list");

		assert!(
			a_pos < b_pos,
			"ModelA should come before ModelB (A at {}, B at {})",
			a_pos,
			b_pos
		);
		assert!(
			b_pos < c_pos,
			"ModelB should come before ModelC (B at {}, C at {})",
			b_pos,
			c_pos
		);
	}
}

// =============================================================================
// Phase 2: Error Path Tests
// =============================================================================

mod error_path {
	use super::*;

	#[rstest]
	fn test_similarity_config_threshold_too_low() {
		// Attempt to create config with threshold < 0.45
		let result = SimilarityConfig::new(0.4, 0.8);

		assert!(result.is_err(), "Should reject model threshold below 0.45");
	}

	#[rstest]
	fn test_similarity_config_threshold_too_high() {
		// Attempt to create config with threshold > 0.95
		let result = SimilarityConfig::new(0.96, 0.8);

		assert!(result.is_err(), "Should reject model threshold above 0.95");
	}

	#[rstest]
	fn test_similarity_config_weights_dont_sum() {
		// Attempt to create config with weights not summing to 1.0
		let result = SimilarityConfig::with_weights(0.7, 0.8, 0.6, 0.6);

		assert!(result.is_err(), "Should reject weights not summing to 1.0");
	}

	#[rstest]
	fn test_similarity_config_negative_weights() {
		// Attempt to create config with negative weights
		let result = SimilarityConfig::with_weights(0.7, 0.8, -0.2, 1.2);

		assert!(result.is_err(), "Should reject negative weights");
	}

	#[rstest]
	fn test_circular_dependency_self_reference() {
		// Create a model with FK to itself
		let mut changes = DetectedChanges::default();

		let mut deps = BTreeMap::new();
		deps.insert(
			("app".to_string(), "Node".to_string()),
			vec![("app".to_string(), "Node".to_string())], // Self-reference
		);
		changes.model_dependencies = deps;

		let result = changes.check_circular_dependencies();
		assert!(result.is_err(), "Should detect self-referential dependency");
	}

	#[rstest]
	fn test_circular_dependency_two_models() {
		// A -> B -> A cycle
		let mut changes = DetectedChanges::default();

		let mut deps = BTreeMap::new();
		deps.insert(
			("app".to_string(), "A".to_string()),
			vec![("app".to_string(), "B".to_string())],
		);
		deps.insert(
			("app".to_string(), "B".to_string()),
			vec![("app".to_string(), "A".to_string())],
		);
		changes.model_dependencies = deps;

		let result = changes.check_circular_dependencies();
		assert!(result.is_err(), "Should detect A -> B -> A cycle");
	}

	#[rstest]
	fn test_circular_dependency_three_models() {
		// A -> B -> C -> A cycle
		let mut changes = DetectedChanges::default();

		let mut deps = BTreeMap::new();
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
		assert!(result.is_err(), "Should detect A -> B -> C -> A cycle");
	}
}

// =============================================================================
// Phase 3: Edge Cases
// =============================================================================

mod edge_cases {
	use super::*;

	#[rstest]
	fn test_both_states_empty() {
		let from_state = ProjectState::new();
		let to_state = ProjectState::new();

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert!(
			changes.created_models.is_empty(),
			"No models should be created"
		);
		assert!(
			changes.deleted_models.is_empty(),
			"No models should be deleted"
		);
		assert!(changes.added_fields.is_empty(), "No fields should be added");
		assert!(
			changes.removed_fields.is_empty(),
			"No fields should be removed"
		);
		assert!(
			changes.altered_fields.is_empty(),
			"No fields should be altered"
		);
	}

	#[rstest]
	fn test_from_state_empty_single_model() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "Single");
		model.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1);
		assert_eq!(changes.created_models[0].1, "Single");
	}

	#[rstest]
	fn test_to_state_empty_single_model() {
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Single");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		let to_state = ProjectState::new();

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.deleted_models.len(), 1);
		assert_eq!(changes.deleted_models[0].1, "Single");
	}

	#[rstest]
	fn test_model_with_single_field() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "Minimal");
		model.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1);
	}

	#[rstest]
	fn test_model_with_50_fields() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "Large");
		model.add_field(field("id", FieldType::Integer, false));
		for i in 1..=50 {
			model.add_field(field(
				&format!("field_{}", i),
				FieldType::VarChar(100),
				false,
			));
		}
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1);
		assert_eq!(changes.created_models[0].1, "Large");
	}

	#[rstest]
	fn test_project_with_100_models() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		for i in 1..=100 {
			let mut model = ModelState::new("app", &format!("Model{}", i));
			model.add_field(field("id", FieldType::Integer, false));
			to_state.add_model(model);
		}

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			100,
			"Should detect all 100 model creations"
		);
	}

	#[rstest]
	fn test_same_model_name_different_apps() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();

		let mut user1 = ModelState::new("app1", "User");
		user1.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(user1);

		let mut user2 = ModelState::new("app2", "User");
		user2.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(user2);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			2,
			"Should detect both User models in different apps"
		);
	}

	#[rstest]
	fn test_field_name_with_underscore() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "Timestamp");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("created_at", FieldType::DateTime, false));
		model.add_field(field("updated_at", FieldType::DateTime, false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1);
	}

	#[rstest]
	fn test_app_label_with_underscore() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("user_management", "Profile");
		model.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1);
		assert_eq!(changes.created_models[0].0, "user_management");
	}
}

// =============================================================================
// Phase 4: State Transition Tests
// =============================================================================

mod state_transitions {
	use super::*;

	#[rstest]
	fn test_detect_changes_multiple_times() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "Test");
		model.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state.clone(), to_state.clone());

		// Call detect_changes multiple times
		let changes1 = detector.detect_changes();
		let changes2 = detector.detect_changes();

		// Results should be identical
		assert_eq!(
			changes1.created_models.len(),
			changes2.created_models.len(),
			"Multiple calls should return same results"
		);
		assert_eq!(changes1.created_models, changes2.created_models);
	}

	#[rstest]
	fn test_incremental_model_building() {
		// Test building model state incrementally
		let from_state = ProjectState::new();

		// Build model incrementally
		let mut model = ModelState::new("app", "Incremental");
		model.add_field(field("id", FieldType::Integer, false));

		// Add more fields
		model.add_field(field("name", FieldType::VarChar(100), false));
		model.add_field(field("email", FieldType::VarChar(255), true));

		let mut to_state = ProjectState::new();
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1);
	}
}

// =============================================================================
// Phase 5: Use Case Tests
// =============================================================================

mod use_cases {
	use super::*;

	#[rstest]
	fn test_usecase_blog_system_evolution() {
		// Initial state: Just posts
		let mut from_state = ProjectState::new();
		let mut post = ModelState::new("blog", "Post");
		post.add_field(field("id", FieldType::Integer, false));
		post.add_field(field("title", FieldType::VarChar(200), false));
		post.add_field(field("content", FieldType::Text, false));
		from_state.add_model(post);

		// Evolved state: Posts with comments and tags
		let mut to_state = ProjectState::new();

		// Post with author FK
		let mut post_new = ModelState::new("blog", "Post");
		post_new.add_field(field("id", FieldType::Integer, false));
		post_new.add_field(field("title", FieldType::VarChar(200), false));
		post_new.add_field(field("content", FieldType::Text, false));
		post_new.add_field(field("created_at", FieldType::DateTime, false)); // New field
		to_state.add_model(post_new);

		// Comment (new model)
		let mut comment = ModelState::new("blog", "Comment");
		comment.add_field(field("id", FieldType::Integer, false));
		comment.add_field(field("text", FieldType::Text, false));
		comment.add_field(field(
			"post_id",
			FieldType::ForeignKey {
				to_table: "blog_post".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(comment);

		// Tag (new model)
		let mut tag = ModelState::new("blog", "Tag");
		tag.add_field(field("id", FieldType::Integer, false));
		tag.add_field(field("name", FieldType::VarChar(50), false));
		to_state.add_model(tag);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.created_models.len(),
			2,
			"Should create Comment and Tag models"
		);
		assert_eq!(
			changes.added_fields.len(),
			1,
			"Should add created_at to Post"
		);
		assert_eq!(changes.added_fields[0].2, "created_at");
	}

	#[rstest]
	fn test_usecase_auth_system() {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();

		// User model
		let mut user = ModelState::new("auth", "User");
		user.add_field(field("id", FieldType::Integer, false));
		user.add_field(field("username", FieldType::VarChar(150), false));
		user.add_field(field("email", FieldType::VarChar(255), false));
		user.add_field(field("password_hash", FieldType::VarChar(128), false));
		user.add_field(field("is_active", FieldType::Boolean, false));
		to_state.add_model(user);

		// Role model
		let mut role = ModelState::new("auth", "Role");
		role.add_field(field("id", FieldType::Integer, false));
		role.add_field(field("name", FieldType::VarChar(50), false));
		to_state.add_model(role);

		// Permission model
		let mut permission = ModelState::new("auth", "Permission");
		permission.add_field(field("id", FieldType::Integer, false));
		permission.add_field(field("codename", FieldType::VarChar(100), false));
		permission.add_field(field("name", FieldType::VarChar(255), false));
		to_state.add_model(permission);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.created_models.len(),
			3,
			"Should create User, Role, and Permission"
		);
	}

	#[rstest]
	fn test_refactor_add_timestamps() {
		// Common refactoring: adding created_at/updated_at to models
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Article");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("title", FieldType::VarChar(200), false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Article");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("title", FieldType::VarChar(200), false));
		model_new.add_field(field("created_at", FieldType::DateTime, false));
		model_new.add_field(field("updated_at", FieldType::DateTime, false));
		to_state.add_model(model_new);

		// Act
		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Assert
		assert_eq!(
			changes.added_fields.len(),
			2,
			"Should add created_at and updated_at"
		);
	}
}

// =============================================================================
// Phase 7: Property-Based Tests
// =============================================================================

mod property_based {
	use super::*;

	#[rstest]
	fn test_prop_detect_changes_deterministic() {
		// Same input should always produce same output
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Test");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Test");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("name", FieldType::VarChar(100), false));
		to_state.add_model(model_new);

		// Run detection multiple times
		let mut results = Vec::new();
		for _ in 0..5 {
			let detector = MigrationAutodetector::new(from_state.clone(), to_state.clone());
			results.push(detector.detect_changes());
		}

		// All results should be identical
		for (i, result) in results.iter().enumerate().skip(1) {
			assert_eq!(
				result.added_fields.len(),
				results[0].added_fields.len(),
				"Run {} should have same added_fields count",
				i
			);
			assert_eq!(
				result.added_fields, results[0].added_fields,
				"Run {} should have identical added_fields",
				i
			);
		}
	}

	#[rstest]
	fn test_prop_no_changes_for_identical_states() {
		let mut state = ProjectState::new();
		let mut model = ModelState::new("app", "Identical");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("name", FieldType::VarChar(100), false));
		state.add_model(model);

		let detector = MigrationAutodetector::new(state.clone(), state.clone());
		let changes = detector.detect_changes();

		assert!(
			changes.created_models.is_empty(),
			"Identical states should produce no created models"
		);
		assert!(
			changes.deleted_models.is_empty(),
			"Identical states should produce no deleted models"
		);
		assert!(
			changes.added_fields.is_empty(),
			"Identical states should produce no added fields"
		);
		assert!(
			changes.removed_fields.is_empty(),
			"Identical states should produce no removed fields"
		);
		assert!(
			changes.altered_fields.is_empty(),
			"Identical states should produce no altered fields"
		);
	}

	#[rstest]
	fn test_prop_field_order_independent() {
		// Create two states with fields in different order
		let from_state = ProjectState::new();

		let mut to_state1 = ProjectState::new();
		let mut model1 = ModelState::new("app", "OrderTest");
		model1.add_field(field("id", FieldType::Integer, false));
		model1.add_field(field("a", FieldType::VarChar(10), false));
		model1.add_field(field("b", FieldType::VarChar(10), false));
		model1.add_field(field("c", FieldType::VarChar(10), false));
		to_state1.add_model(model1);

		let mut to_state2 = ProjectState::new();
		let mut model2 = ModelState::new("app", "OrderTest");
		// Add fields in reverse order
		model2.add_field(field("c", FieldType::VarChar(10), false));
		model2.add_field(field("b", FieldType::VarChar(10), false));
		model2.add_field(field("a", FieldType::VarChar(10), false));
		model2.add_field(field("id", FieldType::Integer, false));
		to_state2.add_model(model2);

		let detector1 = MigrationAutodetector::new(from_state.clone(), to_state1);
		let detector2 = MigrationAutodetector::new(from_state, to_state2);

		let changes1 = detector1.detect_changes();
		let changes2 = detector2.detect_changes();

		assert_eq!(
			changes1.created_models.len(),
			changes2.created_models.len(),
			"Field order should not affect detection"
		);
	}

	#[rstest]
	fn test_prop_model_order_independent() {
		let from_state = ProjectState::new();

		// Add models in one order
		let mut to_state1 = ProjectState::new();
		for name in ["Alpha", "Beta", "Gamma"] {
			let mut model = ModelState::new("app", name);
			model.add_field(field("id", FieldType::Integer, false));
			to_state1.add_model(model);
		}

		// Add models in reverse order
		let mut to_state2 = ProjectState::new();
		for name in ["Gamma", "Beta", "Alpha"] {
			let mut model = ModelState::new("app", name);
			model.add_field(field("id", FieldType::Integer, false));
			to_state2.add_model(model);
		}

		let detector1 = MigrationAutodetector::new(from_state.clone(), to_state1);
		let detector2 = MigrationAutodetector::new(from_state, to_state2);

		let changes1 = detector1.detect_changes();
		let changes2 = detector2.detect_changes();

		assert_eq!(
			changes1.created_models.len(),
			changes2.created_models.len(),
			"Model order should not affect count"
		);

		// Sort to compare
		let mut names1: Vec<_> = changes1.created_models.iter().map(|m| &m.1).collect();
		let mut names2: Vec<_> = changes2.created_models.iter().map(|m| &m.1).collect();
		names1.sort();
		names2.sort();

		assert_eq!(names1, names2, "Model order should not affect content");
	}
}

// =============================================================================
// Phase 8: Combination Tests
// =============================================================================

mod combination_tests {
	use super::*;

	#[rstest]
	fn test_combo_create_and_delete_model() {
		// From: Model A exists
		let mut from_state = ProjectState::new();
		let mut model_a = ModelState::new("app", "ModelA");
		model_a.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model_a);

		// To: Model A deleted, Model B created
		let mut to_state = ProjectState::new();
		let mut model_b = ModelState::new("app", "ModelB");
		model_b.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(model_b);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.created_models.len(), 1, "Should create ModelB");
		assert_eq!(changes.deleted_models.len(), 1, "Should delete ModelA");
		assert_eq!(changes.created_models[0].1, "ModelB");
		assert_eq!(changes.deleted_models[0].1, "ModelA");
	}

	#[rstest]
	fn test_combo_add_remove_alter_fields() {
		// From: Model with fields a, b
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Multi");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("a", FieldType::VarChar(50), false));
		model.add_field(field("b", FieldType::VarChar(100), false));
		from_state.add_model(model);

		// To: field a removed, field b altered, field c added
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Multi");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("b", FieldType::VarChar(200), false)); // Altered
		model_new.add_field(field("c", FieldType::Text, false)); // Added
		to_state.add_model(model_new);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.added_fields.len(), 1, "Should add field c");
		assert_eq!(changes.removed_fields.len(), 1, "Should remove field a");
		assert_eq!(changes.altered_fields.len(), 1, "Should alter field b");
	}

	#[rstest]
	fn test_combo_add_field_and_index() {
		// From: Model without index
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Indexed");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		// To: Model with new field and index on that field
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Indexed");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field("email", FieldType::VarChar(255), false));
		model_new.indexes.push(IndexDefinition {
			name: "idx_email".to_string(),
			fields: vec!["email".to_string()],
			unique: true,
		});
		to_state.add_model(model_new);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.added_fields.len(), 1, "Should add email field");
		assert_eq!(changes.added_indexes.len(), 1, "Should add email index");
	}

	#[rstest]
	fn test_combo_add_field_and_constraint() {
		// From: Model without constraint
		let mut from_state = ProjectState::new();
		let mut model = ModelState::new("app", "Constrained");
		model.add_field(field("id", FieldType::Integer, false));
		from_state.add_model(model);

		// To: Model with new field and check constraint
		let mut to_state = ProjectState::new();
		let mut model_new = ModelState::new("app", "Constrained");
		model_new.add_field(field("id", FieldType::Integer, false));
		model_new.add_field(field(
			"price",
			FieldType::Decimal {
				precision: 10,
				scale: 2,
			},
			false,
		));
		model_new.constraints.push(ConstraintDefinition {
			name: "chk_price_positive".to_string(),
			constraint_type: "check".to_string(),
			fields: vec!["price".to_string()],
			expression: Some("price > 0".to_string()),
			foreign_key_info: None,
		});
		to_state.add_model(model_new);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(changes.added_fields.len(), 1, "Should add price field");
		assert_eq!(
			changes.added_constraints.len(),
			1,
			"Should add check constraint"
		);
	}

	#[rstest]
	fn test_combo_add_fk_and_target_model() {
		// From: Empty
		let from_state = ProjectState::new();

		// To: Create target model and model with FK in one step
		let mut to_state = ProjectState::new();

		let mut user = ModelState::new("auth", "User");
		user.add_field(field("id", FieldType::Integer, false));
		to_state.add_model(user);

		let mut post = ModelState::new("blog", "Post");
		post.add_field(field("id", FieldType::Integer, false));
		post.add_field(field(
			"author_id",
			FieldType::ForeignKey {
				to_table: "auth_user".to_string(),
				to_field: "id".to_string(),
				on_delete: ForeignKeyAction::Cascade,
			},
			false,
		));
		to_state.add_model(post);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			2,
			"Should create both User and Post"
		);

		// Verify dependency ordering
		let ordered = changes.order_models_by_dependency();
		let user_pos = ordered.iter().position(|m| m.1 == "User");
		let post_pos = ordered.iter().position(|m| m.1 == "Post");

		assert!(
			user_pos.is_some() && post_pos.is_some(),
			"Both models should be in ordered list"
		);
		assert!(
			user_pos.unwrap() < post_pos.unwrap(),
			"User should come before Post due to FK"
		);
	}
}

// =============================================================================
// Phase 9: Sanity Tests
// =============================================================================

mod sanity_tests {
	use super::*;

	#[rstest]
	fn test_sanity_autodetector_new() {
		let from = ProjectState::new();
		let to = ProjectState::new();
		let _detector = MigrationAutodetector::new(from, to);
		// If we get here without panic, the test passes
	}

	#[rstest]
	fn test_sanity_project_state_new() {
		let state = ProjectState::new();
		assert!(state.models.is_empty(), "New ProjectState should be empty");
	}

	#[rstest]
	fn test_sanity_model_state_new() {
		let model = ModelState::new("app", "Model");
		assert_eq!(model.app_label, "app");
		assert_eq!(model.name, "Model");
		assert!(
			model.fields.is_empty(),
			"New ModelState should have no fields"
		);
	}

	#[rstest]
	fn test_sanity_field_state_new() {
		let field = FieldState::new("test".to_string(), FieldType::Integer, false);
		assert_eq!(field.name, "test");
		assert_eq!(field.field_type, FieldType::Integer);
		assert!(!field.nullable);
	}

	#[rstest]
	fn test_sanity_detected_changes_default() {
		let changes = DetectedChanges::default();
		assert!(changes.created_models.is_empty());
		assert!(changes.deleted_models.is_empty());
		assert!(changes.added_fields.is_empty());
		assert!(changes.removed_fields.is_empty());
		assert!(changes.altered_fields.is_empty());
	}
}

// =============================================================================
// Phase 10: Equivalence Partitioning - Using rstest #[case]
// =============================================================================

mod equivalence_partitioning {
	use super::*;

	/// Test integer type equivalence class
	#[rstest]
	#[case(FieldType::Integer, "Integer")]
	#[case(FieldType::BigInteger, "BigInteger")]
	#[case(FieldType::SmallInteger, "SmallInteger")]
	#[case(FieldType::TinyInt, "TinyInt")]
	fn test_integer_type_detection(#[case] field_type: FieldType, #[case] type_name: &str) {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "IntTest");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("value", field_type.clone(), false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			1,
			"Should detect model with {} type",
			type_name
		);
	}

	/// Test string type equivalence class
	#[rstest]
	#[case(FieldType::VarChar(100), "VarChar")]
	#[case(FieldType::Text, "Text")]
	#[case(FieldType::Char(10), "Char")]
	fn test_string_type_detection(#[case] field_type: FieldType, #[case] type_name: &str) {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "StrTest");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("content", field_type.clone(), false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			1,
			"Should detect model with {} type",
			type_name
		);
	}

	/// Test datetime type equivalence class
	#[rstest]
	#[case(FieldType::Date, "Date")]
	#[case(FieldType::DateTime, "DateTime")]
	#[case(FieldType::TimestampTz, "TimestampTz")]
	#[case(FieldType::Time, "Time")]
	fn test_datetime_type_detection(#[case] field_type: FieldType, #[case] type_name: &str) {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "DateTest");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("timestamp", field_type.clone(), false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			1,
			"Should detect model with {} type",
			type_name
		);
	}
}

// =============================================================================
// Phase 11: Boundary Value Analysis - Using rstest #[case]
// =============================================================================

mod boundary_value_analysis {
	use super::*;

	/// Test VarChar length boundaries
	#[rstest]
	#[case(1, "minimum length")]
	#[case(255, "common maximum")]
	#[case(256, "above common max")]
	#[case(65535, "maximum for MySQL")]
	fn test_boundary_varchar_length(#[case] length: u32, #[case] desc: &str) {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "LengthTest");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field("value", FieldType::VarChar(length), false));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			1,
			"Should handle VarChar({}) - {}",
			length,
			desc
		);
	}

	/// Test Decimal precision boundaries
	#[rstest]
	#[case(1, 0, "minimum precision")]
	#[case(10, 2, "common precision")]
	#[case(38, 10, "maximum for PostgreSQL")]
	#[case(65, 30, "maximum for MySQL")]
	fn test_boundary_decimal_precision(
		#[case] precision: u32,
		#[case] scale: u32,
		#[case] desc: &str,
	) {
		let from_state = ProjectState::new();

		let mut to_state = ProjectState::new();
		let mut model = ModelState::new("app", "DecimalTest");
		model.add_field(field("id", FieldType::Integer, false));
		model.add_field(field(
			"value",
			FieldType::Decimal { precision, scale },
			false,
		));
		to_state.add_model(model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		assert_eq!(
			changes.created_models.len(),
			1,
			"Should handle Decimal({}, {}) - {}",
			precision,
			scale,
			desc
		);
	}

	/// Test similarity threshold boundaries
	#[rstest]
	#[case(0.45, 0.80, "minimum valid threshold")]
	#[case(0.70, 0.80, "default threshold")]
	#[case(0.95, 0.95, "maximum valid threshold")]
	fn test_boundary_similarity_threshold(
		#[case] model_threshold: f64,
		#[case] field_threshold: f64,
		#[case] desc: &str,
	) {
		let result = SimilarityConfig::new(model_threshold, field_threshold);

		assert!(
			result.is_ok(),
			"Should accept thresholds ({}, {}) - {}",
			model_threshold,
			field_threshold,
			desc
		);

		let config = result.unwrap();
		assert_eq!(config.model_threshold(), model_threshold);
		assert_eq!(config.field_threshold(), field_threshold);
	}
}

// =============================================================================
// Phase 12: Decision Table Testing - Using rstest #[case]
// =============================================================================

mod decision_table {
	use super::*;

	/// Decision table for model similarity decisions
	/// | Same App | Same Name | High Similarity | Expected Result |
	#[rstest]
	#[case(true, true, true, "no_change")]
	#[case(true, false, true, "rename_or_delete_create")]
	#[case(true, false, false, "delete_create")]
	#[case(false, true, true, "move_or_delete_create")]
	#[case(false, true, false, "delete_create")]
	#[case(false, false, false, "delete_create")]
	fn test_decision_model_similarity(
		#[case] same_app: bool,
		#[case] same_name: bool,
		#[case] high_similarity: bool,
		#[case] expected: &str,
	) {
		let mut from_state = ProjectState::new();

		let from_app = if same_app { "app" } else { "app1" };
		let from_name = "TestModel";
		let to_app = "app";
		let to_name = if same_name {
			"TestModel"
		} else {
			"RenamedModel"
		};

		// Create model in from_state
		let mut from_model = ModelState::new(from_app, from_name);
		from_model.add_field(field("id", FieldType::Integer, false));
		from_model.add_field(field("name", FieldType::VarChar(100), false));
		if high_similarity {
			from_model.add_field(field("email", FieldType::VarChar(255), false));
		} else {
			from_model.add_field(field("old_field", FieldType::Text, false));
		}
		from_state.add_model(from_model);

		// Create model in to_state
		let mut to_state = ProjectState::new();
		let mut to_model = ModelState::new(to_app, to_name);
		to_model.add_field(field("id", FieldType::Integer, false));
		to_model.add_field(field("name", FieldType::VarChar(100), false));
		if high_similarity {
			to_model.add_field(field("email", FieldType::VarChar(255), false));
		} else {
			to_model.add_field(field("new_field", FieldType::Boolean, false));
		}
		to_state.add_model(to_model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		// Verify expected outcome based on decision table
		match expected {
			"no_change" => {
				assert!(changes.created_models.is_empty());
				assert!(changes.deleted_models.is_empty());
				assert!(changes.renamed_models.is_empty());
			}
			"rename_or_delete_create" => {
				// Either rename detected or delete+create
				let has_rename = !changes.renamed_models.is_empty();
				let has_delete_create =
					!changes.deleted_models.is_empty() && !changes.created_models.is_empty();
				assert!(
					has_rename || has_delete_create,
					"Expected rename or delete+create"
				);
			}
			"move_or_delete_create" => {
				let has_move = !changes.moved_models.is_empty();
				let has_delete_create =
					!changes.deleted_models.is_empty() && !changes.created_models.is_empty();
				assert!(
					has_move || has_delete_create,
					"Expected move or delete+create"
				);
			}
			"delete_create" => {
				assert!(
					!changes.deleted_models.is_empty() || !changes.created_models.is_empty(),
					"Expected delete+create pattern"
				);
			}
			_ => panic!("Unknown expected result: {}", expected),
		}
	}

	/// Decision table for field change detection
	/// | Same Name | Same Type | Same Nullable | Expected |
	#[rstest]
	#[case(true, true, true, "no_change")]
	#[case(true, true, false, "alter")]
	#[case(true, false, true, "alter")]
	#[case(true, false, false, "alter")]
	#[case(false, true, true, "add_remove_or_rename")]
	#[case(false, true, false, "add_remove")]
	#[case(false, false, true, "add_remove")]
	#[case(false, false, false, "add_remove")]
	fn test_decision_field_change(
		#[case] same_name: bool,
		#[case] same_type: bool,
		#[case] same_nullable: bool,
		#[case] expected: &str,
	) {
		let mut from_state = ProjectState::new();

		let from_field_name = "test_field";
		let to_field_name = if same_name {
			"test_field"
		} else {
			"renamed_field"
		};
		let from_type = FieldType::VarChar(100);
		let to_type = if same_type {
			FieldType::VarChar(100)
		} else {
			FieldType::Text
		};
		let from_nullable = false;
		let to_nullable = if same_nullable { false } else { true };

		let mut from_model = ModelState::new("app", "Model");
		from_model.add_field(field("id", FieldType::Integer, false));
		from_model.add_field(FieldState::new(
			from_field_name.to_string(),
			from_type,
			from_nullable,
		));
		from_state.add_model(from_model);

		let mut to_state = ProjectState::new();
		let mut to_model = ModelState::new("app", "Model");
		to_model.add_field(field("id", FieldType::Integer, false));
		to_model.add_field(FieldState::new(
			to_field_name.to_string(),
			to_type,
			to_nullable,
		));
		to_state.add_model(to_model);

		let detector = MigrationAutodetector::new(from_state, to_state);
		let changes = detector.detect_changes();

		match expected {
			"no_change" => {
				assert!(changes.added_fields.is_empty());
				assert!(changes.removed_fields.is_empty());
				assert!(changes.altered_fields.is_empty());
				assert!(changes.renamed_fields.is_empty());
			}
			"alter" => {
				assert_eq!(
					changes.altered_fields.len(),
					1,
					"Expected one altered field"
				);
			}
			"add_remove_or_rename" => {
				let has_rename = !changes.renamed_fields.is_empty();
				let has_add_remove =
					!changes.added_fields.is_empty() && !changes.removed_fields.is_empty();
				assert!(
					has_rename || has_add_remove,
					"Expected rename or add+remove"
				);
			}
			"add_remove" => {
				assert!(
					!changes.added_fields.is_empty() || !changes.removed_fields.is_empty(),
					"Expected add and/or remove"
				);
			}
			_ => panic!("Unknown expected result: {}", expected),
		}
	}
}

// =============================================================================
// Proptest-based Fuzz Testing
// =============================================================================

mod proptest_fuzz {
	use super::*;

	proptest! {
		/// Fuzz test: Any ProjectState comparison should not panic
		#[test]
		fn test_fuzz_no_panic_on_random_states(
			field_count in 1usize..10,
		) {
			let mut from_state = ProjectState::new();
			let mut to_state = ProjectState::new();

			// Create random model in from_state
			let mut from_model = ModelState::new("app", "FuzzModel");
			from_model.add_field(field("id", FieldType::Integer, false));
			for i in 0..field_count {
				from_model.add_field(field(
					&format!("field_{}", i),
					FieldType::VarChar(100),
					false,
				));
			}
			from_state.add_model(from_model);

			// Create slightly different model in to_state
			let mut to_model = ModelState::new("app", "FuzzModel");
			to_model.add_field(field("id", FieldType::Integer, false));
			for i in 0..field_count.saturating_sub(1) {
				to_model.add_field(field(
					&format!("field_{}", i),
					FieldType::VarChar(100),
					false,
				));
			}
			to_model.add_field(field("new_field", FieldType::Text, false));
			to_state.add_model(to_model);

			// This should not panic
			let detector = MigrationAutodetector::new(from_state, to_state);
			let _changes = detector.detect_changes();
		}

		/// Fuzz test: Random number of models
		#[test]
		fn test_fuzz_random_model_count(model_count in 1usize..20) {
			let from_state = ProjectState::new();
			let mut to_state = ProjectState::new();

			for i in 0..model_count {
				let mut model = ModelState::new("app", &format!("Model{}", i));
				model.add_field(field("id", FieldType::Integer, false));
				to_state.add_model(model);
			}

			let detector = MigrationAutodetector::new(from_state, to_state);
			let changes = detector.detect_changes();

			prop_assert_eq!(
				changes.created_models.len(),
				model_count,
				"Should detect all {} models",
				model_count
			);
		}
	}
}
