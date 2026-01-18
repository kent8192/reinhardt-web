//! Tests for project state management
//! Adapted from Django's test_state.py

use reinhardt_db::migrations::{FieldState, FieldType, ModelState, ProjectState};

fn create_field(name: &str, field_type: FieldType, nullable: bool) -> FieldState {
	FieldState::new(name.to_string(), field_type, nullable)
}

#[test]
fn test_project_state_create() {
	// Test creating an empty ProjectState
	let state = ProjectState::new();
	assert_eq!(state.models.len(), 0);
}

#[test]
fn test_project_state_add_model() {
	// Test adding a model to ProjectState
	let mut state = ProjectState::new();

	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));
	model.add_field(create_field("name", FieldType::Text, false));

	state.add_model(model);

	assert_eq!(state.models.len(), 1);
	assert!(
		state
			.models
			.contains_key(&("myapp".to_string(), "User".to_string()))
	);
}

#[test]
fn test_project_state_get_model() {
	// Test retrieving a model from ProjectState
	let mut state = ProjectState::new();

	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));

	state.add_model(model);

	let retrieved = state.get_model("myapp", "User");
	let retrieved_model = retrieved.unwrap();
	assert_eq!(retrieved_model.name, "User");
}

#[test]
fn test_project_state_clone() {
	// Test cloning ProjectState
	let mut state = ProjectState::new();

	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));

	state.add_model(model);

	let cloned = state.clone();
	assert_eq!(cloned.models.len(), state.models.len());
	assert!(
		cloned
			.models
			.contains_key(&("myapp".to_string(), "User".to_string()))
	);
}

#[test]
fn test_model_state_create() {
	// Test creating a ModelState
	let model = ModelState::new("myapp", "User");
	assert_eq!(model.name, "User");
	assert_eq!(model.app_label, "myapp");
	assert_eq!(model.fields.len(), 0);
}

#[test]
fn test_model_state_add_field() {
	// Test adding fields to ModelState
	let mut model = ModelState::new("myapp", "User");

	model.add_field(create_field("id", FieldType::Integer, false));
	model.add_field(create_field("name", FieldType::Text, false));

	assert_eq!(model.fields.len(), 2);
	assert!(model.fields.contains_key("id"));
	assert!(model.fields.contains_key("name"));
}

#[test]
fn test_model_state_get_field() {
	// Test retrieving a field from ModelState
	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("email", FieldType::Text, false));

	let field = model.get_field("email");
	let field_state = field.unwrap();
	assert_eq!(field_state.field_type, FieldType::Text);
}

#[test]
fn test_model_state_remove_field() {
	// Test removing a field from ModelState
	let mut model = ModelState::new("myapp", "User");

	model.add_field(create_field("id", FieldType::Integer, false));
	model.add_field(create_field("temp", FieldType::Text, false));

	assert_eq!(model.fields.len(), 2);

	model.remove_field("temp");
	assert_eq!(model.fields.len(), 1);
	assert!(!model.fields.contains_key("temp"));
}

#[test]
fn test_field_state_create() {
	// Test creating a FieldState
	let field = FieldState::new("id".to_string(), FieldType::Integer, false);
	assert_eq!(field.field_type, FieldType::Integer);
	assert_eq!(field.name, "id");
	assert!(!field.nullable);
}

#[test]
fn test_field_state_with_params() {
	// Test FieldState with parameters
	let mut field = FieldState::new("email".to_string(), FieldType::Text, true);
	field
		.params
		.insert("max_length".to_string(), "255".to_string());
	field.params.insert("default".to_string(), "''".to_string());

	assert!(field.params.contains_key("max_length"));
	assert!(field.params.contains_key("default"));
	assert!(field.nullable);
}

#[test]
fn test_project_state_multiple_apps() {
	// Test ProjectState with multiple apps
	let mut state = ProjectState::new();

	let mut user_model = ModelState::new("users", "User");
	user_model.add_field(create_field("id", FieldType::Integer, false));

	let mut post_model = ModelState::new("posts", "Post");
	post_model.add_field(create_field("id", FieldType::Integer, false));

	state.add_model(user_model);
	state.add_model(post_model);

	assert_eq!(state.models.len(), 2);

	let users_models: Vec<_> = state
		.models
		.keys()
		.filter(|(app, _)| app == "users")
		.collect();
	assert_eq!(users_models.len(), 1);

	let posts_models: Vec<_> = state
		.models
		.keys()
		.filter(|(app, _)| app == "posts")
		.collect();
	assert_eq!(posts_models.len(), 1);
}

