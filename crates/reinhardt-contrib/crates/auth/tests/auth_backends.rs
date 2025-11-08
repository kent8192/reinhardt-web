use reinhardt_auth::{
	AnonymousUser, Argon2Hasher, AuthBackend, CompositeAuthBackend, PasswordHasher, SimpleUser,
	User,
};
use reinhardt_exception::Result;
use uuid::Uuid;

/// Mock authentication backend for testing
struct MockAuthBackend {
	users: Vec<SimpleUser>,
	hasher: Argon2Hasher,
}

impl MockAuthBackend {
	fn new() -> Self {
		Self {
			users: Vec::new(),
			hasher: Argon2Hasher::new(),
		}
	}

	fn add_user(&mut self, username: &str, _password: &str, is_active: bool, is_admin: bool) {
		self.users.push(SimpleUser {
			id: Uuid::new_v4(),
			username: username.to_string(),
			email: format!("{}@example.com", username),
			is_active,
			is_admin,
			is_staff: is_admin,  // Staff access follows admin status
			is_superuser: false, // Regular users are not superusers
		});
	}
}

#[async_trait::async_trait]
impl AuthBackend for MockAuthBackend {
	type User = SimpleUser;

	async fn authenticate(&self, username: &str, _password: &str) -> Result<Option<Self::User>> {
		for user in &self.users {
			if user.username == username {
				// Note: Mock implementation skips password verification for testing
				return Ok(Some(user.clone()));
			}
		}
		Ok(None)
	}

	async fn get_user(&self, user_id: &str) -> Result<Option<Self::User>> {
		for user in &self.users {
			if user.id.to_string() == user_id {
				return Ok(Some(user.clone()));
			}
		}
		Ok(None)
	}
}

// === Password Hasher Tests ===

#[test]
fn test_argon2_hash_password() {
	let hasher = Argon2Hasher::new();
	let password = "test_password_123";

	let hash = hasher.hash(password).unwrap();

	// Argon2 hashes start with $argon2
	assert!(hash.starts_with("$argon2"));
	assert!(!hash.is_empty());
}

#[test]
fn test_argon2_verify_correct_password() {
	let hasher = Argon2Hasher::new();
	let password = "secure_password";

	let hash = hasher.hash(password).unwrap();

	assert!(hasher.verify(password, &hash).unwrap());
}

#[test]
fn test_argon2_verify_incorrect_password() {
	let hasher = Argon2Hasher::new();
	let password = "correct_password";
	let wrong_password = "wrong_password";

	let hash = hasher.hash(password).unwrap();

	assert!(!hasher.verify(wrong_password, &hash).unwrap());
}

#[test]
fn test_argon2_different_hashes_for_same_password() {
	let hasher = Argon2Hasher::new();
	let password = "same_password";

	let hash1 = hasher.hash(password).unwrap();
	let hash2 = hasher.hash(password).unwrap();

	// Hashes should be different due to unique salts
	assert_ne!(hash1, hash2);
	// But both should verify correctly
	assert!(hasher.verify(password, &hash1).unwrap());
	assert!(hasher.verify(password, &hash2).unwrap());
}

#[test]
fn test_password_hash_bytes_support() {
	// Test that password hashing works with different input types
	let hasher = Argon2Hasher::new();
	let password = "test_password";

	let hash = hasher.hash(password).unwrap();
	assert!(hasher.verify(password, &hash).unwrap());
}

// === Authentication Backend Tests ===

#[tokio::test]
async fn test_authenticate_valid_user() {
	let mut backend = MockAuthBackend::new();
	backend.add_user("testuser", "testpass", true, false);

	let result = backend.authenticate("testuser", "testpass").await.unwrap();
	assert!(result.is_some());
	let user = result.unwrap();
	assert_eq!(user.username(), "testuser");
}

#[tokio::test]
async fn test_authenticate_invalid_username() {
	let mut backend = MockAuthBackend::new();
	backend.add_user("testuser", "testpass", true, false);

	let result = backend.authenticate("wronguser", "testpass").await.unwrap();
	assert!(result.is_none());
}

#[tokio::test]
async fn test_authenticate_inactive_user() {
	// An inactive user can still be authenticated (session created)
	// but permissions should check is_active
	let mut backend = MockAuthBackend::new();
	backend.add_user("inactive", "pass", false, false);

	let result = backend.authenticate("inactive", "pass").await.unwrap();
	assert!(result.is_some());
	let user = result.unwrap();
	assert!(!user.is_active());
}

