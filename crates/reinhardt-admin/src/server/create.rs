//! Create operation Server Function
//!
//! Provides create operations for admin models.

#[cfg(server)]
use super::admin_auth::AdminAuthenticatedUser;
use crate::adapters::{AdminDatabase, AdminSite};
#[cfg(server)]
use crate::core::{AdminDatabaseKey, AdminSiteKey, ModelAdmin};
#[cfg(server)]
use crate::types::AdminResult;
use crate::types::MutationResponse;
#[cfg(server)]
use reinhardt_di::Depends;
#[cfg(server)]
use reinhardt_pages::server_fn::ServerFnRequest;
use reinhardt_pages::server_fn::{ServerFnError, server_fn};

#[cfg(server)]
use super::audit;
#[cfg(server)]
use super::error::{AdminAuth, MapServerFnError, ModelPermission, require_object_filters};
#[cfg(server)]
use super::security::{require_csrf_token, sanitize_mutation_values};
#[cfg(server)]
use super::validation::validate_mutation_data;

/// Create a new model instance
///
/// Inserts a new record into the database using the provided field data.
/// Returns the number of affected rows (typically 1) on success.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
///
/// # Authentication
///
/// Requires authentication and add permission for the model.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin::server::create_record;
/// use reinhardt_admin::types::MutationRequest;
/// use std::collections::HashMap;
///
/// // Client-side usage (automatically generates HTTP request)
/// let mut data = HashMap::new();
/// data.insert("username".to_string(), serde_json::json!("alice"));
/// data.insert("email".to_string(), serde_json::json!("alice@example.com"));
///
/// let request = MutationRequest { csrf_token: "token".to_string(), data };
/// let response = create_record("User".to_string(), request).await?;
/// println!("Created: {}", response.message);
/// ```
#[server_fn]
pub async fn create_record(
	model_name: String,
	request: crate::types::MutationRequest,
	#[inject] site: Depends<AdminSiteKey, AdminSite>,
	#[inject] db: Depends<AdminDatabaseKey, AdminDatabase>,
	#[inject] http_request: ServerFnRequest,
	#[inject] AdminAuthenticatedUser(user): AdminAuthenticatedUser,
) -> Result<crate::types::MutationResponse, ServerFnError> {
	// CSRF token validation (double-submit cookie pattern)
	require_csrf_token(&request.csrf_token, &http_request.inner().headers)?;

	// Authentication and authorization check
	let auth = AdminAuth::from_request(&http_request);
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	auth.require_model_permission(model_admin.as_ref(), user.as_ref(), ModelPermission::Add)
		.await?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	let mut sanitized_data = prepare_create_data(request.data, model_admin.as_ref(), table_name)
		.map_server_fn_error()?;
	let object_filters = require_object_filters(model_admin.as_ref(), user.as_ref())?;
	enforce_creation_scope(&mut sanitized_data, &object_filters)?;

	let user_id = auth.user_id().unwrap_or("unknown").to_string();

	let result = db
		.create_with_scope(
			table_name,
			Some(pk_field),
			sanitized_data.clone(),
			&object_filters,
		)
		.await
		.map_server_fn_error();

	let success = result.is_ok();
	audit::log_create(&user_id, &model_name, &sanitized_data, success);

	let affected = result?;

	Ok(MutationResponse {
		success: true,
		message: format!("{} created successfully", model_name),
		affected: Some(affected),
		data: None,
	})
}

#[cfg(server)]
pub(crate) fn prepare_create_data(
	mut data: std::collections::HashMap<String, serde_json::Value>,
	model_admin: &dyn ModelAdmin,
	table_name: &str,
) -> AdminResult<std::collections::HashMap<String, serde_json::Value>> {
	validate_mutation_data(&data, model_admin, false)?;
	sanitize_mutation_values(&mut data);
	inject_auto_timestamps(&mut data, table_name);
	Ok(data)
}

