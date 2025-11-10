//! Tests for #[derive(Schema)] macro

use reinhardt_openapi::{Schema, ToSchema};
use utoipa::openapi::{
	Deprecated,
	schema::{SchemaFormat, SchemaType, Type},
};

#[test]
fn test_simple_struct_schema_generation() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		id: i64,
		name: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			assert!(matches!(obj.schema_type, SchemaType::Type(Type::Object)));
			assert_eq!(obj.properties.len(), 2);
			assert!(obj.properties.contains_key("id"));
			assert!(obj.properties.contains_key("name"));
			assert_eq!(obj.required, vec!["id", "name"]);
		}
		_ => panic!("Expected Object schema"),
	}

	assert_eq!(User::schema_name(), Some("User".to_string()));
}

#[test]
fn test_optional_fields_not_required() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		id: i64,
		name: String,
		email: Option<String>,
		age: Option<i32>,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			// Only non-Option fields should be required
			assert_eq!(obj.required, vec!["id", "name"]);
			assert_eq!(obj.properties.len(), 4);
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_description() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		#[schema(description = "Unique user identifier")]
		id: i64,

		name: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(id_schema) = obj.properties.get("id") {
				match id_schema {
					utoipa::openapi::RefOr::T(Schema::Object(id_obj)) => {
						assert_eq!(
							id_obj.description,
							Some("Unique user identifier".to_string())
						);
					}
					_ => panic!("Expected Object schema for id field"),
				}
			} else {
				panic!("id field not found in properties");
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_example() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		#[schema(example = "42")]
		id: i64,

		#[schema(example = "John Doe")]
		name: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(id_schema) = obj.properties.get("id") {
				match id_schema {
					utoipa::openapi::RefOr::T(Schema::Object(id_obj)) => {
						assert!(id_obj.example.is_some());
					}
					_ => panic!("Expected Object schema for id field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_format() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		#[schema(format = "email")]
		email: String,

		#[schema(format = "uri")]
		website: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(email_schema) = obj.properties.get("email") {
				match email_schema {
					utoipa::openapi::RefOr::T(Schema::Object(email_obj)) => {
						assert!(matches!(
							email_obj.format,
							Some(SchemaFormat::Custom(ref s)) if s == "email"
						));
					}
					_ => panic!("Expected Object schema for email field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_read_only() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		#[schema(read_only)]
		id: i64,

		name: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(id_schema) = obj.properties.get("id") {
				match id_schema {
					utoipa::openapi::RefOr::T(Schema::Object(id_obj)) => {
						assert_eq!(id_obj.read_only, Some(true));
					}
					_ => panic!("Expected Object schema for id field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_write_only() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct CreateUser {
		name: String,

		#[schema(write_only)]
		password: String,
	}

	let schema = CreateUser::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(password_schema) = obj.properties.get("password") {
				match password_schema {
					utoipa::openapi::RefOr::T(Schema::Object(password_obj)) => {
						assert_eq!(password_obj.write_only, Some(true));
					}
					_ => panic!("Expected Object schema for password field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_deprecated() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		id: i64,
		name: String,

		#[schema(deprecated)]
		old_field: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(old_field_schema) = obj.properties.get("old_field") {
				match old_field_schema {
					utoipa::openapi::RefOr::T(Schema::Object(old_field_obj)) => {
						assert!(matches!(old_field_obj.deprecated, Some(Deprecated::True)));
					}
					_ => panic!("Expected Object schema for old_field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_numeric_constraints() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct Product {
		#[schema(minimum = 0, maximum = 100)]
		rating: i32,

		#[schema(minimum = 0)]
		price: f64,
	}

	let schema = Product::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(rating_schema) = obj.properties.get("rating") {
				match rating_schema {
					utoipa::openapi::RefOr::T(Schema::Object(rating_obj)) => {
						assert!(matches!(rating_obj.minimum, Some(_)));
						assert!(matches!(rating_obj.maximum, Some(_)));
					}
					_ => panic!("Expected Object schema for rating field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_string_length_constraints() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		#[schema(min_length = 3, max_length = 50)]
		username: String,

		#[schema(min_length = 8)]
		password: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(username_schema) = obj.properties.get("username") {
				match username_schema {
					utoipa::openapi::RefOr::T(Schema::Object(username_obj)) => {
						assert_eq!(username_obj.min_length, Some(3));
						assert_eq!(username_obj.max_length, Some(50));
					}
					_ => panic!("Expected Object schema for username field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_field_with_pattern() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		#[schema(pattern = "^[a-zA-Z0-9_]+$")]
		username: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(username_schema) = obj.properties.get("username") {
				match username_schema {
					utoipa::openapi::RefOr::T(Schema::Object(username_obj)) => {
						assert_eq!(username_obj.pattern, Some("^[a-zA-Z0-9_]+$".to_string()));
					}
					_ => panic!("Expected Object schema for username field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_doc_comments_as_description() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		/// User's unique identifier
		id: i64,

		/// Full name of the user
		name: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(id_schema) = obj.properties.get("id") {
				match id_schema {
					utoipa::openapi::RefOr::T(Schema::Object(id_obj)) => {
						assert_eq!(
							id_obj.description,
							Some("User's unique identifier".to_string())
						);
					}
					_ => panic!("Expected Object schema for id field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_explicit_description_overrides_doc_comment() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		/// This doc comment will be overridden
		#[schema(description = "Explicit description")]
		id: i64,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(id_schema) = obj.properties.get("id") {
				match id_schema {
					utoipa::openapi::RefOr::T(Schema::Object(id_obj)) => {
						assert_eq!(id_obj.description, Some("Explicit description".to_string()));
					}
					_ => panic!("Expected Object schema for id field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_combined_attributes() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		/// User's email address
		#[schema(
			format = "email",
			example = "user@example.com",
			min_length = 5,
			max_length = 255
		)]
		email: String,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			if let Some(email_schema) = obj.properties.get("email") {
				match email_schema {
					utoipa::openapi::RefOr::T(Schema::Object(email_obj)) => {
						assert_eq!(
							email_obj.description,
							Some("User's email address".to_string())
						);
						assert!(matches!(
							email_obj.format,
							Some(SchemaFormat::Custom(ref s)) if s == "email"
						));
						assert!(email_obj.example.is_some());
						assert_eq!(email_obj.min_length, Some(5));
						assert_eq!(email_obj.max_length, Some(255));
					}
					_ => panic!("Expected Object schema for email field"),
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}

#[test]
fn test_complex_struct_with_all_features() {
	#[allow(dead_code)]
	#[derive(Schema)]
	struct User {
		/// Unique identifier
		#[schema(read_only, example = "123")]
		id: i64,

		/// Username for login
		#[schema(
			min_length = 3,
			max_length = 50,
			pattern = "^[a-zA-Z0-9_]+$",
			example = "john_doe"
		)]
		username: String,

		/// User's email address
		#[schema(format = "email", example = "john@example.com")]
		email: String,

		/// User's age (optional)
		#[schema(minimum = 0, maximum = 150)]
		age: Option<i32>,

		/// Password (write-only)
		#[schema(write_only, min_length = 8)]
		password: String,

		/// Deprecated field
		#[schema(deprecated)]
		legacy_field: Option<String>,
	}

	let schema = User::schema();

	match schema {
		Schema::Object(obj) => {
			// Verify structure
			assert_eq!(obj.properties.len(), 6);

			// Only non-Option and non-write-only fields should be required
			assert!(obj.required.contains(&"id".to_string()));
			assert!(obj.required.contains(&"username".to_string()));
			assert!(obj.required.contains(&"email".to_string()));
			assert!(obj.required.contains(&"password".to_string()));

			// Verify id field (read-only)
			if let Some(id_schema) = obj.properties.get("id") {
				if let utoipa::openapi::RefOr::T(Schema::Object(id_obj)) = id_schema {
					assert_eq!(id_obj.read_only, Some(true));
					assert!(id_obj.example.is_some());
				}
			}

			// Verify username field (with pattern and length constraints)
			if let Some(username_schema) = obj.properties.get("username") {
				if let utoipa::openapi::RefOr::T(Schema::Object(username_obj)) = username_schema {
					assert_eq!(username_obj.min_length, Some(3));
					assert_eq!(username_obj.max_length, Some(50));
					assert_eq!(username_obj.pattern, Some("^[a-zA-Z0-9_]+$".to_string()));
				}
			}

			// Verify email field (with format)
			if let Some(email_schema) = obj.properties.get("email") {
				if let utoipa::openapi::RefOr::T(Schema::Object(email_obj)) = email_schema {
					assert!(matches!(
						email_obj.format,
						Some(SchemaFormat::Custom(ref s)) if s == "email"
					));
				}
			}

			// Verify password field (write-only)
			if let Some(password_schema) = obj.properties.get("password") {
				if let utoipa::openapi::RefOr::T(Schema::Object(password_obj)) = password_schema {
					assert_eq!(password_obj.write_only, Some(true));
					assert_eq!(password_obj.min_length, Some(8));
				}
			}

			// Verify deprecated field
			if let Some(legacy_schema) = obj.properties.get("legacy_field") {
				if let utoipa::openapi::RefOr::T(Schema::Object(legacy_obj)) = legacy_schema {
					assert!(matches!(legacy_obj.deprecated, Some(Deprecated::True)));
				}
			}
		}
		_ => panic!("Expected Object schema"),
	}
}
