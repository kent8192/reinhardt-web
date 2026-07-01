use std::sync::Arc;

use reinhardt_core::exception::Error;

/// Result of checking a password against a stored hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordVerification {
	/// The password did not match the stored hash.
	Invalid,
	/// The password matched and the stored hash is current.
	Valid,
	/// The password matched, but the hash should be replaced.
	ValidNeedsRehash {
		/// Hash generated with the preferred password hasher.
		updated_hash: String,
	},
}

/// Compact status for callers that only need to know whether storage changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordCheck {
	/// The password did not match the stored hash.
	Invalid,
	/// The password matched without updating the stored hash.
	Valid,
	/// The password matched and a new hash was generated.
	ValidUpdated,
}

/// Ordered password hash policy with one preferred hasher and legacy hashers.
///
/// The preferred hasher is used for new password hashes and upgrade rehashes.
/// Verification checks the preferred hasher first, then checks legacy hashers in
/// registration order. A password that matches a legacy hash returns
/// [`PasswordVerification::ValidNeedsRehash`] with a replacement hash generated
/// by the preferred hasher.
///
/// # Password hash upgrades
///
/// ```
/// use reinhardt_auth::{PasswordHashPolicy, PasswordHasher};
/// #[cfg(feature = "argon2-hasher")]
/// use reinhardt_auth::Argon2Hasher;
///
/// # #[cfg(feature = "argon2-hasher")]
/// # {
/// let policy = PasswordHashPolicy::new(Argon2Hasher::default());
/// let hash = policy.hash("secret").unwrap();
///
/// assert!(Argon2Hasher::default().identify(&hash));
/// # }
/// ```
///
/// ```
/// #[cfg(all(feature = "argon2-hasher", feature = "bcrypt-hasher"))]
/// use reinhardt_auth::{Argon2Hasher, BcryptHasher, PasswordHashPolicy};
///
/// # #[cfg(all(feature = "argon2-hasher", feature = "bcrypt-hasher"))]
/// # {
/// let policy = PasswordHashPolicy::new(BcryptHasher::default())
///     .with_legacy(Argon2Hasher::default());
/// # let _ = policy;
/// # }
/// ```
#[derive(Clone)]
pub struct PasswordHashPolicy {
	/// Hasher used for new passwords and upgrade rehashes.
	preferred: Arc<dyn PasswordHasher>,
	/// Legacy hashers checked after the preferred hasher.
	legacy: Vec<Arc<dyn PasswordHasher>>,
}

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
	fn hash(&self, password: &str) -> Result<String, reinhardt_core::exception::Error>;

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
	fn verify(&self, password: &str, hash: &str) -> Result<bool, reinhardt_core::exception::Error>;

	/// Returns the stable algorithm identifier for hashes produced by this hasher.
	fn algorithm(&self) -> Option<&'static str> {
		None
	}

	/// Returns whether this hasher recognizes the stored hash format.
	fn identify(&self, _hash: &str) -> bool {
		false
	}

	/// Returns whether a recognized hash should be upgraded after verification.
	fn must_update(&self, _hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		Ok(false)
	}
}

impl PasswordHashPolicy {
	/// Creates a policy with the preferred password hasher.
	pub fn new<H>(preferred: H) -> Self
	where
		H: PasswordHasher + 'static,
	{
		Self::from_arc(Arc::new(preferred))
	}

	/// Creates a policy from a shared preferred password hasher.
	pub fn from_arc(preferred: Arc<dyn PasswordHasher>) -> Self {
		Self {
			preferred,
			legacy: Vec::new(),
		}
	}

	/// Appends a legacy hasher to the ordered verification policy.
	pub fn with_legacy<H>(mut self, legacy: H) -> Self
	where
		H: PasswordHasher + 'static,
	{
		self.legacy.push(Arc::new(legacy));
		self
	}

