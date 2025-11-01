use crate::base_user::BaseUser;
use chrono::{DateTime, Utc};

/// FullUser trait - Django's AbstractUser equivalent
///
/// Extends `BaseUser` with additional fields commonly used in web applications:
/// username, email, first name, last name, staff status, superuser status, and join date.
///
/// This trait is equivalent to Django's `django.contrib.auth.models.AbstractUser`.
/// It provides a complete user model suitable for most applications without requiring
/// customization.
///
/// # Relationship with BaseUser
///
/// `FullUser` extends `BaseUser`, meaning any type implementing `FullUser` must also
/// implement `BaseUser`. This gives you access to all password management and authentication
/// methods from `BaseUser`.
///
/// # Examples
///
/// Implementing FullUser for a custom type:
///
/// ```
/// use reinhardt_auth::{BaseUser, FullUser, Argon2Hasher};
/// use uuid::Uuid;
/// use chrono::{DateTime, Utc};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyFullUser {
///     id: Uuid,
///     username: String,
///     email: String,
///     first_name: String,
///     last_name: String,
///     password_hash: Option<String>,
///     last_login: Option<DateTime<Utc>>,
///     is_active: bool,
///     is_staff: bool,
///     is_superuser: bool,
///     date_joined: DateTime<Utc>,
/// }
///
/// impl BaseUser for MyFullUser {
///     type PrimaryKey = Uuid;
///     type Hasher = Argon2Hasher;
///
///     fn get_username_field() -> &'static str { "username" }
///     fn get_username(&self) -> &str { &self.username }
///     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
///     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
///     fn last_login(&self) -> Option<DateTime<Utc>> { self.last_login }
///     fn set_last_login(&mut self, time: DateTime<Utc>) { self.last_login = Some(time); }
///     fn is_active(&self) -> bool { self.is_active }
/// }
///
/// impl FullUser for MyFullUser {
///     fn username(&self) -> &str { &self.username }
///     fn email(&self) -> &str { &self.email }
///     fn first_name(&self) -> &str { &self.first_name }
///     fn last_name(&self) -> &str { &self.last_name }
///     fn is_staff(&self) -> bool { self.is_staff }
///     fn is_superuser(&self) -> bool { self.is_superuser }
///     fn date_joined(&self) -> DateTime<Utc> { self.date_joined }
/// }
///
/// let user = MyFullUser {
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
/// };
///
/// assert_eq!(user.get_full_name(), "Alice Smith");
/// assert_eq!(user.get_short_name(), "Alice");
/// ```
pub trait FullUser: BaseUser {
	/// Returns the username
	///
	/// This is the unique identifier for login. Typically alphanumeric with some special characters.
	fn username(&self) -> &str;

	/// Returns the email address
	///
	/// The user's email address. May or may not be unique depending on application requirements.
	fn email(&self) -> &str;

	/// Returns the first name
	///
	/// The user's given name or first name.
	fn first_name(&self) -> &str;

	/// Returns the last name
	///
	/// The user's family name or surname.
	fn last_name(&self) -> &str;

	/// Returns whether this user has staff access
	///
	/// Staff users can access the admin interface. This is different from `is_superuser` -
	/// a staff user may have limited admin permissions, while a superuser has all permissions.
	fn is_staff(&self) -> bool;

	/// Returns whether this user has superuser privileges
	///
	/// Superusers bypass all permission checks and have access to everything.
	/// This is typically used for system administrators.
	fn is_superuser(&self) -> bool;

	/// Returns the date/time when this user account was created
	///
	/// This is set automatically when the user is created and should not be modified.
	fn date_joined(&self) -> DateTime<Utc>;

