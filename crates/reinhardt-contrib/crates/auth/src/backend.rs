use crate::User;
use async_trait::async_trait;

/// Authentication backend trait for implementing custom authentication strategies.
///
/// This trait enables composition of multiple authentication backends, allowing
/// applications to support various authentication methods (database, LDAP, OAuth, etc.).
///
/// # Examples
///
/// ```
/// use reinhardt_auth::{AuthBackend, SimpleUser, Argon2Hasher, PasswordHasher};
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use uuid::Uuid;
///
// Custom authentication backend using in-memory storage
/// struct InMemoryBackend {
///     users: HashMap<String, (String, SimpleUser)>, // username -> (password_hash, user)
///     hasher: Argon2Hasher,
/// }
///
/// impl InMemoryBackend {
///     fn new() -> Self {
///         let mut users = HashMap::new();
///         let hasher = Argon2Hasher::new();
///
///         // Create a test user with hashed password
///         let user = SimpleUser {
///             id: Uuid::new_v4(),
///             username: "admin".to_string(),
///             email: "admin@example.com".to_string(),
///             is_active: true,
///             is_admin: true,
///         };
///         let hash = hasher.hash("admin_password").unwrap();
///         users.insert("admin".to_string(), (hash, user));
///
///         Self { users, hasher }
///     }
/// }
///
/// #[async_trait]
/// impl AuthBackend for InMemoryBackend {
///     type User = SimpleUser;
///
///     async fn authenticate(
///         &self,
///         username: &str,
///         password: &str,
///     ) -> reinhardt_apps::Result<Option<Self::User>> {
///         if let Some((hash, user)) = self.users.get(username) {
///             if self.hasher.verify(password, hash)? {
///                 return Ok(Some(user.clone()));
///             }
///         }
///         Ok(None)
///     }
///
///     async fn get_user(&self, user_id: &str)
///         -> reinhardt_apps::Result<Option<Self::User>> {
///         Ok(self.users.values()
///             .find(|(_, u)| u.id.to_string() == user_id)
///             .map(|(_, u)| u.clone()))
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let backend = InMemoryBackend::new();
///
///     // Successful authentication
///     let user = backend.authenticate("admin", "admin_password").await.unwrap();
///     assert!(user.is_some());
///     assert_eq!(user.as_ref().unwrap().username, "admin");
///
///     // Failed authentication with wrong password
///     let user = backend.authenticate("admin", "wrong_password").await.unwrap();
///     assert!(user.is_none());
/// }
/// ```
#[async_trait]
pub trait AuthBackend: Send + Sync {
    type User: User;

    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> reinhardt_apps::Result<Option<Self::User>>;

    async fn get_user(&self, user_id: &str) -> reinhardt_apps::Result<Option<Self::User>>;
}

/// Password hasher trait for composing different hashing algorithms.
///
/// This trait allows you to implement custom password hashing strategies
/// or use the provided `Argon2Hasher` implementation.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::{PasswordHasher, Argon2Hasher};
///
/// let hasher = Argon2Hasher::new();
///
// Hash a password
/// let password = "my_secure_password";
/// let hash = hasher.hash(password).unwrap();
/// assert!(!hash.is_empty());
///
// Verify correct password
/// assert!(hasher.verify(password, &hash).unwrap());
///
// Verify incorrect password
/// assert!(!hasher.verify("wrong_password", &hash).unwrap());
/// ```
///
/// # Implementing a Custom Hasher
///
/// ```
/// use reinhardt_auth::PasswordHasher;
/// use sha2::{Sha256, Digest};
///
// Simple SHA-256 hasher (NOT recommended for production!)
/// struct SimpleHasher;
///
/// impl PasswordHasher for SimpleHasher {
///     fn hash(&self, password: &str) -> reinhardt_apps::Result<String> {
///         let mut hasher = Sha256::new();
///         hasher.update(password.as_bytes());
///         let result = hasher.finalize();
///         Ok(format!("{:x}", result))
///     }
///
///     fn verify(&self, password: &str, hash: &str) -> reinhardt_apps::Result<bool> {
///         let computed_hash = self.hash(password)?;
///         Ok(computed_hash == hash)
///     }
/// }
///
/// let hasher = SimpleHasher;
/// let hash = hasher.hash("test123").unwrap();
/// assert!(hasher.verify("test123", &hash).unwrap());
/// assert!(!hasher.verify("wrong", &hash).unwrap());
/// ```
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> reinhardt_apps::Result<String>;
    fn verify(&self, password: &str, hash: &str) -> reinhardt_apps::Result<bool>;
}

/// Argon2id password hasher (recommended for new applications)
pub struct Argon2Hasher;

impl Argon2Hasher {
    /// Creates a new Argon2 password hasher.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::{Argon2Hasher, PasswordHasher};
    ///
    /// let hasher = Argon2Hasher::new();
    /// let password = "secure_password123";
    ///
    // Hash a password
    /// let hash = hasher.hash(password).unwrap();
    /// assert!(!hash.is_empty());
    ///
    // Verify the password against the hash
    /// assert!(hasher.verify(password, &hash).unwrap());
    ///
    // Wrong password should fail verification
    /// assert!(!hasher.verify("wrong_password", &hash).unwrap());
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordHasher for Argon2Hasher {
    fn hash(&self, password: &str) -> reinhardt_apps::Result<String> {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher as _, SaltString},
        };

        // Use rand crate's rng which provides OsRng-backed randomness
        let mut rng = rand::rng();