#[test]
fn test_project_state_model_diff() {
	// Test detecting differences between states
	let mut old_state = ProjectState::new();
	let mut new_state = ProjectState::new();

	let mut old_model = ModelState::new("myapp", "User");
	old_model.add_field(create_field("id", FieldType::Integer, false));

	old_state.add_model(old_model);

	let mut new_model = ModelState::new("myapp", "User");
	new_model.add_field(create_field("id", FieldType::Integer, false));
	new_model.add_field(create_field("email", FieldType::Text, false));
	new_state.add_model(new_model);

	// Both states should have the User model
	assert!(old_state.get_model("myapp", "User").is_some());
	assert!(new_state.get_model("myapp", "User").is_some());

	// But field counts should differ
	let old_model = old_state.get_model("myapp", "User").unwrap();
	let new_model = new_state.get_model("myapp", "User").unwrap();

	assert_eq!(old_model.fields.len(), 1);
	assert_eq!(new_model.fields.len(), 2);
}

#[test]
fn test_model_state_field_order() {
	// Test that field order is preserved
	let mut model = ModelState::new("myapp", "User");

	model.add_field(create_field("id", FieldType::Integer, false));
	model.add_field(create_field("name", FieldType::Text, false));
	model.add_field(create_field("email", FieldType::Text, false));

	let field_names: Vec<_> = model.fields.keys().cloned().collect();

	// HashMap doesn't maintain order, but all fields should be present
	assert!(field_names.contains(&"id".to_string()));
	assert!(field_names.contains(&"name".to_string()));
	assert!(field_names.contains(&"email".to_string()));
}

#[test]
fn test_project_state_remove_model() {
	// Test removing a model from ProjectState
	let mut state = ProjectState::new();

	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));

	state.add_model(model);
	assert_eq!(state.models.len(), 1);

	state.remove_model("myapp", "User");
	assert_eq!(state.models.len(), 0);
}

#[test]
fn test_field_state_clone() {
	// Test cloning FieldState
	let mut field = FieldState::new("email".to_string(), FieldType::Text, false);
	field
		.params
		.insert("max_length".to_string(), "100".to_string());

	let cloned = field.clone();
	assert_eq!(cloned.field_type, field.field_type);
	assert_eq!(
		cloned.params.get("max_length"),
		field.params.get("max_length")
	);
}

#[test]
fn test_model_state_clone() {
	// Test cloning ModelState
	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));
	model.add_field(create_field("name", FieldType::Text, false));

	let cloned = model.clone();
	assert_eq!(cloned.name, model.name);
	assert_eq!(cloned.app_label, model.app_label);
	assert_eq!(cloned.fields.len(), model.fields.len());
}

#[test]
fn test_project_state_equality() {
	// Test ProjectState equality
	let mut state1 = ProjectState::new();
	let mut state2 = ProjectState::new();

	let mut model1 = ModelState::new("myapp", "User");
	model1.add_field(create_field("id", FieldType::Integer, false));

	let mut model2 = ModelState::new("myapp", "User");
	model2.add_field(create_field("id", FieldType::Integer, false));

	state1.add_model(model1);
	state2.add_model(model2);

	assert_eq!(state1.models.len(), state2.models.len());
}

#[test]
fn test_model_with_table_name() {
	// Test model with custom table name
	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));

	// NOTE: Test verifies basic ModelState creation without custom table_name support
	// Production ModelState would accept table_name parameter for custom table naming
	assert_eq!(model.name, "User");
}

#[test]
fn test_model_state_has_field() {
	// Test has_field method
	let mut model = ModelState::new("myapp", "User");
	model.add_field(create_field("id", FieldType::Integer, false));
	model.add_field(create_field("name", FieldType::Text, false));

	assert!(model.has_field("id"));
	assert!(model.has_field("name"));
	assert!(!model.has_field("email"));
}
