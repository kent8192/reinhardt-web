//! Asynchronous save hooks for model admins.
//!
//! [`ModelAdmin::before_save`](crate::core::ModelAdmin::before_save) and
//! [`ModelAdmin::after_save`](crate::core::ModelAdmin::after_save) receive an
//! [`AdminSaveContext`] describing the write and an [`AdminChange`] pairing
//! each submitted field value with the value stored before the write.
//!
//! These types contain only plain data, so they are available on both the
//! server and the WASM client targets with the same behavior (P2 parity). The
//! client target never invokes the hooks; the types exist there so shared
//! `ModelAdmin` implementations compile unchanged.

use crate::core::{AdminFormMode, AdminUser};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

/// Request-scoped description of one admin write, passed to save hooks.
///
/// The admin server builds one context per saved record. On update, the
/// context carries the record's canonical primary key and the record as it was
/// stored before the write. On create, [`stored`](Self::stored) is `None`, and
/// [`pk`](Self::pk) is `None` in `before_save` and holds the new primary key in
/// `after_save`.
///
/// Stored records contain every column of the row, keyed by the same field
/// names as admin detail responses.
///
/// The constructor is public so hook implementations can be unit-tested
/// without running the admin server.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::core::{AdminFormMode, AdminSaveContext, AdminUser};
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// struct Staff;
///
/// impl AdminUser for Staff {
///     fn is_active(&self) -> bool { true }
///     fn is_staff(&self) -> bool { true }
///     fn is_superuser(&self) -> bool { false }
///     fn get_username(&self) -> &str { "staff" }
/// }
///
/// let stored = HashMap::from([("is_active".to_owned(), json!(false))]);
/// let ctx = AdminSaveContext::new(&Staff, AdminFormMode::Update)
///     .with_pk("42")
///     .with_stored(&stored);
///
/// assert_eq!(ctx.user().get_username(), "staff");
/// assert_eq!(ctx.mode(), AdminFormMode::Update);
/// assert_eq!(ctx.pk(), Some("42"));
/// assert_eq!(ctx.stored(), Some(&stored));
/// ```
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct AdminSaveContext<'a> {
	user: &'a dyn AdminUser,
	mode: AdminFormMode,
	pk: Option<&'a str>,
	stored: Option<&'a HashMap<String, Value>>,
}

impl<'a> AdminSaveContext<'a> {
	/// Create a context for a write performed by `user`, without a primary key
	/// or stored record.
	pub fn new(user: &'a dyn AdminUser, mode: AdminFormMode) -> Self {
		Self {
			user,
			mode,
			pk: None,
			stored: None,
		}
	}

	/// Attach the canonical primary key of the saved record.
	pub fn with_pk(mut self, pk: &'a str) -> Self {
		self.pk = Some(pk);
		self
	}

	/// Attach the record as it was stored before the write.
	pub fn with_stored(mut self, stored: &'a HashMap<String, Value>) -> Self {
		self.stored = Some(stored);
		self
	}

	/// Return the authenticated admin user performing the write.
	pub fn user(&self) -> &'a dyn AdminUser {
		self.user
	}

	/// Return whether the write creates or updates a record.
	pub fn mode(&self) -> AdminFormMode {
		self.mode
	}

	/// Return the canonical primary key of the saved record, if known.
	pub fn pk(&self) -> Option<&'a str> {
		self.pk
	}

	/// Return the record as stored before the write, or `None` on create.
	pub fn stored(&self) -> Option<&'a HashMap<String, Value>> {
		self.stored
	}
}

impl fmt::Debug for AdminSaveContext<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Stored values are omitted because records may contain credentials
		// such as password hashes.
		f.debug_struct("AdminSaveContext")
			.field("user", &self.user.get_username())
			.field("mode", &self.mode)
			.field("pk", &self.pk)
			.field("has_stored", &self.stored.is_some())
			.finish_non_exhaustive()
	}
}

