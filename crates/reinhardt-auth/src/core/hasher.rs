/// Password hasher trait
///
/// Implement this trait to create custom password hashing algorithms.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::PasswordHasher;
/// #[cfg(feature = "argon2-hasher")]
/// use reinhardt_auth::Argon2Hasher;
///
/// # #[cfg(feature = "argon2-hasher")]
/// # {
/// let hasher = Argon2Hasher::new();
/// let password = "my_secure_password";
///
/// // Hash the password
/// let hash = hasher.hash(password).unwrap();
///
/// // Verify the password
/// assert!(hasher.verify(password, &hash).unwrap());
/// assert!(!hasher.verify("wrong_password", &hash).unwrap());
/// # }
/// ```
pub trait PasswordHasher: Send + Sync {
	/// Hashes a password
	///
	/// # Arguments
	///
	/// * `password` - The plaintext password to hash
	///
	/// # Returns
	///
	/// The hashed password as a string, or an error if hashing fails.
	fn hash(&self, password: &str) -> Result<String, reinhardt_exception::Error>;

	/// Verifies a password against a hash
	///
	/// # Arguments
	///
	/// * `password` - The plaintext password to verify
	/// * `hash` - The hash to verify against
	///
	/// # Returns
	///
	/// `Ok(true)` if the password matches, `Ok(false)` if it doesn't,
	/// or an error if verification fails.
	fn verify(&self, password: &str, hash: &str) -> Result<bool, reinhardt_exception::Error>;
}

/// Argon2id password hasher (recommended for new applications)
///
/// This hasher uses the Argon2id algorithm, which is currently recommended
/// by OWASP for password hashing. It provides strong resistance against
/// both GPU-based and side-channel attacks.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::{PasswordHasher, Argon2Hasher};
///
/// # #[cfg(feature = "argon2-hasher")]
/// # {
/// let hasher = Argon2Hasher::new();
/// let password = "secure_password123";
///
/// // Hash a password
/// let hash = hasher.hash(password).unwrap();
/// assert!(!hash.is_empty());
///
/// // Verify the password against the hash
/// assert!(hasher.verify(password, &hash).unwrap());
///
/// // Wrong password should fail verification
/// assert!(!hasher.verify("wrong_password", &hash).unwrap());
/// # }
/// ```
#[cfg(feature = "argon2-hasher")]
pub struct Argon2Hasher;

#[cfg(feature = "argon2-hasher")]
impl Argon2Hasher {
	/// Creates a new Argon2 password hasher
	pub fn new() -> Self {
		Self
	}
}

#[cfg(feature = "argon2-hasher")]
impl Default for Argon2Hasher {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(feature = "argon2-hasher")]
impl PasswordHasher for Argon2Hasher {
	fn hash(&self, password: &str) -> Result<String, reinhardt_exception::Error> {
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
			.map_err(|e| reinhardt_exception::Error::Authentication(e.to_string()))?;

		let argon2 = Argon2::default();

		argon2
			.hash_password(password.as_bytes(), &salt)
			.map(|hash| hash.to_string())
			.map_err(|e| reinhardt_exception::Error::Authentication(e.to_string()))
	}

	fn verify(&self, password: &str, hash: &str) -> Result<bool, reinhardt_exception::Error> {
		use argon2::{
			Argon2,
			password_hash::{PasswordHash, PasswordVerifier},
		};

		let parsed_hash = PasswordHash::new(hash)
			.map_err(|e| reinhardt_exception::Error::Authentication(e.to_string()))?;

		Ok(Argon2::default()
			.verify_password(password.as_bytes(), &parsed_hash)
			.is_ok())
	}
}
