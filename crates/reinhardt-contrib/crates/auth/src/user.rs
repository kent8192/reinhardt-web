use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User trait - compose authentication features
///
/// This trait defines the core interface for user authentication and authorization
/// in Reinhardt applications. It provides methods to identify users and check their
/// authentication status and permissions.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::user::{User, SimpleUser};
/// use uuid::Uuid;
///
/// let user = SimpleUser {
///     id: Uuid::new_v4(),
///     username: "alice".to_string(),
///     email: "alice@example.com".to_string(),
///     is_active: true,
///     is_admin: false,
/// };
///
/// assert_eq!(user.username(), "alice");
/// assert_eq!(user.get_username(), "alice");
/// assert!(user.is_authenticated());
/// assert!(user.is_active());
/// assert!(!user.is_admin());
/// ```
pub trait User: Send + Sync {
    /// Returns the unique identifier for this user.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::user::{User, SimpleUser};
    /// use uuid::Uuid;
    ///
    /// let user_id = Uuid::new_v4();
    /// let user = SimpleUser {
    ///     id: user_id,
    ///     username: "bob".to_string(),
    ///     email: "bob@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// };
    ///
    /// assert_eq!(user.id(), user_id.to_string());
    /// ```
    fn id(&self) -> String;

    /// Returns the username of this user.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::user::{User, SimpleUser};
    /// use uuid::Uuid;
    ///
    /// let user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "charlie".to_string(),
    ///     email: "charlie@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// };
    ///
    /// assert_eq!(user.username(), "charlie");
    /// ```
    fn username(&self) -> &str;

    /// Returns the username of this user (alias for `username()`).
    ///
    /// This method provides Django-compatible naming for retrieving usernames.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::user::{User, SimpleUser};
    /// use uuid::Uuid;
    ///
    /// let user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "diana".to_string(),
    ///     email: "diana@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// };
    ///
    /// assert_eq!(user.get_username(), user.username());
    /// assert_eq!(user.get_username(), "diana");
    /// ```
    fn get_username(&self) -> &str {
        self.username()
    }

    /// Returns whether this user is authenticated.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::user::{User, SimpleUser, AnonymousUser};
    /// use uuid::Uuid;
    ///
    /// let authenticated_user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "eve".to_string(),
    ///     email: "eve@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// };
    ///
    /// let anonymous_user = AnonymousUser;
    ///
    /// assert!(authenticated_user.is_authenticated());
    /// assert!(!anonymous_user.is_authenticated());
    /// ```
    fn is_authenticated(&self) -> bool;

    /// Returns whether this user account is active.
    ///
    /// Inactive users are typically disabled accounts that should not be allowed
    /// to access the system.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::user::{User, SimpleUser};
    /// use uuid::Uuid;
    ///
    /// let active_user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "frank".to_string(),
    ///     email: "frank@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// };
    ///
    /// let inactive_user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "grace".to_string(),
    ///     email: "grace@example.com".to_string(),
    ///     is_active: false,
    ///     is_admin: false,
    /// };
    ///
    /// assert!(active_user.is_active());
    /// assert!(!inactive_user.is_active());
    /// ```
    fn is_active(&self) -> bool;

    /// Returns whether this user has administrator privileges.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::user::{User, SimpleUser};
    /// use uuid::Uuid;
    ///
    /// let admin_user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "admin".to_string(),
    ///     email: "admin@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: true,
    /// };
    ///
    /// let regular_user = SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "henry".to_string(),
    ///     email: "henry@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// };
    ///
    /// assert!(admin_user.is_admin());
    /// assert!(!regular_user.is_admin());
    /// ```
    fn is_admin(&self) -> bool;
}

/// Simple user implementation for basic authentication scenarios.
///
/// `SimpleUser` provides a straightforward implementation of the `User` trait
/// with essential fields for user identification and authorization. It includes
/// support for serialization and deserialization via Serde.
///
/// # Fields
///
/// - `id`: Unique identifier (UUID) for the user
/// - `username`: User's login name
/// - `email`: User's email address
/// - `is_active`: Whether the user account is active
/// - `is_admin`: Whether the user has administrator privileges
///
/// # Examples
///
/// ```
/// use reinhardt_auth::user::{User, SimpleUser};
/// use uuid::Uuid;
///
/// // Create a regular user
/// let user = SimpleUser {
///     id: Uuid::new_v4(),
///     username: "john_doe".to_string(),
///     email: "john@example.com".to_string(),
///     is_active: true,
///     is_admin: false,
/// };
///
/// assert_eq!(user.username(), "john_doe");
/// assert_eq!(user.email, "john@example.com");
/// assert!(user.is_authenticated());
/// assert!(user.is_active());
/// assert!(!user.is_admin());
/// ```
///
/// # Serialization
///
/// ```
/// use reinhardt_auth::user::SimpleUser;
/// use uuid::Uuid;
/// use serde_json;
///
/// let user = SimpleUser {
///     id: Uuid::new_v4(),
///     username: "jane_smith".to_string(),
///     email: "jane@example.com".to_string(),
///     is_active: true,
///     is_admin: true,
/// };
///
/// let json = serde_json::to_string(&user).unwrap();
/// assert!(json.contains("jane_smith"));
/// assert!(json.contains("jane@example.com"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_admin: bool,
}

impl User for SimpleUser {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn is_authenticated(&self) -> bool {
        true // If we have a user object, they're authenticated
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn is_admin(&self) -> bool {
        self.is_admin
    }
}

/// Anonymous user representing an unauthenticated visitor.
///
/// `AnonymousUser` is a zero-sized type that implements the `User` trait
/// for representing users who are not logged in. This follows Django's pattern
/// of having a consistent user interface for both authenticated and
/// unauthenticated users.
///
/// All authentication and authorization checks return `false`, and the user
/// has no identifier or username.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::user::{User, AnonymousUser};
///
/// let anon = AnonymousUser;
///
/// assert_eq!(anon.id(), "");
/// assert_eq!(anon.username(), "");
/// assert!(!anon.is_authenticated());
/// assert!(!anon.is_active());
/// assert!(!anon.is_admin());
/// ```
///
/// # Comparison with authenticated users
///
/// ```
/// use reinhardt_auth::user::{User, SimpleUser, AnonymousUser};
/// use uuid::Uuid;
///
/// let authenticated = SimpleUser {
///     id: Uuid::new_v4(),
///     username: "user".to_string(),
///     email: "user@example.com".to_string(),
///     is_active: true,
///     is_admin: false,
/// };
///
/// let anonymous = AnonymousUser;
///
/// // Authenticated user has identity
/// assert!(!authenticated.id().is_empty());
/// assert_eq!(authenticated.username(), "user");
/// assert!(authenticated.is_authenticated());
///
/// // Anonymous user has no identity
/// assert!(anonymous.id().is_empty());
/// assert!(anonymous.username().is_empty());
/// assert!(!anonymous.is_authenticated());
/// ```
pub struct AnonymousUser;

impl User for AnonymousUser {
    fn id(&self) -> String {
        String::new()
    }

    fn username(&self) -> &str {
        ""
    }

    fn is_authenticated(&self) -> bool {
        false
    }

    fn is_active(&self) -> bool {
        false
    }

    fn is_admin(&self) -> bool {
        false
    }
}