#[tokio::test]
async fn test_auth_backends_get_user() {
	let mut backend = MockAuthBackend::new();
	backend.add_user("testuser", "testpass", true, false);

	// Get the user to obtain their ID
	let auth_result = backend.authenticate("testuser", "testpass").await.unwrap();
	let user = auth_result.unwrap();
	let user_id = user.id();

	// Retrieve user by ID
	let result = backend.get_user(&user_id).await.unwrap();
	assert!(result.is_some());
	let retrieved_user = result.unwrap();
	assert_eq!(retrieved_user.username(), "testuser");
}

#[tokio::test]
async fn test_get_user_nonexistent_id() {
	let backend = MockAuthBackend::new();

	let result = backend.get_user(&Uuid::new_v4().to_string()).await.unwrap();
	assert!(result.is_none());
}

// === Composite Backend Tests ===

#[tokio::test]
async fn test_composite_backend_multiple_backends() {
	let mut backend1 = MockAuthBackend::new();
	backend1.add_user("user1", "pass1", true, false);

	let mut backend2 = MockAuthBackend::new();
	backend2.add_user("user2", "pass2", true, false);

	let mut composite = CompositeAuthBackend::new();
	composite.add_backend(Box::new(backend1));
	composite.add_backend(Box::new(backend2));

	// Should authenticate users from first backend
	let result1 = composite.authenticate("user1", "pass1").await.unwrap();
	assert!(result1.is_some());

	// Should authenticate users from second backend
	let result2 = composite.authenticate("user2", "pass2").await.unwrap();
	assert!(result2.is_some());
}

#[tokio::test]
async fn test_composite_backend_first_match_wins() {
	let mut backend1 = MockAuthBackend::new();
	backend1.add_user("duplicate", "pass1", true, false);

	let mut backend2 = MockAuthBackend::new();
	backend2.add_user("duplicate", "pass2", true, true); // Different admin status

	let mut composite = CompositeAuthBackend::new();
	composite.add_backend(Box::new(backend1));
	composite.add_backend(Box::new(backend2));

	// Should use first backend that matches
	let result = composite.authenticate("duplicate", "pass1").await.unwrap();
	assert!(result.is_some());
	let user = result.unwrap();
	assert!(!user.is_admin()); // From first backend
}

// === User Model Tests ===

#[test]
fn test_simple_user_properties() {
	let user = SimpleUser {
		id: Uuid::new_v4(),
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		is_active: true,
		is_admin: false,
		is_staff: false,
		is_superuser: false,
	};

	assert!(!user.id().is_empty());
	assert_eq!(user.username(), "testuser");
	assert_eq!(user.get_username(), "testuser"); // Test alias method
	assert!(user.is_authenticated());
	assert!(user.is_active());
	assert!(!user.is_admin());
}

#[test]
fn test_simple_user_admin() {
	let user = SimpleUser {
		id: Uuid::new_v4(),
		username: "admin".to_string(),
		email: "admin@example.com".to_string(),
		is_active: true,
		is_admin: true,
		is_staff: true,
		is_superuser: false,
	};

	assert!(user.is_admin());
	assert!(user.is_active());
}

#[test]
fn test_simple_user_inactive() {
	let user = SimpleUser {
		id: Uuid::new_v4(),
		username: "inactive".to_string(),
		email: "inactive@example.com".to_string(),
		is_active: false,
		is_admin: false,
		is_staff: false,
		is_superuser: false,
	};

	assert!(!user.is_active());
	assert!(user.is_authenticated()); // Still authenticated even if inactive
}

#[test]
fn test_auth_backends_anonymous_properties() {
	let user = AnonymousUser;

	assert_eq!(user.id(), "");
	assert_eq!(user.username(), "");
	assert_eq!(user.get_username(), "");
	assert!(!user.is_authenticated());
	assert!(!user.is_active());
	assert!(!user.is_admin());
}

#[test]
fn test_anonymous_user_has_no_permissions() {
	// #17903 - Anonymous users shouldn't have permissions
	let user = AnonymousUser;

	assert!(!user.is_authenticated());
	assert!(!user.is_active());
	assert!(!user.is_admin());
}

// === Superuser Tests ===

#[test]
fn test_superuser_has_all_flags() {
	// A superuser should be active and admin
	let superuser = SimpleUser {
		id: Uuid::new_v4(),
		username: "superuser".to_string(),
		email: "super@example.com".to_string(),
		is_active: true,
		is_admin: true,
		is_staff: true,
		is_superuser: true,
	};

	assert!(superuser.is_active());
	assert!(superuser.is_admin());
	assert!(superuser.is_authenticated());
}