/// Apply the same equality scope used for object reads before inserting a row.
/// Unsupported predicates fail closed because their result cannot be proved for a new row.
#[cfg(server)]
pub(crate) fn enforce_creation_scope(
	data: &mut std::collections::HashMap<String, serde_json::Value>,
	filters: &[reinhardt_db::orm::Filter],
) -> Result<(), ServerFnError> {
	use reinhardt_db::orm::{FilterOperator, FilterValue};

	for filter in filters {
		if !matches!(&filter.operator, FilterOperator::Eq)
			|| filter.source_field_name() != Some(filter.field.as_str())
		{
			return Err(ServerFnError::server(403, "Object permission denied"));
		}
		let expected = match &filter.value {
			FilterValue::String(value) => serde_json::Value::String(value.clone()),
			FilterValue::Integer(value) | FilterValue::Int(value) => (*value).into(),
			FilterValue::Float(value) => serde_json::Number::from_f64(*value)
				.map(serde_json::Value::Number)
				.ok_or_else(|| ServerFnError::server(403, "Object permission denied"))?,
			FilterValue::Boolean(value) | FilterValue::Bool(value) => (*value).into(),
			FilterValue::Uuid(value) => serde_json::Value::String(value.to_string()),
			FilterValue::Timestamp(value) => creation_scope_json(value)?,
			FilterValue::Date(value) => creation_scope_json(value)?,
			FilterValue::Time(value) => creation_scope_json(value)?,
			FilterValue::NaiveDateTime(value) => creation_scope_json(value)?,
			FilterValue::Decimal(value) => serde_json::Value::String(value.to_string()),
			FilterValue::Null => serde_json::Value::Null,
			_ => return Err(ServerFnError::server(403, "Object permission denied")),
		};
		if let Some(actual) = data.get(&filter.field) {
			// Compare typed scalars by value, then retain the trusted representation.
			// UTC offsets, fractional-second precision and decimal scale can differ
			// between JSON and delimited input without changing scope membership.
			let matches = match (&filter.value, actual) {
				(FilterValue::Float(value), actual) => actual.as_f64() == Some(*value),
				(FilterValue::Timestamp(value), serde_json::Value::String(actual)) => {
					creation_scope_string_matches(actual, value)
				}
				(FilterValue::Date(value), serde_json::Value::String(actual)) => {
					creation_scope_string_matches(actual, value)
				}
				(FilterValue::Time(value), serde_json::Value::String(actual)) => {
					creation_scope_string_matches(actual, value)
				}
				(FilterValue::NaiveDateTime(value), serde_json::Value::String(actual)) => {
					creation_scope_string_matches(actual, value)
				}
				(FilterValue::Decimal(value), serde_json::Value::String(actual)) => {
					creation_scope_string_matches(actual, value)
				}
				(FilterValue::Decimal(value), serde_json::Value::Number(actual)) => {
					creation_scope_string_matches(&actual.to_string(), value)
				}
				_ => actual == &expected,
			};
			if !matches {
				return Err(ServerFnError::server(403, "Object permission denied"));
			}
		}
		data.insert(filter.field.clone(), expected);
	}
	Ok(())
}

#[cfg(server)]
fn creation_scope_json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, ServerFnError> {
	serde_json::to_value(value).map_err(|_| ServerFnError::server(403, "Object permission denied"))
}

#[cfg(server)]
fn creation_scope_string_matches<T: std::str::FromStr + PartialEq>(
	actual: &str,
	expected: &T,
) -> bool {
	actual.parse::<T>().is_ok_and(|value| &value == expected)
}

/// Injects the current UTC timestamp for fields with `auto_now` or `auto_now_add`.
///
/// This mirrors Django's behavior: `auto_now_add` sets the timestamp on creation,
/// and `auto_now` sets it on every save (both apply during creation). Any existing
/// value for these fields is overwritten — they are always server-controlled.
///
/// For updates, call [`inject_auto_now_timestamps`] instead, which only handles
/// `auto_now` fields.
#[cfg(server)]
pub(crate) fn inject_auto_timestamps(
	data: &mut std::collections::HashMap<String, serde_json::Value>,
	table_name: &str,
) {
	use crate::server::type_inference::find_model_by_table_name;

	let Some(model) = find_model_by_table_name(table_name) else {
		return;
	};

	let now = chrono::Utc::now();

	for (field_name, meta) in &model.fields {
		let is_auto_now = meta
			.params
			.get("auto_now")
			.is_some_and(|v| v == "true" || v == "True");
		let is_auto_now_add = meta
			.params
			.get("auto_now_add")
			.is_some_and(|v| v == "true" || v == "True");

		if is_auto_now || is_auto_now_add {
			// Format based on field type: Date, Time, or DateTime
			let value = match &meta.field_type {
				reinhardt_db::migrations::FieldType::Date => {
					serde_json::Value::String(now.format("%Y-%m-%d").to_string())
				}
				reinhardt_db::migrations::FieldType::Time => {
					serde_json::Value::String(now.format("%H:%M:%S").to_string())
				}
				_ => {
					// DateTime and other types: ISO 8601 format
					serde_json::Value::String(now.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string())
				}
			};
			data.insert(field_name.clone(), value);
		}
	}
}