	/// Returns the user's full name
	///
	/// Combines first name and last name with a space. Trims any extra whitespace.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_auth::{BaseUser, FullUser, Argon2Hasher};
	/// # use uuid::Uuid;
	/// # use chrono::{DateTime, Utc};
	/// # use serde::{Serialize, Deserialize};
	/// # #[derive(Serialize, Deserialize)]
	/// # struct MyFullUser { id: Uuid, username: String, email: String, first_name: String,
	/// #   last_name: String, password_hash: Option<String>, last_login: Option<DateTime<Utc>>,
	/// #   is_active: bool, is_staff: bool, is_superuser: bool, date_joined: DateTime<Utc> }
	/// # impl BaseUser for MyFullUser {
	/// #     type PrimaryKey = Uuid;
	/// #     type Hasher = Argon2Hasher;
	/// #     fn get_username_field() -> &'static str { "username" }
	/// #     fn get_username(&self) -> &str { &self.username }
	/// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
	/// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
	/// #     fn last_login(&self) -> Option<DateTime<Utc>> { self.last_login }
	/// #     fn set_last_login(&mut self, time: DateTime<Utc>) { self.last_login = Some(time); }
	/// #     fn is_active(&self) -> bool { self.is_active }
	/// # }
	/// # impl FullUser for MyFullUser {
	/// #     fn username(&self) -> &str { &self.username }
	/// #     fn email(&self) -> &str { &self.email }
	/// #     fn first_name(&self) -> &str { &self.first_name }
	/// #     fn last_name(&self) -> &str { &self.last_name }
	/// #     fn is_staff(&self) -> bool { self.is_staff }
	/// #     fn is_superuser(&self) -> bool { self.is_superuser }
	/// #     fn date_joined(&self) -> DateTime<Utc> { self.date_joined }
	/// # }
	/// let user = MyFullUser {
	///     id: Uuid::new_v4(),
	///     username: "bob".to_string(),
	///     email: "bob@example.com".to_string(),
	///     first_name: "Bob".to_string(),
	///     last_name: "Johnson".to_string(),
	///     password_hash: None,
	///     last_login: None,
	///     is_active: true,
	///     is_staff: false,
	///     is_superuser: false,
	///     date_joined: Utc::now(),
	/// };
	///
	/// assert_eq!(user.get_full_name(), "Bob Johnson");
	/// ```
	fn get_full_name(&self) -> String {
		format!("{} {}", self.first_name(), self.last_name())
			.trim()
			.to_string()
	}

	/// Returns the user's short name
	///
	/// Returns the first name only. This is typically used for informal greetings.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_auth::{BaseUser, FullUser, Argon2Hasher};
	/// # use uuid::Uuid;
	/// # use chrono::{DateTime, Utc};
	/// # use serde::{Serialize, Deserialize};
	/// # #[derive(Serialize, Deserialize)]
	/// # struct MyFullUser { id: Uuid, username: String, email: String, first_name: String,
	/// #   last_name: String, password_hash: Option<String>, last_login: Option<DateTime<Utc>>,
	/// #   is_active: bool, is_staff: bool, is_superuser: bool, date_joined: DateTime<Utc> }
	/// # impl BaseUser for MyFullUser {
	/// #     type PrimaryKey = Uuid;
	/// #     type Hasher = Argon2Hasher;
	/// #     fn get_username_field() -> &'static str { "username" }
	/// #     fn get_username(&self) -> &str { &self.username }
	/// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
	/// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
	/// #     fn last_login(&self) -> Option<DateTime<Utc>> { self.last_login }
	/// #     fn set_last_login(&mut self, time: DateTime<Utc>) { self.last_login = Some(time); }
	/// #     fn is_active(&self) -> bool { self.is_active }
	/// # }
	/// # impl FullUser for MyFullUser {
	/// #     fn username(&self) -> &str { &self.username }
	/// #     fn email(&self) -> &str { &self.email }
	/// #     fn first_name(&self) -> &str { &self.first_name }
	/// #     fn last_name(&self) -> &str { &self.last_name }
	/// #     fn is_staff(&self) -> bool { self.is_staff }
	/// #     fn is_superuser(&self) -> bool { self.is_superuser }
	/// #     fn date_joined(&self) -> DateTime<Utc> { self.date_joined }
	/// # }
	/// let user = MyFullUser {
	///     id: Uuid::new_v4(),
	///     username: "charlie".to_string(),
	///     email: "charlie@example.com".to_string(),
	///     first_name: "Charlie".to_string(),
	///     last_name: "Brown".to_string(),
	///     password_hash: None,
	///     last_login: None,
	///     is_active: true,
	///     is_staff: true,
	///     is_superuser: false,
	///     date_joined: Utc::now(),
	/// };
	///
	/// assert_eq!(user.get_short_name(), "Charlie");
	/// ```
	fn get_short_name(&self) -> &str {
		self.first_name()
	}
}
