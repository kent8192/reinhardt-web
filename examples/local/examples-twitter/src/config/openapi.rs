//! OpenAPI schema generation for examples-twitter
//!
//! This module provides the OpenAPI 3.0 schema generation for the Twitter API example.

use reinhardt::rest::openapi::{
	OpenApiSchema, RedocUI, Schema, SchemaExt, SchemaGenerator, SwaggerUI,
};
use std::sync::{Arc, LazyLock};

/// Global Swagger UI instance
pub static SWAGGER_UI: LazyLock<Arc<SwaggerUI>> = LazyLock::new(|| {
	let schema = build_openapi_schema();
	Arc::new(SwaggerUI::new(schema.clone()))
});

/// Global Redoc UI instance
pub static REDOC_UI: LazyLock<Arc<RedocUI>> = LazyLock::new(|| {
	let schema = build_openapi_schema();
	Arc::new(RedocUI::new(schema))
});

/// Build the complete OpenAPI schema for the Twitter API
///
/// Note: This function is currently not used when running via `cargo run --bin manage runserver`.
/// The runserver command uses `OpenApiRouter::wrap()` which calls `generate_openapi_schema()`
/// from `reinhardt-openapi/src/endpoints.rs` instead.
///
/// This function is kept for potential future use or manual schema generation.
pub fn build_openapi_schema() -> OpenApiSchema {
	let mut generator = SchemaGenerator::new()
		.title("Twitter Example API")
		.version("1.0.0")
		.description("A Twitter-like API built with Reinhardt Framework")
		.add_server_fn_endpoints(); // Auto-discover server function endpoints from #[server_fn]

	// Register schemas for all apps
	register_auth_schemas(&mut generator);
	register_profile_schemas(&mut generator);
	register_relationship_schemas(&mut generator);
	register_dm_schemas(&mut generator);

	generator
		.generate()
		.expect("Failed to generate OpenAPI schema")
}

/// Register Auth-related schemas
fn register_auth_schemas(generator: &mut SchemaGenerator) {
	let registry = generator.registry();

	// RegisterRequest schema
	registry.register(
		"RegisterRequest",
		Schema::object_with_description(
			vec![
				("username", Schema::string()),
				("email", Schema::string()),
				("password", Schema::string()),
			],
			vec!["username", "email", "password"],
			"User registration request data",
		),
	);

	// RegisterResponse schema
	registry.register(
		"RegisterResponse",
		Schema::object_with_description(
			vec![
				("id", Schema::string()),
				("username", Schema::string()),
				("email", Schema::string()),
			],
			vec!["id", "username", "email"],
			"User registration response data",
		),
	);

	// SignInRequest schema
	registry.register(
		"SignInRequest",
		Schema::object_with_description(
			vec![("email", Schema::string()), ("password", Schema::string())],
			vec!["email", "password"],
			"User sign-in request data",
		),
	);

	// SignInResponse schema
	registry.register(
		"SignInResponse",
		Schema::object_with_description(
			vec![
				("access_token", Schema::string()),
				(
					"user",
					Schema::object_with_properties(
						vec![
							("id", Schema::string()),
							("username", Schema::string()),
							("email", Schema::string()),
						],
						vec!["id", "username", "email"],
					),
				),
			],
			vec!["access_token", "user"],
			"User sign-in response with JWT token",
		),
	);
}

/// Register Profile-related schemas
fn register_profile_schemas(generator: &mut SchemaGenerator) {
	let registry = generator.registry();

	// Profile schema
	registry.register(
		"Profile",
		Schema::object_with_description(
			vec![
				("user_id", Schema::string()),
				("bio", Schema::string()),
				("location", Schema::string()),
				("avatar_url", Schema::string()),
				("created_at", Schema::datetime()),
			],
			vec!["user_id"],
			"User profile data",
		),
	);

	// CreateProfileRequest schema
	registry.register(
		"CreateProfileRequest",
		Schema::object_with_description(
			vec![
				("bio", Schema::string()),
				("location", Schema::string()),
				("avatar_url", Schema::string()),
			],
			Vec::<&str>::new(),
			"Profile creation request data",
		),
	);

	// PatchProfileRequest schema
	registry.register(
		"PatchProfileRequest",
		Schema::object_with_description(
			vec![
				("bio", Schema::string()),
				("location", Schema::string()),
				("avatar_url", Schema::string()),
			],
			Vec::<&str>::new(),
			"Profile update request data (all fields optional)",
		),
	);
}