/// Submitted field values paired with the values stored before the write.
///
/// A change contains only the fields submitted for the saved record, after
/// form normalization, validation, and sanitization, keyed by their submitted
/// names. It does not contain server-generated values such as `auto_now`
/// timestamps, many-to-many selections, or inline child rows.
///
/// The admin server reads each old value from the database column the field
/// is written to, so a foreign key submitted as `author_id` is paired with the
/// stored `author_id` value even when the stored record names it `author`.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::core::AdminChange;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let stored = HashMap::from([
///     ("is_active".to_owned(), json!(false)),
///     ("name".to_owned(), json!("Alice")),
/// ]);
/// let submitted = HashMap::from([
///     ("is_active".to_owned(), json!(true)),
///     ("name".to_owned(), json!("Alice")),
/// ]);
///
/// let change = AdminChange::new(Some(&stored), &submitted);
///
/// assert_eq!(change.changed_fields().collect::<Vec<_>>(), vec!["is_active"]);
/// assert_eq!(change.transition::<bool>("is_active"), Some((false, true)));
/// assert_eq!(change.transition::<String>("name"), None);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct AdminChange {
	fields: BTreeMap<String, FieldChange>,
}

#[derive(Debug, Clone, PartialEq)]
struct FieldChange {
	old: Option<Value>,
	new: Value,
}

impl FieldChange {
	fn is_changed(&self) -> bool {
		self.old.as_ref() != Some(&self.new)
	}
}

impl AdminChange {
	/// Pair every submitted value with the value stored under the same name.
	///
	/// Pass `None` for `stored` when creating a record; every submitted field
	/// then has no old value.
	pub fn new(
		stored: Option<&HashMap<String, Value>>,
		submitted: &HashMap<String, Value>,
	) -> Self {
		let fields = submitted
			.iter()
			.map(|(field, new)| {
				let old = stored.and_then(|stored| stored.get(field)).cloned();
				(
					field.clone(),
					FieldChange {
						old,
						new: new.clone(),
					},
				)
			})
			.collect();
		Self { fields }
	}

	/// Iterate, in field-name order, over submitted fields whose value differs
	/// from the stored value.
	///
	/// A submitted field without a stored value, such as every field of a new
	/// record, counts as changed. Values are compared as JSON, so `1` and
	/// `1.0` differ; use [`transition`](Self::transition) to compare decoded
	/// values.
	pub fn changed_fields(&self) -> impl Iterator<Item = &str> {
		self.fields
			.iter()
			.filter(|(_, change)| change.is_changed())
			.map(|(field, _)| field.as_str())
	}

	/// Return the stored value of a submitted field.
	///
	/// Returns `None` when the field was not submitted or has no stored value.
	pub fn old_value(&self, field: &str) -> Option<&Value> {
		self.fields.get(field)?.old.as_ref()
	}

	/// Return the submitted value of a field, or `None` when it was not
	/// submitted.
	pub fn new_value(&self, field: &str) -> Option<&Value> {
		self.fields.get(field).map(|change| &change.new)
	}

	/// Return the decoded `(old, new)` pair when a submitted field changes.
	///
	/// Returns `Some` only when the field was submitted, has a stored value,
	/// both values decode as `T`, and the decoded values differ.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::AdminChange;
	/// use serde_json::json;
	/// use std::collections::HashMap;
	///
	/// let stored = HashMap::from([("is_active".to_owned(), json!(false))]);
	/// let submitted = HashMap::from([("is_active".to_owned(), json!(true))]);
	/// let change = AdminChange::new(Some(&stored), &submitted);
	///
	/// assert_eq!(change.transition::<bool>("is_active"), Some((false, true)));
	/// assert_eq!(change.transition::<String>("is_active"), None);
	/// ```
	pub fn transition<T>(&self, field: &str) -> Option<(T, T)>
	where
		T: DeserializeOwned + PartialEq,
	{
		let change = self.fields.get(field)?;
		let old = T::deserialize(change.old.as_ref()?).ok()?;
		let new = T::deserialize(&change.new).ok()?;
		(old != new).then_some((old, new))
	}
}

#[cfg(all(test, server))]
mod tests {
	use super::*;
	use rstest::rstest;
	use serde_json::json;

	struct TestUser;

	impl AdminUser for TestUser {
		fn is_active(&self) -> bool {
			true
		}

		fn is_staff(&self) -> bool {
			true
		}

		fn is_superuser(&self) -> bool {
			false
		}

		fn get_username(&self) -> &str {
			"hook-tester"
		}
	}

	fn values(entries: &[(&str, Value)]) -> HashMap<String, Value> {
		entries
			.iter()
			.map(|(field, value)| ((*field).to_owned(), value.clone()))
			.collect()
	}

