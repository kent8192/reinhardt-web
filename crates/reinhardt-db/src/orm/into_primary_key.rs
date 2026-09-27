/// Trait for types that can be converted into a model's primary key.
///
/// This trait enables flexible function signatures in generated builder setters,
/// allowing both model instances and raw primary key values to be passed.
///
/// Supported raw keys are [`uuid::Uuid`], [`i32`], [`i64`], and [`String`] when
/// they match the related model's primary key type. Models with a [`String`]
/// primary key also accept string slices (`&str`). An owned string is moved
/// unchanged, while a string slice is copied into the owned key stored by the
/// relationship.
/// A model reference (`&T`) extracts the model's primary key.
///
/// # Examples
///
/// Convert owned and borrowed text keys without loading a related model:
///
/// ```rust
/// use reinhardt_db::orm::{IntoPrimaryKey, Model};
///
/// fn owned_key<T: Model<PrimaryKey = String>>(key: String) -> String {
///     <String as IntoPrimaryKey<T>>::into_primary_key(key)
/// }
///
/// fn borrowed_key<T: Model<PrimaryKey = String>>(key: &str) -> String {
///     <&str as IntoPrimaryKey<T>>::into_primary_key(key)
/// }
/// ```
///
/// ```rust,ignore
/// use reinhardt::db::orm::{Model, IntoPrimaryKey};
/// use uuid::Uuid;
///
/// // Pass a primary key value directly
/// let user_id = Uuid::now_v7();
/// let message = DMMessage::build()
///     .room(room_id)
///     .sender(user_id)
///     .content("Hello")
///     .finish();
///
/// // Pass model instances by reference
/// let message = DMMessage::build()
///     .room(&room)
///     .sender(&user)
///     .content("Hello")
///     .finish();
/// ```
pub trait IntoPrimaryKey<T: super::Model> {
	/// Convert this value into the primary key of model `T`.
	fn into_primary_key(self) -> T::PrimaryKey;
}

// Implementation for model references (borrows the model)
impl<T: super::Model> IntoPrimaryKey<T> for &T {
	fn into_primary_key(self) -> T::PrimaryKey {
		self.primary_key().expect(
			"Model instance must have a primary key set. \
			         Ensure the model was loaded from database or constructed with a PK.",
		)
	}
}

// Implementations for common primary key types
// This avoids conflicts with the blanket impl for T: Model

// UUID (most common in examples)
impl<T: super::Model<PrimaryKey = uuid::Uuid>> IntoPrimaryKey<T> for uuid::Uuid {
	fn into_primary_key(self) -> T::PrimaryKey {
		self
	}
}

// i32
impl<T: super::Model<PrimaryKey = i32>> IntoPrimaryKey<T> for i32 {
	fn into_primary_key(self) -> T::PrimaryKey {
		self
	}
}

// i64
impl<T: super::Model<PrimaryKey = i64>> IntoPrimaryKey<T> for i64 {
	fn into_primary_key(self) -> T::PrimaryKey {
		self
	}
}

// String
impl<T: super::Model<PrimaryKey = String>> IntoPrimaryKey<T> for String {
	fn into_primary_key(self) -> T::PrimaryKey {
		self
	}
}

// Borrowed text for models with owned String primary keys
impl<T: super::Model<PrimaryKey = String>> IntoPrimaryKey<T> for &str {
	fn into_primary_key(self) -> T::PrimaryKey {
		self.to_owned()
	}
}

// Option<Uuid>
impl<T: super::Model<PrimaryKey = uuid::Uuid>> IntoPrimaryKey<T> for Option<uuid::Uuid> {
	fn into_primary_key(self) -> T::PrimaryKey {
		self.expect("Primary key value must be provided")
	}
}

// Option<i32>
impl<T: super::Model<PrimaryKey = i32>> IntoPrimaryKey<T> for Option<i32> {
	fn into_primary_key(self) -> T::PrimaryKey {
		self.expect("Primary key value must be provided")
	}
}

// Option<i64>
impl<T: super::Model<PrimaryKey = i64>> IntoPrimaryKey<T> for Option<i64> {
	fn into_primary_key(self) -> T::PrimaryKey {
		self.expect("Primary key value must be provided")
	}
}