	/// Returns the preferred hasher algorithm identifier.
	pub fn preferred_algorithm(&self) -> Option<&'static str> {
		self.preferred.algorithm()
	}

	/// Hashes a password using the preferred hasher.
	pub fn hash(&self, password: &str) -> Result<String, Error> {
		self.preferred.hash(password)
	}

	/// Verifies a password and returns an updated preferred hash when needed.
	pub fn verify_with_update(
		&self,
		password: &str,
		hash: &str,
	) -> Result<PasswordVerification, Error> {
		if hash.is_empty() || hash == "!" {
			return Ok(PasswordVerification::Invalid);
		}

		if self.preferred.identify(hash) {
			return self.verify_preferred(password, hash);
		}

		for legacy in &self.legacy {
			if legacy.identify(hash) {
				if !legacy.verify(password, hash)? {
					return Ok(PasswordVerification::Invalid);
				}

				return Ok(PasswordVerification::ValidNeedsRehash {
					updated_hash: self.preferred.hash(password)?,
				});
			}
		}

		if self.preferred.algorithm().is_none() {
			return if self.preferred.verify(password, hash)? {
				Ok(PasswordVerification::Valid)
			} else {
				Ok(PasswordVerification::Invalid)
			};
		}

		Err(Error::Authentication(
			"Unknown password hashing algorithm".to_string(),
		))
	}

	fn verify_preferred(&self, password: &str, hash: &str) -> Result<PasswordVerification, Error> {
		if !self.preferred.verify(password, hash)? {
			return Ok(PasswordVerification::Invalid);
		}

		if self.preferred.must_update(hash)? {
			return Ok(PasswordVerification::ValidNeedsRehash {
				updated_hash: self.preferred.hash(password)?,
			});
		}

		Ok(PasswordVerification::Valid)
	}
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
#[derive(Clone)]
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
	fn hash(&self, password: &str) -> Result<String, reinhardt_core::exception::Error> {
		use argon2::Argon2;
		use password_hash::{PasswordHasher as _, SaltString, rand_core::OsRng};

		// Generate salt using cryptographically secure randomness
		let salt = SaltString::generate(&mut OsRng);

		let argon2 = Argon2::default();

		argon2
			.hash_password(password.as_bytes(), &salt)
			.map(|hash| hash.to_string())
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))
	}

	fn verify(&self, password: &str, hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		use argon2::Argon2;
		use password_hash::{PasswordHash, PasswordVerifier};

		let parsed_hash = PasswordHash::new(hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))?;

		Ok(Argon2::default()
			.verify_password(password.as_bytes(), &parsed_hash)
			.is_ok())
	}

	fn algorithm(&self) -> Option<&'static str> {
		Some("argon2id")
	}

	fn identify(&self, hash: &str) -> bool {
		use password_hash::PasswordHash;

		PasswordHash::new(hash)
			.map(|parsed| parsed.algorithm.as_str() == "argon2id")
			.unwrap_or(false)
	}

	fn must_update(&self, hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		use argon2::{Argon2, Params, Version};
		use password_hash::PasswordHash;

		let parsed_hash = PasswordHash::new(hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))?;
		let stored_params = Params::try_from(&parsed_hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))?;
		let current = Argon2::default();
		let current_params = current.params();
		let stored_output_len = stored_params
			.output_len()
			.unwrap_or(Params::DEFAULT_OUTPUT_LEN);
		let current_output_len = current_params
			.output_len()
			.unwrap_or(Params::DEFAULT_OUTPUT_LEN);

		Ok(parsed_hash.version != Some(u32::from(Version::default()))
			|| stored_output_len != current_output_len
			|| stored_params.m_cost() != current_params.m_cost()
			|| stored_params.t_cost() != current_params.t_cost()
			|| stored_params.p_cost() != current_params.p_cost())
	}
}

/// Bcrypt password hasher for compatibility with bcrypt-backed applications.
#[cfg(feature = "bcrypt-hasher")]
#[derive(Clone)]
pub struct BcryptHasher {
	cost: u32,
}

#[cfg(feature = "bcrypt-hasher")]
const BCRYPT_MIN_COST: u32 = 4;
#[cfg(feature = "bcrypt-hasher")]
const BCRYPT_MAX_COST: u32 = 31;

#[cfg(feature = "bcrypt-hasher")]
fn parse_bcrypt_hash_parts(hash: &str) -> Option<bcrypt::HashParts> {
	hash.parse::<bcrypt::HashParts>()
		.ok()
		.filter(|parts| (BCRYPT_MIN_COST..=BCRYPT_MAX_COST).contains(&parts.get_cost()))
}

#[cfg(feature = "bcrypt-hasher")]
impl BcryptHasher {
	/// Creates a bcrypt hasher using the bcrypt crate's default cost.
	pub fn new() -> Self {
		Self {
			cost: bcrypt::DEFAULT_COST,
		}
	}

	/// Creates a bcrypt hasher with an explicit cost.
	pub fn with_cost(cost: u32) -> Self {
		Self { cost }
	}
}

#[cfg(feature = "bcrypt-hasher")]
impl Default for BcryptHasher {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(feature = "bcrypt-hasher")]
impl PasswordHasher for BcryptHasher {
	fn hash(&self, password: &str) -> Result<String, reinhardt_core::exception::Error> {
		bcrypt::hash(password, self.cost)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))
	}

	fn verify(&self, password: &str, hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		bcrypt::verify(password, hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))
	}

	fn algorithm(&self) -> Option<&'static str> {
		Some("bcrypt")
	}

	fn identify(&self, hash: &str) -> bool {
		parse_bcrypt_hash_parts(hash).is_some()
	}

	fn must_update(&self, hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		let Some(parts) = parse_bcrypt_hash_parts(hash) else {
			return Ok(false);
		};

		Ok(parts.get_cost() != self.cost || !hash.starts_with("$2b$"))
	}
}