	#[rstest]
	#[case::activation(json!(false), json!(true), Some((false, true)))]
	#[case::deactivation(json!(true), json!(false), Some((true, false)))]
	#[case::unchanged(json!(true), json!(true), None)]
	#[case::undecodable_old(json!(1), json!(true), None)]
	#[case::undecodable_new(json!(false), json!("true"), None)]
	fn transition_decodes_both_values_and_requires_a_difference(
		#[case] old: Value,
		#[case] new: Value,
		#[case] expected: Option<(bool, bool)>,
	) {
		// Arrange
		let stored = values(&[("is_active", old)]);
		let submitted = values(&[("is_active", new)]);
		let change = AdminChange::new(Some(&stored), &submitted);

		// Act
		let transition = change.transition::<bool>("is_active");

		// Assert
		assert_eq!(transition, expected);
	}

	#[rstest]
	fn transition_compares_decoded_values_rather_than_json() {
		// Arrange
		let stored = values(&[("ratio", json!(1))]);
		let submitted = values(&[("ratio", json!(1.0))]);
		let change = AdminChange::new(Some(&stored), &submitted);

		// Act
		let changed = change.changed_fields().collect::<Vec<_>>();
		let transition = change.transition::<f64>("ratio");

		// Assert
		assert_eq!(changed, vec!["ratio"]);
		assert_eq!(transition, None);
	}

	#[rstest]
	fn unsubmitted_and_unstored_fields_have_no_transition() {
		// Arrange
		let stored = values(&[("is_active", json!(false)), ("is_staff", json!(false))]);
		let submitted = values(&[("name", json!("Alice"))]);
		let change = AdminChange::new(Some(&stored), &submitted);

		// Act
		let unsubmitted = change.transition::<bool>("is_active");
		let unstored = change.transition::<String>("name");

		// Assert
		assert_eq!(unsubmitted, None);
		assert_eq!(unstored, None);
		assert_eq!(change.old_value("is_active"), None);
		assert_eq!(change.new_value("is_active"), None);
		assert_eq!(change.old_value("name"), None);
		assert_eq!(change.new_value("name"), Some(&json!("Alice")));
	}

	#[rstest]
	fn changed_fields_lists_only_differing_submitted_fields_in_name_order() {
		// Arrange
		let stored = values(&[
			("name", json!("Alice")),
			("is_active", json!(false)),
			("email", json!("alice@example.com")),
		]);
		let submitted = values(&[
			("name", json!("Alice")),
			("is_active", json!(true)),
			("email", json!("alice@example.org")),
		]);

		// Act
		let change = AdminChange::new(Some(&stored), &submitted);

		// Assert
		assert_eq!(
			change.changed_fields().collect::<Vec<_>>(),
			vec!["email", "is_active"]
		);
		assert_eq!(change.old_value("email"), Some(&json!("alice@example.com")));
		assert_eq!(change.new_value("email"), Some(&json!("alice@example.org")));
	}

	#[rstest]
	fn create_change_marks_every_submitted_field_as_changed() {
		// Arrange
		let submitted = values(&[("name", json!("Alice")), ("is_active", json!(true))]);

		// Act
		let change = AdminChange::new(None, &submitted);

		// Assert
		assert_eq!(
			change.changed_fields().collect::<Vec<_>>(),
			vec!["is_active", "name"]
		);
		assert_eq!(change.old_value("is_active"), None);
		assert_eq!(change.transition::<bool>("is_active"), None);
	}

	#[rstest]
	fn context_exposes_user_mode_pk_and_stored_record() {
		// Arrange
		let user = TestUser;
		let stored = values(&[("is_active", json!(false))]);

		// Act
		let create = AdminSaveContext::new(&user, AdminFormMode::Create);
		let update = AdminSaveContext::new(&user, AdminFormMode::Update)
			.with_pk("7")
			.with_stored(&stored);

		// Assert
		assert_eq!(create.user().get_username(), "hook-tester");
		assert_eq!(create.mode(), AdminFormMode::Create);
		assert_eq!(create.pk(), None);
		assert_eq!(create.stored(), None);
		assert_eq!(update.mode(), AdminFormMode::Update);
		assert_eq!(update.pk(), Some("7"));
		assert_eq!(update.stored(), Some(&stored));
	}

	#[rstest]
	fn context_debug_omits_stored_values() {
		// Arrange
		let user = TestUser;
		let stored = values(&[("password_hash", json!("secret-hash"))]);
		let ctx = AdminSaveContext::new(&user, AdminFormMode::Update)
			.with_pk("7")
			.with_stored(&stored);

		// Act
		let rendered = format!("{ctx:?}");

		// Assert
		assert_eq!(
			rendered,
			"AdminSaveContext { user: \"hook-tester\", mode: Update, pk: Some(\"7\"), has_stored: true, .. }"
		);
	}
}
