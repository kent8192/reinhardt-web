#[cfg(feature = "argon2-hasher")]
use crate::Argon2Hasher;
use crate::{BaseUser, FullUser, PermissionsMixin, User};
use chrono::{DateTime, Utc};
use reinhardt_db::orm::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DefaultUser struct - Django's AbstractUser equivalent
///
/// A complete, ready-to-use user model that combines BaseUser, FullUser, and PermissionsMixin.
/// This is the default user model provided by Reinhardt, suitable for most applications.
///
/// # Relationship with Django
///
/// This struct is equivalent to Django's `django.contrib.auth.models.AbstractUser`.
/// It provides a full-featured user model with:
/// - Username-based authentication
/// - Email address
/// - First and last name
/// - Password hashing (Argon2id by default)
/// - Active/staff/superuser flags
/// - Timestamps (last_login, date_joined)
/// - Permissions and groups
///
/// # Database Schema
///
/// The default table name is `auth_user` with the following columns:
/// - `id` (UUID, primary key)
/// - `username` (String, unique)
/// - `email` (String)
/// - `first_name` (String)
/// - `last_name` (String)
/// - `password_hash` (Option<String>)
/// - `last_login` (Option<DateTime<Utc>>)
/// - `is_active` (bool)
/// - `is_staff` (bool)
/// - `is_superuser` (bool)
/// - `date_joined` (DateTime<Utc>)
/// - `user_permissions` (Vec<String>)
/// - `groups` (Vec<String>)
///
/// # Examples
///
/// Creating a new user with automatic Argon2id password hashing:
///
/// ```
/// use reinhardt_auth::{BaseUser, DefaultUser};
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let mut user = DefaultUser {
///     id: Uuid::new_v4(),
///     username: "alice".to_string(),
///     email: "alice@example.com".to_string(),
///     first_name: "Alice".to_string(),
///     last_name: "Smith".to_string(),
///     password_hash: None,
///     last_login: None,
///     is_active: true,
///     is_staff: false,
///     is_superuser: false,
///     date_joined: Utc::now(),
///     user_permissions: Vec::new(),
///     groups: Vec::new(),
/// };
///
/// // Password is automatically hashed with Argon2id
/// user.set_password("securepass123").unwrap();
///
/// // Verify password
/// assert!(user.check_password("securepass123").unwrap());
/// assert!(!user.check_password("wrongpass").unwrap());
/// ```
///
/// Using with permissions:
///
/// ```
/// use reinhardt_auth::{DefaultUser, PermissionsMixin};
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let mut user = DefaultUser {
///     id: Uuid::new_v4(),
///     username: "bob".to_string(),
///     email: "bob@example.com".to_string(),
///     first_name: "Bob".to_string(),
///     last_name: "Johnson".to_string(),
///     password_hash: None,
///     last_login: None,
///     is_active: true,
///     is_staff: true,
///     is_superuser: false,
///     date_joined: Utc::now(),
///     user_permissions: vec![
///         "blog.add_post".to_string(),
///         "blog.change_post".to_string(),
///     ],
///     groups: vec!["editors".to_string()],
/// };
///
/// // Check permissions
/// assert!(user.has_perm("blog.add_post"));
/// assert!(user.has_perm("blog.change_post"));
/// assert!(!user.has_perm("blog.delete_post"));
/// assert!(user.has_module_perms("blog"));
/// ```
#[cfg(feature = "argon2-hasher")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultUser {
	/// Unique identifier (primary key)
	pub id: Uuid,

	/// Username (unique, used for login)
	pub username: String,

	/// Email address
	pub email: String,

	/// First name
	pub first_name: String,

	/// Last name
	pub last_name: String,

	/// Password hash (hashed with Argon2id by default)
	pub password_hash: Option<String>,

	/// Last login timestamp
	pub last_login: Option<DateTime<Utc>>,

	/// Whether this user account is active
	pub is_active: bool,

	/// Whether this user can access the admin site
	pub is_staff: bool,

	/// Whether this user has all permissions (superuser)
	pub is_superuser: bool,

	/// When this user account was created
	pub date_joined: DateTime<Utc>,

	/// List of permissions (format: "app_label.permission_name")
	pub user_permissions: Vec<String>,

	/// List of groups this user belongs to
	pub groups: Vec<String>,
}

#[cfg(feature = "argon2-hasher")]
impl BaseUser for DefaultUser {
	type PrimaryKey = Uuid;
	type Hasher = Argon2Hasher;

	fn get_username_field() -> &'static str {
		"username"
	}

	fn get_username(&self) -> &str {
		&self.username
	}

	fn password_hash(&self) -> Option<&str> {
		self.password_hash.as_deref()
	}

	fn set_password_hash(&mut self, hash: String) {
		self.password_hash = Some(hash);
	}

	fn last_login(&self) -> Option<DateTime<Utc>> {
		self.last_login
	}

	fn set_last_login(&mut self, time: DateTime<Utc>) {
		self.last_login = Some(time);
	}

	fn is_active(&self) -> bool {
		self.is_active
	}
}

#[cfg(feature = "argon2-hasher")]
impl FullUser for DefaultUser {
	fn username(&self) -> &str {
		&self.username
	}

	fn email(&self) -> &str {
		&self.email
	}

	fn first_name(&self) -> &str {
		&self.first_name
	}

	fn last_name(&self) -> &str {
		&self.last_name
	}

	fn is_staff(&self) -> bool {
		self.is_staff
	}

	fn is_superuser(&self) -> bool {
		self.is_superuser
	}

	fn date_joined(&self) -> DateTime<Utc> {
		self.date_joined
	}
}

#[cfg(feature = "argon2-hasher")]
impl PermissionsMixin for DefaultUser {
	fn is_superuser(&self) -> bool {
		self.is_superuser
	}

	fn user_permissions(&self) -> &[String] {
		&self.user_permissions
	}

	fn groups(&self) -> &[String] {
		&self.groups
	}
}

#[cfg(feature = "argon2-hasher")]
impl Model for DefaultUser {
	type PrimaryKey = Uuid;

	fn table_name() -> &'static str {
		"auth_user"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		Some(&self.id)
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = value;
	}

	fn primary_key_field() -> &'static str {
		"id"
	}
}

#[cfg(feature = "argon2-hasher")]
impl Default for DefaultUser {
	fn default() -> Self {
		Self {
			id: Uuid::nil(),
			username: String::new(),
			email: String::new(),
			first_name: String::new(),
			last_name: String::new(),
			password_hash: None,
			last_login: None,
			is_active: true,
			is_staff: false,
			is_superuser: false,
			date_joined: Utc::now(),
			user_permissions: Vec::new(),
			groups: Vec::new(),
		}
	}
}

#[cfg(feature = "argon2-hasher")]
impl User for DefaultUser {
	fn id(&self) -> String {
		self.id.to_string()
	}

	fn username(&self) -> &str {
		&self.username
	}

	fn get_username(&self) -> &str {
		&self.username
	}

	fn is_authenticated(&self) -> bool {
		true
	}

	fn is_active(&self) -> bool {
		self.is_active
	}

	fn is_admin(&self) -> bool {
		self.is_superuser
	}

	fn is_staff(&self) -> bool {
		self.is_staff
	}

	fn is_superuser(&self) -> bool {
		self.is_superuser
	}
}
