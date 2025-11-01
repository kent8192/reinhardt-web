use crate::backend::PasswordHasher;
use chrono::{DateTime, Utc};
use reinhardt_apps::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// BaseUser trait - Django's AbstractBaseUser equivalent
///
/// Provides minimal authentication functionality with automatic Argon2id password hashing by default.
/// The hashing algorithm can be customized by setting the associated type `Hasher`.
///
/// This trait represents the minimal set of fields and methods required for a user model
/// to participate in Django-style authentication. Unlike Django's class-based inheritance,
/// Reinhardt uses composition through traits.
///
/// # Default Password Hashing
///
/// All types implementing `BaseUser` automatically get Argon2id password hashing through
/// the `set_password()` and `check_password()` methods. This happens automatically without
/// any additional configuration.
///
/// # Examples
///
/// Basic usage with default Argon2id hashing:
///
/// ```
/// use reinhardt_auth::{BaseUser, Argon2Hasher};
/// use uuid::Uuid;
/// use chrono::Utc;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyUser {
///     id: Uuid,
///     email: String,
///     password_hash: Option<String>,
///     last_login: Option<chrono::DateTime<Utc>>,
///     is_active: bool,
/// }
///
/// impl BaseUser for MyUser {
///     type PrimaryKey = Uuid;
///     type Hasher = Argon2Hasher;
///
///     fn get_username_field() -> &'static str { "email" }
///     fn get_username(&self) -> &str { &self.email }
///     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
///     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
///     fn last_login(&self) -> Option<chrono::DateTime<Utc>> { self.last_login }
///     fn set_last_login(&mut self, time: chrono::DateTime<Utc>) { self.last_login = Some(time); }
///     fn is_active(&self) -> bool { self.is_active }
/// }
///
/// let mut user = MyUser {
///     id: Uuid::new_v4(),
///     email: "alice@example.com".to_string(),
///     password_hash: None,
///     last_login: None,
///     is_active: true,
/// };
///
/// // Password is automatically hashed with Argon2id
/// user.set_password("securepass123").unwrap();
/// assert!(user.check_password("securepass123").unwrap());
/// assert!(!user.check_password("wrongpass").unwrap());
/// ```
pub trait BaseUser: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// Primary key type for this user model
    type PrimaryKey: Clone + Send + Sync + Display;

    /// Password hasher type (defaults to Argon2Hasher)
    ///
    /// Override this to use a different hashing algorithm:
    ///
    /// ```ignore
    /// type Hasher = BcryptHasher;
    /// ```
    type Hasher: PasswordHasher + Default;

    // ===== Required methods =====

    /// Returns the name of the field used as the unique identifier (USERNAME_FIELD in Django)
    ///
    /// This is typically `"username"` or `"email"`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser;
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = i64;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username(&self) -> &str { "" }
    /// #     fn password_hash(&self) -> Option<&str> { None }
    /// #     fn set_password_hash(&mut self, hash: String) {}
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { None }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) {}
    /// #     fn is_active(&self) -> bool { true }
    /// fn get_username_field() -> &'static str {
    ///     "email"  // Use email as username
    /// }
    /// # }
    /// ```
    fn get_username_field() -> &'static str;

    /// Returns the username value
    ///
    /// This should return the value of the field specified by `get_username_field()`.
    fn get_username(&self) -> &str;

    /// Returns the password hash (for internal use)
    ///
    /// This should return the hashed password, not the plain text.
    /// Users should use `set_password()` to set passwords and `check_password()` to verify them.
    fn password_hash(&self) -> Option<&str>;

    /// Sets the password hash (for internal use)
    ///
    /// This is used internally by `set_password()`. Users should call `set_password()` instead.
    fn set_password_hash(&mut self, hash: String);

    /// Returns the last login timestamp
    fn last_login(&self) -> Option<DateTime<Utc>>;

    /// Sets the last login timestamp
    fn set_last_login(&mut self, time: DateTime<Utc>);

    /// Returns whether this user account is active
    ///
    /// Inactive users cannot log in.
    fn is_active(&self) -> bool;

    // ===== Default implementations (can be overridden) =====

    /// Sets the password, automatically hashing it with the configured hasher
    ///
    /// By default, uses Argon2id for hashing. Can be customized by setting the `Hasher` type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use uuid::Uuid;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser { id: Uuid, email: String, password_hash: Option<String>, last_login: Option<chrono::DateTime<chrono::Utc>>, is_active: bool }
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = Uuid;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username_field() -> &'static str { "email" }
    /// #     fn get_username(&self) -> &str { &self.email }
    /// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    /// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { self.last_login }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) { self.last_login = Some(time); }
    /// #     fn is_active(&self) -> bool { self.is_active }
    /// # }
    /// let mut user = MyUser {
    ///     id: Uuid::new_v4(),
    ///     email: "bob@example.com".to_string(),
    ///     password_hash: None,
    ///     last_login: None,
    ///     is_active: true,
    /// };
    ///
    /// user.set_password("mysecretpassword").unwrap();
    /// assert!(user.password_hash().is_some());
    /// ```
    fn set_password(&mut self, password: &str) -> Result<()> {
        let hasher = Self::Hasher::default();
        let hash = hasher.hash(password)?;
        self.set_password_hash(hash);
        Ok(())
    }

    /// Verifies a password against the stored hash
    ///
    /// Uses the configured hasher (Argon2id by default) to verify the password.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use uuid::Uuid;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser { id: Uuid, email: String, password_hash: Option<String>, last_login: Option<chrono::DateTime<chrono::Utc>>, is_active: bool }
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = Uuid;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username_field() -> &'static str { "email" }
    /// #     fn get_username(&self) -> &str { &self.email }
    /// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    /// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { self.last_login }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) { self.last_login = Some(time); }
    /// #     fn is_active(&self) -> bool { self.is_active }
    /// # }
    /// let mut user = MyUser {
    ///     id: Uuid::new_v4(),
    ///     email: "charlie@example.com".to_string(),
    ///     password_hash: None,
    ///     last_login: None,
    ///     is_active: true,
    /// };
    ///
    /// user.set_password("correctpassword").unwrap();
    /// assert!(user.check_password("correctpassword").unwrap());
    /// assert!(!user.check_password("wrongpassword").unwrap());
    /// ```
    fn check_password(&self, password: &str) -> Result<bool> {
        if let Some(hash) = self.password_hash() {
            let hasher = Self::Hasher::default();
            hasher.verify(password, hash)
        } else {
            Ok(false)
        }
    }

    /// Sets an unusable password (Django-compatible marker)
    ///
    /// After calling this, `has_usable_password()` will return `false`.
    /// This is useful for accounts that should not be able to log in with a password
    /// (e.g., OAuth-only accounts).
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use uuid::Uuid;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser { id: Uuid, email: String, password_hash: Option<String>, last_login: Option<chrono::DateTime<chrono::Utc>>, is_active: bool }
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = Uuid;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username_field() -> &'static str { "email" }
    /// #     fn get_username(&self) -> &str { &self.email }
    /// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    /// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { self.last_login }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) { self.last_login = Some(time); }
    /// #     fn is_active(&self) -> bool { self.is_active }
    /// # }
    /// let mut user = MyUser {
    ///     id: Uuid::new_v4(),
    ///     email: "oauth@example.com".to_string(),
    ///     password_hash: None,
    ///     last_login: None,
    ///     is_active: true,
    /// };
    ///
    /// user.set_unusable_password();
    /// assert!(!user.has_usable_password());
    /// ```
    fn set_unusable_password(&mut self) {
        self.set_password_hash("!".to_string());
    }

    /// Returns whether this user has a usable password
    ///
    /// Returns `false` if `set_unusable_password()` was called or if no password is set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use uuid::Uuid;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser { id: Uuid, email: String, password_hash: Option<String>, last_login: Option<chrono::DateTime<chrono::Utc>>, is_active: bool }
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = Uuid;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username_field() -> &'static str { "email" }
    /// #     fn get_username(&self) -> &str { &self.email }
    /// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    /// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { self.last_login }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) { self.last_login = Some(time); }
    /// #     fn is_active(&self) -> bool { self.is_active }
    /// # }
    /// let mut user = MyUser {
    ///     id: Uuid::new_v4(),
    ///     email: "test@example.com".to_string(),
    ///     password_hash: None,
    ///     last_login: None,
    ///     is_active: true,
    /// };
    ///
    /// assert!(!user.has_usable_password());  // No password set
    ///
    /// user.set_password("password").unwrap();
    /// assert!(user.has_usable_password());   // Password set
    ///
    /// user.set_unusable_password();
    /// assert!(!user.has_usable_password());  // Marked as unusable
    /// ```
    fn has_usable_password(&self) -> bool {
        self.password_hash()
            .map(|h| !h.starts_with('!'))
            .unwrap_or(false)
    }

    /// Returns whether this user is authenticated
    ///
    /// Always returns `true` for authenticated user objects.
    /// `AnonymousUser` returns `false`.
    fn is_authenticated(&self) -> bool {
        true
    }

    /// Returns whether this user is anonymous
    ///
    /// Always returns `false` for authenticated user objects.
    /// `AnonymousUser` returns `true`.
    fn is_anonymous(&self) -> bool {
        false
    }

    /// Returns the name of the email field (EMAIL_FIELD in Django)
    ///
    /// Defaults to `"email"`. Override if your model uses a different field name.
    fn get_email_field() -> &'static str {
        "email"
    }

    /// Normalizes a username using NFKC Unicode normalization
    ///
    /// This is the same normalization used by Django to prevent homograph attacks
    /// and ensure consistent username comparisons.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser;
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = i64;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username_field() -> &'static str { "username" }
    /// #     fn get_username(&self) -> &str { "" }
    /// #     fn password_hash(&self) -> Option<&str> { None }
    /// #     fn set_password_hash(&mut self, hash: String) {}
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { None }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) {}
    /// #     fn is_active(&self) -> bool { true }
    /// # }
    /// let normalized = MyUser::normalize_username("alice");
    /// assert_eq!(normalized, "alice");
    /// ```
    fn normalize_username(username: &str) -> String {
        use unicode_normalization::UnicodeNormalization;
        username.nfkc().collect::<String>()
    }

    /// Returns an HMAC hash for session authentication
    ///
    /// This hash is based on the password hash and username. When the password changes,
    /// this hash changes, which invalidates all sessions for this user.
    ///
    /// This is Django's `get_session_auth_hash()` equivalent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_auth::{BaseUser, Argon2Hasher};
    /// # use uuid::Uuid;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyUser { id: Uuid, email: String, password_hash: Option<String>, last_login: Option<chrono::DateTime<chrono::Utc>>, is_active: bool }
    /// # impl BaseUser for MyUser {
    /// #     type PrimaryKey = Uuid;
    /// #     type Hasher = Argon2Hasher;
    /// #     fn get_username_field() -> &'static str { "email" }
    /// #     fn get_username(&self) -> &str { &self.email }
    /// #     fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    /// #     fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    /// #     fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> { self.last_login }
    /// #     fn set_last_login(&mut self, time: chrono::DateTime<chrono::Utc>) { self.last_login = Some(time); }
    /// #     fn is_active(&self) -> bool { self.is_active }
    /// # }
    /// let mut user = MyUser {
    ///     id: Uuid::new_v4(),
    ///     email: "session@example.com".to_string(),
    ///     password_hash: None,
    ///     last_login: None,
    ///     is_active: true,
    /// };
    ///
    /// user.set_password("password1").unwrap();
    /// let hash1 = user.get_session_auth_hash();
    ///
    /// user.set_password("password2").unwrap();
    /// let hash2 = user.get_session_auth_hash();
    ///
    /// assert_ne!(hash1, hash2);  // Hash changes when password changes
    /// ```
    fn get_session_auth_hash(&self) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let key = self.password_hash().unwrap_or("");
        let mut mac = HmacSha256::new_from_slice(key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(self.get_username().as_bytes());

        format!("{:x}", mac.finalize().into_bytes())
    }
}