/// Register Relationship-related schemas
fn register_relationship_schemas(generator: &mut SchemaGenerator) {
	let registry = generator.registry();

	// FollowersList schema
	registry.register(
		"FollowersList",
		Schema::object_with_description(
			vec![
				(
					"followers",
					Schema::array(Schema::object_with_properties(
						vec![
							("user_id", Schema::string()),
							("username", Schema::string()),
							("created_at", Schema::datetime()),
						],
						vec!["user_id", "username"],
					)),
				),
				("count", Schema::integer()),
			],
			vec!["followers", "count"],
			"List of followers",
		),
	);

	// FollowingsList schema
	registry.register(
		"FollowingsList",
		Schema::object_with_description(
			vec![
				(
					"followings",
					Schema::array(Schema::object_with_properties(
						vec![
							("user_id", Schema::string()),
							("username", Schema::string()),
							("created_at", Schema::datetime()),
						],
						vec!["user_id", "username"],
					)),
				),
				("count", Schema::integer()),
			],
			vec!["followings", "count"],
			"List of users being followed",
		),
	);

	// BlockingsList schema
	registry.register(
		"BlockingsList",
		Schema::object_with_description(
			vec![
				(
					"blockings",
					Schema::array(Schema::object_with_properties(
						vec![
							("user_id", Schema::string()),
							("username", Schema::string()),
							("created_at", Schema::datetime()),
						],
						vec!["user_id", "username"],
					)),
				),
				("count", Schema::integer()),
			],
			vec!["blockings", "count"],
			"List of blocked users",
		),
	);
}

/// Register DM (Direct Message)-related schemas
fn register_dm_schemas(generator: &mut SchemaGenerator) {
	let registry = generator.registry();

	// Room schema
	registry.register(
		"Room",
		Schema::object_with_description(
			vec![
				("id", Schema::string()),
				("participants", Schema::array(Schema::string())),
				("created_at", Schema::datetime()),
				("updated_at", Schema::datetime()),
			],
			vec!["id", "participants", "created_at"],
			"Direct message room",
		),
	);

	// Message schema
	registry.register(
		"Message",
		Schema::object_with_description(
			vec![
				("id", Schema::string()),
				("room_id", Schema::string()),
				("sender_id", Schema::string()),
				("content", Schema::string()),
				("created_at", Schema::datetime()),
			],
			vec!["id", "room_id", "sender_id", "content", "created_at"],
			"Direct message",
		),
	);

	// CreateRoomRequest schema
	registry.register(
		"CreateRoomRequest",
		Schema::object_with_description(
			vec![("participants", Schema::array(Schema::string()))],
			vec!["participants"],
			"Room creation request data",
		),
	);

	// SendMessageRequest schema
	registry.register(
		"SendMessageRequest",
		Schema::object_with_description(
			vec![("content", Schema::string())],
			vec!["content"],
			"Message sending request data",
		),
	);

	// RoomsList schema
	registry.register(
		"RoomsList",
		Schema::object_with_description(
			vec![
				(
					"rooms",
					Schema::array(Schema::object_with_properties(
						vec![
							("id", Schema::string()),
							("participants", Schema::array(Schema::string())),
							("created_at", Schema::datetime()),
							("updated_at", Schema::datetime()),
						],
						vec!["id", "participants", "created_at"],
					)),
				),
				("count", Schema::integer()),
			],
			vec!["rooms", "count"],
			"List of direct message rooms",
		),
	);

	// MessagesList schema
	registry.register(
		"MessagesList",
		Schema::object_with_description(
			vec![
				(
					"messages",
					Schema::array(Schema::object_with_properties(
						vec![
							("id", Schema::string()),
							("room_id", Schema::string()),
							("sender_id", Schema::string()),
							("content", Schema::string()),
							("created_at", Schema::datetime()),
						],
						vec!["id", "room_id", "sender_id", "content", "created_at"],
					)),
				),
				("count", Schema::integer()),
			],
			vec!["messages", "count"],
			"List of messages in a room",
		),
	);
}