/// Injects the current UTC timestamp for fields with `auto_now` only.
///
/// Used during updates — `auto_now_add` fields are not touched because they
/// should only be set on initial creation.
#[cfg(server)]
pub(crate) fn inject_auto_now_timestamps(
	data: &mut std::collections::HashMap<String, serde_json::Value>,
	table_name: &str,
) {
	use crate::server::type_inference::find_model_by_table_name;

	let Some(model) = find_model_by_table_name(table_name) else {
		return;
	};

	let now = chrono::Utc::now();

	for (field_name, meta) in &model.fields {
		let is_auto_now = meta
			.params
			.get("auto_now")
			.is_some_and(|v| v == "true" || v == "True");

		if is_auto_now {
			let value = match &meta.field_type {
				reinhardt_db::migrations::FieldType::Date => {
					serde_json::Value::String(now.format("%Y-%m-%d").to_string())
				}
				reinhardt_db::migrations::FieldType::Time => {
					serde_json::Value::String(now.format("%H:%M:%S").to_string())
				}
				_ => serde_json::Value::String(now.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()),
			};
			data.insert(field_name.clone(), value);
		}
	}
}

#[cfg(all(test, server))]
mod tests {
	use super::*;
	use crate::core::ModelAdminConfig;
	use reinhardt_db::orm::{Filter, FilterOperator, FilterValue};