        // Generate salt bytes directly
        let mut salt_bytes = [0u8; 16];
        use rand::RngCore;
        rng.fill_bytes(&mut salt_bytes);

        // Create salt string from bytes
        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| reinhardt_apps::Error::Authentication(e.to_string()))?;

        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| reinhardt_apps::Error::Authentication(e.to_string()))
    }

    fn verify(&self, password: &str, hash: &str) -> reinhardt_apps::Result<bool> {
        use argon2::{
            Argon2,
            password_hash::{PasswordHash, PasswordVerifier},
        };

        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| reinhardt_apps::Error::Authentication(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

/// Composite auth backend - tries multiple backends
pub struct CompositeAuthBackend<U: User> {
    backends: Vec<Box<dyn AuthBackend<User = U>>>,
}

impl<U: User> CompositeAuthBackend<U> {
    /// Creates a new composite authentication backend with no backends.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::{CompositeAuthBackend, SimpleUser, AuthBackend, Argon2Hasher, PasswordHasher};
    /// use async_trait::async_trait;
    /// use std::collections::HashMap;
    /// use uuid::Uuid;
    ///
    // Example: Create a custom authentication backend
    /// struct InMemoryAuthBackend {
    ///     users: HashMap<String, (String, SimpleUser)>, // username -> (password_hash, user)
    ///     hasher: Argon2Hasher,
    /// }
    ///
    /// impl InMemoryAuthBackend {
    ///     fn new() -> Self {
    ///         let mut users = HashMap::new();
    ///         let hasher = Argon2Hasher::new();
    ///
    ///         // Add a test user with hashed password
    ///         let user = SimpleUser {
    ///             id: Uuid::new_v4(),
    ///             username: "alice".to_string(),
    ///             email: "alice@example.com".to_string(),
    ///             is_active: true,
    ///             is_admin: false,
    ///         };
    ///         let hash = hasher.hash("password123").unwrap();
    ///         users.insert("alice".to_string(), (hash, user));
    ///
    ///         Self { users, hasher }
    ///     }
    /// }
    ///
    /// #[async_trait]
    /// impl AuthBackend for InMemoryAuthBackend {
    ///     type User = SimpleUser;
    ///
    ///     async fn authenticate(&self, username: &str, password: &str)
    ///         -> reinhardt_apps::Result<Option<Self::User>> {
    ///         if let Some((hash, user)) = self.users.get(username) {
    ///             if self.hasher.verify(password, hash)? {
    ///                 return Ok(Some(user.clone()));
    ///             }
    ///         }
    ///         Ok(None)
    ///     }
    ///
    ///     async fn get_user(&self, user_id: &str)
    ///         -> reinhardt_apps::Result<Option<Self::User>> {
    ///         Ok(self.users.values()
    ///             .find(|(_, u)| u.id.to_string() == user_id)
    ///             .map(|(_, u)| u.clone()))
    ///     }
    /// }
    ///
    // Create a composite backend with no backends initially
    /// let backend: CompositeAuthBackend<SimpleUser> = CompositeAuthBackend::new();
    /// ```
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }
    /// Adds an authentication backend to the composite.
    ///
    /// Backends are tried in the order they are added.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::{CompositeAuthBackend, SimpleUser, AuthBackend, Argon2Hasher, PasswordHasher};
    /// use async_trait::async_trait;
    /// use std::collections::HashMap;
    /// use uuid::Uuid;
    ///
    // Define a simple in-memory auth backend
    /// struct TestAuthBackend {
    ///     users: HashMap<String, SimpleUser>,
    /// }
    ///
    /// #[async_trait]
    /// impl AuthBackend for TestAuthBackend {
    ///     type User = SimpleUser;
    ///
    ///     async fn authenticate(&self, username: &str, password: &str)
    ///         -> reinhardt_apps::Result<Option<Self::User>> {
    ///         if password == "test123" {
    ///             Ok(self.users.get(username).cloned())
    ///         } else {
    ///             Ok(None)
    ///         }
    ///     }
    ///
    ///     async fn get_user(&self, user_id: &str)
    ///         -> reinhardt_apps::Result<Option<Self::User>> {
    ///         Ok(self.users.values()
    ///             .find(|u| u.id.to_string() == user_id)
    ///             .cloned())
    ///     }
    /// }
    ///
    // Create and configure composite backend
    /// let mut composite: CompositeAuthBackend<SimpleUser> = CompositeAuthBackend::new();
    ///
    /// let mut users = HashMap::new();
    /// users.insert("bob".to_string(), SimpleUser {
    ///     id: Uuid::new_v4(),
    ///     username: "bob".to_string(),
    ///     email: "bob@example.com".to_string(),
    ///     is_active: true,
    ///     is_admin: false,
    /// });
    ///
    /// composite.add_backend(Box::new(TestAuthBackend { users }));
    /// ```
    pub fn add_backend(&mut self, backend: Box<dyn AuthBackend<User = U>>) {
        self.backends.push(backend);
    }
}

impl<U: User> Default for CompositeAuthBackend<U> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<U: User + 'static> AuthBackend for CompositeAuthBackend<U> {
    type User = U;

    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> reinhardt_apps::Result<Option<Self::User>> {
        for backend in &self.backends {
            if let Some(user) = backend.authenticate(username, password).await? {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    async fn get_user(&self, user_id: &str) -> reinhardt_apps::Result<Option<Self::User>> {
        for backend in &self.backends {
            if let Some(user) = backend.get_user(user_id).await? {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }
}