	#[rstest::rstest]
	#[case(FilterValue::Integer(42), "42", serde_json::json!(42))]
	#[case(FilterValue::Int(42), "42", serde_json::json!(42))]
	#[case(FilterValue::Boolean(true), "true", serde_json::json!(true))]
	#[case(FilterValue::Bool(false), "false", serde_json::json!(false))]
	#[case::float(FilterValue::Float(1.25), "1.25", serde_json::json!(1.25))]
	#[case::timestamp(
		FilterValue::Timestamp("2026-10-12T00:30:00Z".parse().unwrap()),
		"2026-10-12T09:30:00+09:00",
		serde_json::json!("2026-10-12T00:30:00Z")
	)]
	#[case::date(
		FilterValue::Date("2026-10-12".parse().unwrap()),
		"2026-10-12",
		serde_json::json!("2026-10-12")
	)]
	#[case::time(
		FilterValue::Time("09:30:00".parse().unwrap()),
		"09:30:00.000",
		serde_json::json!("09:30:00")
	)]
	#[case::naive_datetime(
		FilterValue::NaiveDateTime("2026-10-12T09:30:00".parse().unwrap()),
		"2026-10-12T09:30:00.000",
		serde_json::json!("2026-10-12T09:30:00")
	)]
	#[case::decimal(
		FilterValue::Decimal("42.50".parse().unwrap()),
		"42.5",
		serde_json::json!("42.50")
	)]
	fn delimited_import_scope_accepts_matching_typed_cells(
		#[values(b',', b'\t')] delimiter: u8,
		#[case] scope: FilterValue,
		#[case] cell: &str,
		#[case] expected: serde_json::Value,
	) {
		// Arrange: use the same CSV deserializer as CSV/TSV imports.
		let text = format!(
			"scope{}name\n{cell}{}safe\n",
			delimiter as char, delimiter as char
		);
		let mut reader = csv::ReaderBuilder::new()
			.delimiter(delimiter)
			.from_reader(text.as_bytes());
		let mut record = reader
			.deserialize::<std::collections::HashMap<String, serde_json::Value>>()
			.next()
			.unwrap()
			.unwrap();
		let filter = Filter::new("scope", FilterOperator::Eq, scope);

		// Act: authorize the typed delimited field against its trusted predicate.
		enforce_creation_scope(&mut record, &[filter]).unwrap();

		// Assert: the value saved to the database has the predicate's type.
		assert_eq!(record["scope"], expected);
		assert_eq!(record["name"], serde_json::json!("safe"));
	}

	#[rstest::rstest]
	#[case::float(FilterValue::Float(1.0), serde_json::json!(1), serde_json::json!(1.0))]
	#[case::timestamp(
		FilterValue::Timestamp("2026-10-12T00:30:00Z".parse().unwrap()),
		serde_json::json!("2026-10-12T09:30:00+09:00"),
		serde_json::json!("2026-10-12T00:30:00Z")
	)]
	#[case::date(
		FilterValue::Date("2026-10-12".parse().unwrap()),
		serde_json::json!("2026-10-12"),
		serde_json::json!("2026-10-12")
	)]
	#[case::time(
		FilterValue::Time("09:30:00".parse().unwrap()),
		serde_json::json!("09:30:00.000"),
		serde_json::json!("09:30:00")
	)]
	#[case::naive_datetime(
		FilterValue::NaiveDateTime("2026-10-12T09:30:00".parse().unwrap()),
		serde_json::json!("2026-10-12T09:30:00.000"),
		serde_json::json!("2026-10-12T09:30:00")
	)]
	#[case::decimal(
		FilterValue::Decimal("42.50".parse().unwrap()),
		serde_json::json!("42.5"),
		serde_json::json!("42.50")
	)]
	fn creation_scope_accepts_and_injects_typed_scalars(
		#[case] scope: FilterValue,
		#[case] supplied: serde_json::Value,
		#[case] expected: serde_json::Value,
		#[values(false, true)] field_present: bool,
	) {
		// Arrange: the scope is trusted; the supplied field is application input.
		let mut data =
			std::collections::HashMap::from([("name".into(), serde_json::json!("safe"))]);
		if field_present {
			data.insert("scope".into(), supplied);
		}
		let filters = [Filter::new("scope", FilterOperator::Eq, scope)];

		// Act: accept matching values or inject a missing scope before insertion.
		enforce_creation_scope(&mut data, &filters).unwrap();

		// Assert: both paths use the trusted scalar's canonical JSON representation.
		assert_eq!(data["scope"], expected);
		assert_eq!(data["name"], serde_json::json!("safe"));
	}

	#[rstest::rstest]
	#[case(FilterValue::Integer(42), serde_json::json!("99"))]
	#[case(FilterValue::Integer(42), serde_json::json!("42 OR 1=1"))]
	#[case(FilterValue::Integer(42), serde_json::json!(true))]
	#[case(FilterValue::Boolean(true), serde_json::json!("false"))]
	#[case(FilterValue::Boolean(true), serde_json::json!("1"))]
	#[case(FilterValue::String("42".into()), serde_json::json!(42))]
	#[case(FilterValue::Float(1.25), serde_json::json!(1.5))]
	#[case(FilterValue::Float(1.25), serde_json::json!("1.25"))]
	#[case(FilterValue::Float(f64::NAN), serde_json::Value::Null)]
	#[case(FilterValue::Float(f64::INFINITY), serde_json::Value::Null)]
	#[case(FilterValue::Timestamp("2026-10-12T00:30:00Z".parse().unwrap()), serde_json::json!("2026-10-12T00:30:01Z"))]
	#[case(FilterValue::Date("2026-10-12".parse().unwrap()), serde_json::json!("2026-10-13"))]
	#[case(FilterValue::Date("2026-10-12".parse().unwrap()), serde_json::json!("2026-13-12"))]
	#[case(FilterValue::Time("09:30:00".parse().unwrap()), serde_json::json!("09:31:00"))]
	#[case(FilterValue::NaiveDateTime("2026-10-12T09:30:00".parse().unwrap()), serde_json::json!("2026-10-12T09:30:01"))]
	#[case(FilterValue::Decimal("42.50".parse().unwrap()), serde_json::json!("42.51"))]
	#[case(FilterValue::Decimal("42.50".parse().unwrap()), serde_json::json!(true))]
	#[case(FilterValue::Timestamp("2026-10-12T00:30:00Z".parse().unwrap()), serde_json::json!(0))]
	#[case(FilterValue::Date("2026-10-12".parse().unwrap()), serde_json::json!(20261012))]
	#[case(FilterValue::Time("09:30:00".parse().unwrap()), serde_json::json!(false))]
	#[case(FilterValue::NaiveDateTime("2026-10-12T09:30:00".parse().unwrap()), serde_json::json!({"date": "2026-10-12"}))]
	#[case(FilterValue::Decimal("42.50".parse().unwrap()), serde_json::json!([42.5]))]
	fn creation_scope_denies_mismatched_values_and_json_types(
		#[case] scope: FilterValue,
		#[case] actual: serde_json::Value,
	) {
		// Arrange / Act: caller values cannot redefine the trusted predicate's type.
		let mut data = std::collections::HashMap::from([("scope".into(), actual)]);
		let error = enforce_creation_scope(
			&mut data,
			&[Filter::new("scope", FilterOperator::Eq, scope)],
		)
		.unwrap_err();

		// Assert: invalid, cross-scope and mismatched JSON types remain forbidden.
		assert_eq!(
			error.to_string(),
			"Server error (403): Object permission denied"
		);
	}

	#[rstest::rstest]
	#[case(FilterOperator::Ne, FilterValue::Int(42))]
	#[case(FilterOperator::Gt, FilterValue::Int(42))]
	#[case(FilterOperator::In, FilterValue::List(vec![FilterValue::Int(42)]))]
	#[case(FilterOperator::Eq, FilterValue::Array(vec!["42".into()]))]
	#[case(
		FilterOperator::Eq,
		FilterValue::FieldRef(reinhardt_db::orm::expressions::F::new("other_scope"))
	)]
	#[case(FilterOperator::Eq, FilterValue::Expression(reinhardt_db::orm::annotation::Expression::Coalesce(vec![])))]
	fn creation_scope_denies_non_scalar_or_non_equality_predicates(
		#[case] operator: FilterOperator,
		#[case] value: FilterValue,
	) {
		// Arrange / Act: creation cannot derive a scalar from arbitrary predicates.
		let mut data = std::collections::HashMap::new();
		let error = enforce_creation_scope(&mut data, &[Filter::new("scope", operator, value)])
			.unwrap_err();
		// Assert
		assert_eq!(
			error.to_string(),
			"Server error (403): Object permission denied"
		);
		assert!(!data.contains_key("scope"));
	}

	#[rstest::rstest]
	fn creation_scope_sets_or_rejects_tenant_id() {
		let filters = [Filter::new(
			"tenant_id",
			FilterOperator::Eq,
			FilterValue::Integer(42),
		)];
		let mut data =
			std::collections::HashMap::from([("name".to_string(), serde_json::json!("safe"))]);
		enforce_creation_scope(&mut data, &filters).unwrap();
		assert_eq!(data.get("tenant_id"), Some(&serde_json::json!(42)));

		data.insert("tenant_id".to_string(), serde_json::json!(99));
		assert!(enforce_creation_scope(&mut data, &filters).is_err());
	}

	#[test]
	fn prepare_create_data_rejects_readonly_import_fields() {
		let admin = ModelAdminConfig::builder()
			.model_name("Record")
			.list_display(vec!["id", "name"])
			.fields(vec!["id", "name", "owner_id"])
			.readonly_fields(vec!["owner_id"])
			.build()
			.expect("test admin should build");
		let data = std::collections::HashMap::from([
			("name".to_string(), serde_json::json!("safe")),
			("owner_id".to_string(), serde_json::json!(42)),
		]);

		let result = prepare_create_data(data, &admin, "records");

		assert!(matches!(
			result,
			Err(crate::types::AdminError::ValidationError(message))
				if message == "Field 'owner_id' is read-only and cannot be modified"
		));
	}
}
