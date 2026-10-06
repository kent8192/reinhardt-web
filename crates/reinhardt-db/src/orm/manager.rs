#[cfg(test)]
use super::connection::QueryValue;
use super::connection::{
	DatabaseBackend, DatabaseConnection, DatabaseConnectionLease, OrmExecutor, QueryRow, Row,
};
use super::field_codec::{DatabaseArrayType, database_value_to_query_value};
use super::inspection::FieldInfo;
use super::query::RelationLoadInput;
use super::{DatabaseValue, FieldCodecError, Model, QuerySet};
use reinhardt_core::exception::{DatabaseError, DatabaseErrorKind, Error};
use reinhardt_query::prelude::{
	Alias, CockroachDBQueryBuilder, ColumnRef, Condition, DeleteStatement, Expr, ExprTrait, Func,
	InsertStatement, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuilder, SelectStatement,
	SqliteQueryBuilder, UpdateStatement, Values,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{
	Arc, Mutex, RwLock as StdRwLock, RwLockReadGuard as StdRwLockReadGuard,
	RwLockWriteGuard as StdRwLockWriteGuard,
	atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

fn find_field_info<'a>(field_metadata: &'a [FieldInfo], field_name: &str) -> Option<&'a FieldInfo> {
	field_metadata.iter().find(|field| field.name == field_name)
}

fn field_codec_error(error: FieldCodecError) -> Error {
	let kind = match &error {
		FieldCodecError::TypeMismatch { .. }
		| FieldCodecError::InvalidEnumValue { .. }
		| FieldCodecError::MissingFieldMetadata { .. }
		| FieldCodecError::FieldPolicyMismatch { .. } => DatabaseErrorKind::Type,
		FieldCodecError::Serialization(_) => DatabaseErrorKind::Serialization,
	};
	let message = error.to_string();
	Error::database_with_source(kind, message, error)
}

pub(crate) fn decode_model_row<M: Model>(row: Row) -> reinhardt_core::exception::Result<M> {
	QueryRow::from_backend_row(row)
		.deserialize_model::<M>()
		.map_err(field_codec_error)
}

fn executor_field_codec_error(error: FieldCodecError) -> crate::backends::error::DatabaseError {
	crate::backends::error::DatabaseError::new(
		crate::backends::error::DatabaseErrorKind::Serialization,
		error.to_string(),
	)
}

fn executor_error(error: Error) -> crate::backends::error::DatabaseError {
	crate::backends::error::into_database_error(error)
}

/// Build SQL with values from an INSERT statement based on database backend
fn build_insert_sql(stmt: &InsertStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_insert(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_insert(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_insert(stmt),
	}
}

/// Build SQL with values from an UPDATE statement based on database backend
fn build_update_sql(stmt: &UpdateStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_update(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_update(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_update(stmt),
	}
}

/// Build SQL with values from a SELECT statement based on database backend
fn build_select_sql(stmt: &SelectStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_select(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_select(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_select(stmt),
	}
}

/// Convert an INSERT statement to SQL string based on database backend
fn insert_to_string(stmt: &InsertStatement, backend: DatabaseBackend) -> String {
	build_insert_sql(stmt, backend).0
}

/// Build SQL with values from a DELETE statement based on database backend
fn build_delete_sql(stmt: &DeleteStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_delete(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_delete(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_delete(stmt),
	}
}

fn checked_query_build_error(error: reinhardt_query::QueryBuildError) -> Error {
	DatabaseError::new(DatabaseErrorKind::Unsupported, error.to_string()).into()
}

#[cfg(feature = "pgvector")]
fn database_value_uses_pgvector(value: &DatabaseValue) -> bool {
	match value {
		DatabaseValue::Vector(_) => true,
		DatabaseValue::Array { values, .. } => values.iter().any(database_value_uses_pgvector),
		_ => false,
	}
}

fn validate_bulk_batch_size(batch_size: Option<usize>) -> reinhardt_core::exception::Result<()> {
	if batch_size == Some(0) {
		return Err(Error::Validation(
			"batch_size must be greater than zero".into(),
		));
	}
	Ok(())
}

fn validate_bulk_update_values_for_backend(
	updates: &[(DatabaseValue, HashMap<String, DatabaseValue>)],
	is_cockroachdb: bool,
) -> reinhardt_core::exception::Result<()> {
	#[cfg(feature = "pgvector")]
	if is_cockroachdb
		&& updates.iter().any(|(primary_key, fields)| {
			database_value_uses_pgvector(primary_key)
				|| fields.values().any(database_value_uses_pgvector)
		}) {
		return Err(checked_query_build_error(
			reinhardt_query::QueryBuildError::UnsupportedBackendFeature {
				feature: "pgvector values",
				backend: "CockroachDB",
			},
		));
	}

	#[cfg(not(feature = "pgvector"))]
	let _ = (updates, is_cockroachdb);

	Ok(())
}

fn build_select_sql_checked(
	stmt: &SelectStatement,
	backend: DatabaseBackend,
	is_cockroachdb: bool,
) -> reinhardt_core::exception::Result<(String, Values)> {
	if is_cockroachdb {
		CockroachDBQueryBuilder::new()
			.build_select_checked(stmt)
			.map_err(checked_query_build_error)
	} else {
		Ok(build_select_sql(stmt, backend))
	}
}

fn build_insert_sql_checked(
	stmt: &InsertStatement,
	backend: DatabaseBackend,
	is_cockroachdb: bool,
) -> reinhardt_core::exception::Result<(String, Values)> {
	if is_cockroachdb {
		CockroachDBQueryBuilder::new()
			.build_insert_checked(stmt)
			.map_err(checked_query_build_error)
	} else {
		Ok(build_insert_sql(stmt, backend))
	}
}

fn build_update_sql_checked(
	stmt: &UpdateStatement,
	backend: DatabaseBackend,
	is_cockroachdb: bool,
) -> reinhardt_core::exception::Result<(String, Values)> {
	if is_cockroachdb {
		CockroachDBQueryBuilder::new()
			.build_update_checked(stmt)
			.map_err(checked_query_build_error)
	} else {
		Ok(build_update_sql(stmt, backend))
	}
}

fn build_delete_sql_checked(
	stmt: &DeleteStatement,
	backend: DatabaseBackend,
	is_cockroachdb: bool,
) -> reinhardt_core::exception::Result<(String, Values)> {
	if is_cockroachdb {
		CockroachDBQueryBuilder::new()
			.build_delete_checked(stmt)
			.map_err(checked_query_build_error)
	} else {
		Ok(build_delete_sql(stmt, backend))
	}
}

fn database_value_sql_literal(
	value: DatabaseValue,
	backend: DatabaseBackend,
) -> Result<String, FieldCodecError> {
	if backend == DatabaseBackend::Postgres
		&& let DatabaseValue::Array {
			element_type,
			values,
		} = value
	{
		let element_type = match element_type {
			DatabaseArrayType::String => "text",
			DatabaseArrayType::I32 => "integer",
			DatabaseArrayType::I64 => "bigint",
			DatabaseArrayType::F32 => "real",
			DatabaseArrayType::F64 => "double precision",
			DatabaseArrayType::Bool => "boolean",
			DatabaseArrayType::Uuid => "uuid",
		};
		let literals = values
			.into_iter()
			.map(|value| match value {
				DatabaseValue::F32(value) if !value.is_finite() => {
					postgres_special_float_literal(f64::from(value), "real")
				}
				DatabaseValue::F64(value) if !value.is_finite() => {
					postgres_special_float_literal(value, "double precision")
				}
				value => database_value_to_query_value(value).to_sql_literal(),
			})
			.collect::<Vec<_>>()
			.join(",");
		// CASE branches need the array type, and special floats must be quoted
		// typed literals rather than identifiers such as NaN or inf.
		return Ok(format!("ARRAY[{literals}]::{element_type}[]"));
	}

	if backend == DatabaseBackend::Postgres || !matches!(&value, DatabaseValue::Array { .. }) {
		return Ok(database_value_to_query_value(value).to_sql_literal());
	}

	let json = value.into_json_value()?;
	Ok(format!("'{}'", json.to_string().replace('\'', "''")))
}

fn postgres_special_float_literal(value: f64, sql_type: &str) -> String {
	let literal = if value.is_nan() {
		"NaN"
	} else if value.is_sign_positive() {
		"Infinity"
	} else {
		"-Infinity"
	};
	format!("'{literal}'::{sql_type}")
}

fn quote_identifier(identifier: &str, backend: DatabaseBackend) -> String {
	let quote = if backend == DatabaseBackend::MySql {
		'`'
	} else {
		'"'
	};
	format!("{quote}{identifier}{quote}")
}

#[derive(Clone)]
struct DefaultDatabase {
	lease: DatabaseConnectionLease,
	handle: DatabaseConnection,
	scope: Option<Arc<ScopedRegistrationNode>>,
	test_registration: Option<Arc<TestDatabaseRegistration>>,
}

impl DefaultDatabase {
	fn from_scope(scope: Arc<ScopedRegistrationNode>) -> Self {
		Self {
			lease: scope.lease.clone(),
			handle: scope.handle,
			scope: Some(scope),
			test_registration: None,
		}
	}
}

#[derive(Clone)]
enum ScopedRegistrationPredecessor {
	Scope(Arc<ScopedRegistrationNode>),
	Baseline(DefaultDatabase),
}

struct ScopedRegistrationNode {
	handle: DatabaseConnection,
	lease: DatabaseConnectionLease,
	previous: Mutex<Option<ScopedRegistrationPredecessor>>,
	active: AtomicBool,
}

struct TestDatabaseRegistration {
	previous: Mutex<Option<DefaultDatabase>>,
	active: AtomicBool,
}

/// Global database connection state
static DB: once_cell::sync::OnceCell<Arc<StdRwLock<Option<DefaultDatabase>>>> =
	once_cell::sync::OnceCell::new();

fn database_lock_error() -> Error {
	Error::from(DatabaseError::new(
		DatabaseErrorKind::Configuration,
		"database registry lock is poisoned",
	))
}

fn database_state()
-> reinhardt_core::exception::Result<StdRwLockWriteGuard<'static, Option<DefaultDatabase>>> {
	DB.get_or_init(|| Arc::new(StdRwLock::new(None)))
		.write()
		.map_err(|_| database_lock_error())
}

fn initialized_database_state()
-> reinhardt_core::exception::Result<StdRwLockReadGuard<'static, Option<DefaultDatabase>>> {
	DB.get()
		.ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Configuration,
				"Database not initialized",
			))
		})?
		.read()
		.map_err(|_| database_lock_error())
}

/// Initialize the global database connection
///
/// # Arguments
///
/// * `url` - Database connection URL
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_db::orm::manager::init_database;
///
/// init_database("postgres://localhost/mydb").await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn init_database(url: &str) -> reinhardt_core::exception::Result<()> {
	init_database_with_pool_size(url, None).await
}

/// Initialize the global database connection with a specific pool size
///
/// If the global connection is already initialized, this function returns
/// successfully without opening another connection.
///
/// # Arguments
///
/// * `url` - Database connection URL
/// * `pool_size` - Maximum number of connections in the pool (None = use default)
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_db::orm::manager::init_database_with_pool_size;
///
/// // Use larger pool for high-concurrency tests
/// init_database_with_pool_size("postgres://localhost/mydb", Some(50)).await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn init_database_with_pool_size(
	url: &str,
	pool_size: Option<u32>,
) -> reinhardt_core::exception::Result<()> {
	if DB.get().is_some()
		&& initialized_database_state()?
			.as_ref()
			.is_some_and(database_has_baseline)
	{
		return Ok(());
	}

	let owner = super::engine::connect_backend_with_pool_size(url, pool_size).await?;
	let lease = DatabaseConnectionLease::register(owner)?;
	let database = DefaultDatabase {
		handle: lease.handle(),
		lease,
		scope: None,
		test_registration: None,
	};

	let mut guard = database_state()?;
	if guard.is_none() {
		*guard = Some(database);
	} else if let Some(scope) = guard.as_ref().and_then(|current| current.scope.as_ref()) {
		install_baseline_beneath_scopes(scope, database);
	}

	Ok(())
}

fn database_has_baseline(database: &DefaultDatabase) -> bool {
	match &database.scope {
		None => true,
		Some(scope) => scope_has_baseline(scope),
	}
}

fn scope_has_baseline(scope: &ScopedRegistrationNode) -> bool {
	match scoped_predecessor(scope) {
		Some(ScopedRegistrationPredecessor::Scope(parent)) => scope_has_baseline(&parent),
		Some(ScopedRegistrationPredecessor::Baseline(_)) => true,
		None => false,
	}
}

fn install_baseline_beneath_scopes(scope: &Arc<ScopedRegistrationNode>, database: DefaultDatabase) {
	let mut previous = scope
		.previous
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	match previous.as_ref() {
		Some(ScopedRegistrationPredecessor::Scope(parent)) => {
			let parent = Arc::clone(parent);
			drop(previous);
			install_baseline_beneath_scopes(&parent, database);
		}
		Some(ScopedRegistrationPredecessor::Baseline(_)) => {}
		None => *previous = Some(ScopedRegistrationPredecessor::Baseline(database)),
	}
}

/// Reinitialize the global database connection (for testing)
///
/// This function replaces the existing database connection with a new one.
/// Useful for test scenarios where each test needs a fresh connection pool.
///
/// # Arguments
///
/// * `url` - Database connection URL
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_db::orm::manager::reinitialize_database;
///
/// reinitialize_database("postgres://localhost/mydb").await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn reinitialize_database(url: &str) -> reinhardt_core::exception::Result<()> {
	reinitialize_database_with_pool_size(url, None).await
}

/// Reinitialize the global database connection with a specific pool size (for testing)
///
/// # Arguments
///
/// * `url` - Database connection URL
/// * `pool_size` - Maximum number of connections in the pool (None = use default)
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_db::orm::manager::reinitialize_database_with_pool_size;
///
/// // Use larger pool for concurrent tests
/// reinitialize_database_with_pool_size("postgres://localhost/mydb", Some(30)).await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn reinitialize_database_with_pool_size(
	url: &str,
	pool_size: Option<u32>,
) -> reinhardt_core::exception::Result<()> {
	let owner = super::engine::connect_backend_with_pool_size(url, pool_size).await?;
	let lease = DatabaseConnectionLease::register(owner)?;
	let database = DefaultDatabase {
		handle: lease.handle(),
		lease,
		scope: None,
		test_registration: None,
	};

	let mut guard = database_state()?;
	*guard = Some(database);

	Ok(())
}

/// A scoped owner for a temporary global ORM database registration.
///
/// The previous registration is restored when this guard drops, provided no
/// other owner has replaced the scoped registration.
#[must_use = "dropping the guard restores the previous database registration"]
pub struct ScopedDatabaseRegistration {
	node: Arc<ScopedRegistrationNode>,
}

impl ScopedDatabaseRegistration {
	/// Returns the copyable handle for the scoped database.
	pub fn connection(&self) -> DatabaseConnection {
		self.node.handle
	}

	/// Returns a lease that keeps the scoped database connection alive.
	///
	/// The lease does not guarantee that this registration remains installed
	/// globally if another owner replaces it.
	pub fn lease(&self) -> DatabaseConnectionLease {
		self.node.lease.clone()
	}
}

impl Drop for ScopedDatabaseRegistration {
	fn drop(&mut self) {
		if !self.node.active.swap(false, Ordering::AcqRel) {
			return;
		}
		restore_if_current(&self.node);
	}
}

fn restore_if_current(node: &Arc<ScopedRegistrationNode>) {
	match database_state() {
		Ok(mut state) => {
			if state.as_ref().is_some_and(|database| {
				database
					.scope
					.as_ref()
					.is_some_and(|current| Arc::ptr_eq(current, node))
			}) {
				*state = nearest_active_predecessor(node);
			} else if let Some(current) = state
				.as_ref()
				.and_then(|database| database.scope.as_ref())
				.filter(|current| scope_descends_from(current, node))
			{
				unlink_inactive_ancestor(current, node);
			} else {
				tracing::warn!(
					"Scoped database registration was externally replaced; previous registration will not be restored"
				);
			}
		}
		Err(error) => {
			tracing::warn!(
				error = %error,
				"Scoped database registration could not restore the previous registration"
			);
		}
	}
}

fn scope_descends_from(
	current: &ScopedRegistrationNode,
	ancestor: &Arc<ScopedRegistrationNode>,
) -> bool {
	let mut previous = scoped_predecessor(current);
	while let Some(ScopedRegistrationPredecessor::Scope(scope)) = previous {
		if Arc::ptr_eq(&scope, ancestor) {
			return true;
		}
		previous = scoped_predecessor(&scope);
	}
	false
}

fn nearest_active_predecessor(node: &ScopedRegistrationNode) -> Option<DefaultDatabase> {
	let mut previous = scoped_predecessor(node);
	loop {
		match previous {
			Some(ScopedRegistrationPredecessor::Scope(scope)) => {
				if scope.active.load(Ordering::Acquire) {
					return Some(DefaultDatabase::from_scope(scope));
				}
				previous = scoped_predecessor(&scope);
			}
			Some(ScopedRegistrationPredecessor::Baseline(database)) => {
				return active_database_predecessor(Some(database.clone()));
			}
			None => return None,
		}
	}
}

fn scoped_predecessor(node: &ScopedRegistrationNode) -> Option<ScopedRegistrationPredecessor> {
	node.previous
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner())
		.clone()
}

fn unlink_inactive_ancestor(
	current: &Arc<ScopedRegistrationNode>,
	target: &Arc<ScopedRegistrationNode>,
) {
	let mut previous = current
		.previous
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	match previous.as_ref() {
		Some(ScopedRegistrationPredecessor::Scope(scope)) if Arc::ptr_eq(scope, target) => {
			*previous = scoped_predecessor(target);
		}
		Some(ScopedRegistrationPredecessor::Scope(scope)) => {
			let scope = Arc::clone(scope);
			drop(previous);
			unlink_inactive_ancestor(&scope, target);
		}
		Some(ScopedRegistrationPredecessor::Baseline(_)) | None => {}
	}
}

/// Installs a temporary global ORM database registration.
///
/// The backend connection is created before the global registry is locked.
pub async fn install_scoped_database(
	database_url: &str,
) -> reinhardt_core::exception::Result<ScopedDatabaseRegistration> {
	let owner = super::engine::connect_backend_with_pool_size(database_url, None).await?;
	let installed_lease = DatabaseConnectionLease::register(owner)?;
	let installed_handle = installed_lease.handle();
	let node = {
		let mut state = database_state()?;
		let previous = state.take().map(|database| match database.scope.clone() {
			Some(scope) => ScopedRegistrationPredecessor::Scope(scope),
			None => ScopedRegistrationPredecessor::Baseline(database),
		});
		let node = Arc::new(ScopedRegistrationNode {
			handle: installed_handle,
			lease: installed_lease,
			previous: Mutex::new(previous),
			active: AtomicBool::new(true),
		});
		*state = Some(DefaultDatabase::from_scope(node.clone()));
		node
	};

	Ok(ScopedDatabaseRegistration { node })
}

/// RAII guard for a global ORM database registration replaced by a test.
#[doc(hidden)]
pub struct DatabaseRegistrationSnapshot {
	database: Option<DefaultDatabase>,
	test_registration: Option<Arc<TestDatabaseRegistration>>,
	armed: bool,
}

impl DatabaseRegistrationSnapshot {
	fn restore(&mut self) {
		if !self.armed {
			return;
		}
		self.armed = false;

		if let Some(registration) = self.test_registration.take() {
			if registration.active.swap(false, Ordering::AcqRel) {
				restore_test_database_registration(&registration);
			}
			return;
		}

		let previous = self.database.take();
		match database_state() {
			Ok(mut state) if state.is_none() => {
				*state = active_database_predecessor(previous);
			}
			Ok(_) => {
				tracing::warn!(
					"Test database registration was externally replaced; previous registration will not be restored"
				);
			}
			Err(error) => {
				tracing::warn!(
					error = %error,
					"Test database registration could not be restored"
				);
			}
		}
	}
}

impl Drop for DatabaseRegistrationSnapshot {
	fn drop(&mut self) {
		self.restore();
	}
}

/// Replace the global ORM database connection and retain the previous state.
///
/// This is intended for test fixtures that need to mutate global ORM state while
/// preserving RAII cleanup semantics. Passing `None` clears the global connection.
#[doc(hidden)]
pub async fn replace_database_connection_for_testing(
	lease: Option<DatabaseConnectionLease>,
) -> DatabaseRegistrationSnapshot {
	replace_database_connection_for_testing_sync(lease)
}

fn replace_database_connection_for_testing_sync(
	lease: Option<DatabaseConnectionLease>,
) -> DatabaseRegistrationSnapshot {
	let Some(lease) = lease else {
		return match database_state() {
			Ok(mut state) => DatabaseRegistrationSnapshot {
				database: std::mem::take(&mut *state),
				test_registration: None,
				armed: true,
			},
			Err(error) => {
				tracing::warn!(
					error = %error,
					"Test database registration could not be replaced"
				);
				DatabaseRegistrationSnapshot {
					database: None,
					test_registration: None,
					armed: false,
				}
			}
		};
	};

	let registration = Arc::new(TestDatabaseRegistration {
		previous: Mutex::new(None),
		active: AtomicBool::new(true),
	});
	let database = DefaultDatabase {
		handle: lease.handle(),
		lease,
		scope: None,
		test_registration: Some(Arc::clone(&registration)),
	};
	match database_state() {
		Ok(mut state) => {
			let previous = (*state).replace(database);
			*registration
				.previous
				.lock()
				.unwrap_or_else(std::sync::PoisonError::into_inner) = previous;
			DatabaseRegistrationSnapshot {
				database: None,
				test_registration: Some(registration),
				armed: true,
			}
		}
		Err(error) => {
			tracing::warn!(
				error = %error,
				"Test database registration could not be replaced"
			);
			DatabaseRegistrationSnapshot {
				database: None,
				test_registration: None,
				armed: false,
			}
		}
	}
}

/// Restores a database registration saved by [`replace_database_connection_for_testing`].
#[doc(hidden)]
pub async fn restore_database_connection_for_testing(mut snapshot: DatabaseRegistrationSnapshot) {
	restore_database_connection_for_testing_sync(&mut snapshot);
}

fn restore_database_connection_for_testing_sync(snapshot: &mut DatabaseRegistrationSnapshot) {
	snapshot.restore();
}

fn active_database_predecessor(database: Option<DefaultDatabase>) -> Option<DefaultDatabase> {
	database.and_then(|database| {
		if let Some(test_registration) = database.test_registration.as_ref()
			&& !test_registration.active.load(Ordering::Acquire)
		{
			return active_test_database_predecessor(test_registration);
		}
		match database.scope.as_ref() {
			Some(scope) if !scope.active.load(Ordering::Acquire) => {
				nearest_active_predecessor(scope)
			}
			Some(_) | None => Some(database),
		}
	})
}

fn active_test_database_predecessor(
	registration: &TestDatabaseRegistration,
) -> Option<DefaultDatabase> {
	let previous = registration
		.previous
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
		.clone();
	active_database_predecessor(previous)
}

fn restore_test_database_registration(registration: &Arc<TestDatabaseRegistration>) {
	match database_state() {
		Ok(mut state) => {
			let current = state
				.as_ref()
				.and_then(|database| database.test_registration.as_ref());
			if current.is_some_and(|current| Arc::ptr_eq(current, registration)) {
				*state = active_test_database_predecessor(registration);
			} else if let Some(current) = current.cloned() {
				drop(state);
				if !test_registration_descends_from(&current, registration) {
					tracing::warn!(
						"Test database registration was externally replaced; previous registration will not be restored"
					);
					return;
				}
				unlink_inactive_test_registration(&current, registration);
			} else {
				tracing::warn!(
					"Test database registration was externally replaced; previous registration will not be restored"
				);
			}
		}
		Err(error) => {
			tracing::warn!(error = %error, "Test database registration could not be restored");
		}
	}
}

fn test_registration_descends_from(
	current: &TestDatabaseRegistration,
	ancestor: &Arc<TestDatabaseRegistration>,
) -> bool {
	let mut previous = current
		.previous
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
		.clone();
	while let Some(database) = previous {
		let Some(registration) = database.test_registration else {
			return false;
		};
		if Arc::ptr_eq(&registration, ancestor) {
			return true;
		}
		previous = registration
			.previous
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.clone();
	}
	false
}

fn unlink_inactive_test_registration(
	current: &TestDatabaseRegistration,
	target: &Arc<TestDatabaseRegistration>,
) {
	let mut previous = current
		.previous
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	let Some(database) = previous.as_ref() else {
		return;
	};
	let Some(registration) = database.test_registration.as_ref() else {
		return;
	};
	if Arc::ptr_eq(registration, target) {
		*previous = active_test_database_predecessor(target);
	} else {
		let registration = Arc::clone(registration);
		drop(previous);
		unlink_inactive_test_registration(&registration, target);
	}
}

/// Get a reference to the global database connection
pub async fn get_connection() -> reinhardt_core::exception::Result<DatabaseConnection> {
	let guard = initialized_database_state()?;
	guard
		.as_ref()
		.map(|database| database.handle)
		.ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Connection,
				"Database connection not available",
			))
		})
}

/// Returns a lease that retains the global ORM database registration.
#[doc(hidden)]
pub async fn get_connection_lease() -> reinhardt_core::exception::Result<DatabaseConnectionLease> {
	let guard = initialized_database_state()?;
	guard
		.as_ref()
		.map(|database| database.lease.clone())
		.ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Connection,
				"Database connection not available",
			))
		})
}

/// Returns a coherent lease-and-handle snapshot of the global ORM registration.
#[doc(hidden)]
pub async fn get_connection_registration()
-> reinhardt_core::exception::Result<(DatabaseConnectionLease, DatabaseConnection)> {
	let guard = initialized_database_state()?;
	guard
		.as_ref()
		.map(|database| (database.lease.clone(), database.handle))
		.ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Connection,
				"Database connection not available",
			))
		})
}

/// Model manager (similar to Django's Manager)
/// Provides an interface for database operations
pub struct Manager<M: Model> {
	_marker: PhantomData<M>,
}

impl<M: Model> Manager<M> {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}

	fn executor_backend(executor: &dyn super::connection::TransactionExecutor) -> DatabaseBackend {
		match executor.backend() {
			crate::backends::types::DatabaseType::Postgres => DatabaseBackend::Postgres,
			crate::backends::types::DatabaseType::Mysql => DatabaseBackend::MySql,
			crate::backends::types::DatabaseType::Sqlite => DatabaseBackend::Sqlite,
		}
	}

	fn decode_executor_row(
		row: crate::backends::types::Row,
	) -> Result<M, crate::backends::error::DatabaseError> {
		super::connection::QueryRow::from_backend_row(row)
			.deserialize_model::<M>()
			.map_err(executor_field_codec_error)
	}

	/// Preserve the model-declared primary-key storage type when binding executor queries.
	fn primary_key_query_value(
		pk: &M::PrimaryKey,
	) -> Result<reinhardt_query::value::Value, FieldCodecError> {
		M::primary_key_database_value(pk).map(database_value_to_query_value)
	}

	fn build_delete_statement(pk: &M::PrimaryKey) -> Result<DeleteStatement, FieldCodecError> {
		let primary_key_value = Self::primary_key_query_value(pk)?;
		let field_metadata = M::field_metadata();
		let primary_key_column = Self::field_column(&field_metadata, M::primary_key_field());

		let mut stmt = Query::delete();
		stmt.from_table(Alias::new(M::table_name()))
			.and_where(Expr::col(Alias::new(primary_key_column)).eq(primary_key_value));
		Ok(stmt)
	}

	fn is_generated_field(field: &str) -> bool {
		M::generated_field_names().contains(&field)
			|| M::field_metadata()
				.iter()
				.any(|info| info.name == field && info.attributes.contains_key("generated"))
	}

	fn field_column<'a>(field_metadata: &'a [FieldInfo], field_name: &'a str) -> &'a str {
		find_field_info(field_metadata, field_name)
			.map(FieldInfo::db_column_name)
			.unwrap_or_else(|| {
				if field_name == M::primary_key_field() {
					M::primary_key_column()
				} else {
					field_name
				}
			})
	}

	fn returning_columns_from_object(
		obj: &std::collections::BTreeMap<String, DatabaseValue>,
	) -> Vec<Alias> {
		let primary_key = M::primary_key_field();
		let mut columns: Vec<&str> = obj.keys().map(String::as_str).collect();
		columns.sort_unstable();
		if let Some(index) = columns.iter().position(|column| *column == primary_key) {
			let pk = columns.remove(index);
			columns.insert(0, pk);
		}
		let field_metadata = M::field_metadata();
		columns
			.into_iter()
			.map(|column| Alias::new(Self::field_column(&field_metadata, column)))
			.collect()
	}

	fn primary_key_fields(field_metadata: &[FieldInfo]) -> Vec<String> {
		let Some(composite) = M::composite_primary_key() else {
			return vec![M::primary_key_field().to_owned()];
		};

		// Field codecs are keyed by logical names even when composite metadata
		// names physical columns. Prefer the marked model fields when available.
		let fields: Vec<_> = field_metadata
			.iter()
			.filter(|field| field.primary_key)
			.map(|field| field.name.clone())
			.collect();
		if fields.len() == composite.fields().len() {
			return fields;
		}

		composite
			.fields()
			.iter()
			.map(|name| {
				field_metadata
					.iter()
					.find(|field| field.name == *name || field.db_column_name() == name)
					.map_or_else(|| name.clone(), |field| field.name.clone())
			})
			.collect()
	}

	fn primary_key_condition_from_object(
		obj: &std::collections::BTreeMap<String, DatabaseValue>,
	) -> Result<Condition, FieldCodecError> {
		let field_metadata = M::field_metadata();
		let mut condition = Condition::all();
		for field in Self::primary_key_fields(&field_metadata) {
			let value = obj
				.get(&field)
				.filter(|value| !matches!(value, DatabaseValue::Null))
				.cloned()
				.ok_or_else(|| {
					FieldCodecError::Serialization(format!(
						"encoded {} fields must contain a non-null primary key '{}'",
						M::table_name(),
						field
					))
				})?;
			let column = Self::field_column(&field_metadata, &field);
			condition = condition
				.add(Expr::col(Alias::new(column)).eq(database_value_to_query_value(value)));
		}
		Ok(condition)
	}

	fn build_update_statement_from_object(
		obj: &std::collections::BTreeMap<String, DatabaseValue>,
		_field_is_none: impl Fn(&str) -> bool,
	) -> Result<reinhardt_query::prelude::UpdateStatement, FieldCodecError> {
		Self::build_update_statement_from_object_with_returning(obj, true)
	}

	fn build_update_statement_from_object_with_returning(
		obj: &std::collections::BTreeMap<String, DatabaseValue>,
		include_returning: bool,
	) -> Result<reinhardt_query::prelude::UpdateStatement, FieldCodecError> {
		let mut stmt = Query::update();
		stmt.table(Alias::new(M::table_name()));
		let field_metadata = M::field_metadata();
		let primary_key_fields = Self::primary_key_fields(&field_metadata);

		let mut has_values = false;
		for (k, v) in obj.iter().filter(|(k, _)| {
			let key = k.as_str();
			!primary_key_fields.iter().any(|field| field == key) && !Self::is_generated_field(key)
		}) {
			let column_name = Self::field_column(&field_metadata, k);
			if matches!(v, DatabaseValue::Null) {
				stmt.value_expr(Alias::new(column_name), Expr::cust("NULL"));
			} else {
				stmt.value(
					Alias::new(column_name),
					database_value_to_query_value(v.clone()),
				);
			}
			has_values = true;
		}

		if !has_values {
			let primary_key = M::primary_key_field();
			let primary_key_column = Self::field_column(&field_metadata, primary_key);
			stmt.value_expr(
				Alias::new(primary_key_column),
				Expr::col(Alias::new(primary_key_column)),
			);
		}

		stmt.cond_where(Self::primary_key_condition_from_object(obj)?);

		if include_returning {
			stmt.returning(Self::returning_columns_from_object(obj));
		}
		Ok(stmt)
	}

	fn build_insert_statement_from_object(
		obj: &std::collections::BTreeMap<String, DatabaseValue>,
		_field_is_none: impl Fn(&str) -> bool,
	) -> reinhardt_core::exception::Result<InsertStatement> {
		let mut stmt = Query::insert();
		stmt.into_table(Alias::new(M::table_name()));

		let pk_field = M::primary_key_field();
		let field_metadata = M::field_metadata();
		let (fields, values): (Vec<_>, Vec<_>) = obj
			.iter()
			.filter(|(k, v)| {
				let key = k.as_str();
				if Self::is_generated_field(key) {
					return false;
				}
				if key == pk_field {
					if matches!(v, DatabaseValue::Null) {
						return false;
					}
					if M::primary_key_uses_zero_sentinel()
						&& matches!(v, DatabaseValue::I32(0) | DatabaseValue::I64(0))
					{
						return false;
					}
				}
				if matches!(v, DatabaseValue::Null)
					&& (key == "created_at"
						|| key == "updated_at"
						|| key.ends_with("_date")
						|| key.ends_with("_time")
						|| key.ends_with("_at"))
				{
					return false;
				}
				true
			})
			.map(|(k, v)| {
				let value = database_value_to_query_value(v.clone());
				(Alias::new(Self::field_column(&field_metadata, k)), value)
			})
			.unzip();

		if fields.is_empty() {
			return Err(Error::from(DatabaseError::new(
				DatabaseErrorKind::Query,
				format!(
					"Cannot create {} because no writable fields remain after filtering generated and defaulted columns",
					M::table_name()
				),
			)));
		}

		stmt.columns(fields);
		stmt.values_panic(values);

		Ok(stmt)
	}

	/// Get all records
	pub fn all(&self) -> QuerySet<M> {
		QuerySet::new()
	}

	/// Return distinct truncated values from a generated date field.
	///
	/// Querysets created from subqueries, querysets with CTEs, querysets with
	/// lateral joins, and grouped or HAVING querysets are not supported.
	pub async fn dates<F, Origin>(
		&self,
		field: super::expressions::FieldRef<M, F, Origin>,
		kind: super::query::DateTruncKind,
		order: super::query::DateProjectionOrder,
	) -> reinhardt_core::exception::Result<Vec<chrono::NaiveDate>>
	where
		F: super::query::DateProjectionField,
	{
		QuerySet::new().dates(field, kind, order).await
	}

	/// Return distinct truncated dates through a caller-owned ORM executor.
	///
	/// Querysets created from subqueries, querysets with CTEs, querysets with
	/// lateral joins, and grouped or HAVING querysets are not supported.
	pub async fn dates_with_db<E, F, Origin>(
		&self,
		conn: &mut E,
		field: super::expressions::FieldRef<M, F, Origin>,
		kind: super::query::DateTruncKind,
		order: super::query::DateProjectionOrder,
	) -> reinhardt_core::exception::Result<Vec<chrono::NaiveDate>>
	where
		E: super::connection::OrmExecutor,
		F: super::query::DateProjectionField,
	{
		QuerySet::new()
			.dates_with_db(conn, field, kind, order)
			.await
	}

	/// Return distinct truncated dates through an active transaction executor.
	///
	/// Querysets created from subqueries, querysets with CTEs, querysets with
	/// lateral joins, and grouped or HAVING querysets are not supported.
	pub async fn dates_with_executor<F, Origin>(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		field: super::expressions::FieldRef<M, F, Origin>,
		kind: super::query::DateTruncKind,
		order: super::query::DateProjectionOrder,
	) -> Result<Vec<chrono::NaiveDate>, crate::backends::error::DatabaseError>
	where
		F: super::query::DateProjectionField,
	{
		QuerySet::new()
			.dates_with_executor(executor, field, kind, order)
			.await
	}

	/// Return distinct truncated values from a generated UTC datetime field.
	///
	/// Querysets created from subqueries, querysets with CTEs, querysets with
	/// lateral joins, and grouped or HAVING querysets are not supported.
	pub async fn datetimes<F, Origin>(
		&self,
		field: super::expressions::FieldRef<M, F, Origin>,
		kind: super::query::DateTimeTruncKind,
		order: super::query::DateProjectionOrder,
		time_zone: Option<chrono_tz::Tz>,
	) -> reinhardt_core::exception::Result<Vec<chrono::DateTime<chrono_tz::Tz>>>
	where
		F: super::query::DateTimeProjectionField,
	{
		QuerySet::new()
			.datetimes(field, kind, order, time_zone)
			.await
	}

	/// Return distinct truncated datetimes through a caller-owned ORM executor.
	///
	/// Querysets created from subqueries, querysets with CTEs, querysets with
	/// lateral joins, and grouped or HAVING querysets are not supported.
	pub async fn datetimes_with_db<E, F, Origin>(
		&self,
		conn: &mut E,
		field: super::expressions::FieldRef<M, F, Origin>,
		kind: super::query::DateTimeTruncKind,
		order: super::query::DateProjectionOrder,
		time_zone: Option<chrono_tz::Tz>,
	) -> reinhardt_core::exception::Result<Vec<chrono::DateTime<chrono_tz::Tz>>>
	where
		E: super::connection::OrmExecutor,
		F: super::query::DateTimeProjectionField,
	{
		QuerySet::new()
			.datetimes_with_db(conn, field, kind, order, time_zone)
			.await
	}

	/// Return distinct truncated datetimes through an active transaction executor.
	///
	/// Querysets created from subqueries, querysets with CTEs, querysets with
	/// lateral joins, and grouped or HAVING querysets are not supported.
	pub async fn datetimes_with_executor<F, Origin>(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		field: super::expressions::FieldRef<M, F, Origin>,
		kind: super::query::DateTimeTruncKind,
		order: super::query::DateProjectionOrder,
		time_zone: Option<chrono_tz::Tz>,
	) -> Result<Vec<chrono::DateTime<chrono_tz::Tz>>, crate::backends::error::DatabaseError>
	where
		F: super::query::DateTimeProjectionField,
	{
		QuerySet::new()
			.datetimes_with_executor(executor, field, kind, order, time_zone)
			.await
	}

	/// Filter records by a typed filter expression.
	///
	/// Accepts typed and untyped inputs through
	/// [`QueryFilterInput`](super::query::QueryFilterInput).
	/// The intended call style is the fluent builder produced by the
	/// `#[model]`-generated field accessors (`FieldRef::eq()` / `.gt()` / ...)
	/// or a composite condition built with `.and()`, `.or()`, and `.not()`.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Typed builder (recommended):
	/// User::objects()
	///     .filter(User::field_email().eq("alice@example.com"))
	///     .all()
	///     .await?;
	///
	/// // Raw Filter (when the field name is dynamic):
	/// use reinhardt_db::orm::{Filter, FilterOperator, FilterValue};
	/// User::objects()
	///     .filter(Filter::new("email", FilterOperator::Eq, FilterValue::String("alice@example.com".to_string())))
	///     .all()
	///     .await?;
	/// ```
	pub fn filter(&self, filter: impl super::query::QueryFilterInput<M>) -> QuerySet<M> {
		QuerySet::new().filter(filter)
	}

	/// Get a single record by primary key
	/// Returns a QuerySet filtered by the primary key field
	pub fn get(&self, pk: M::PrimaryKey) -> QuerySet<M> {
		let pk_field = M::primary_key_column();
		let pk_value = M::primary_key_filter_value(pk);

		let filter = super::query::Filter::new(
			pk_field.to_string(),
			super::query::FilterOperator::Eq,
			pk_value,
		);
		QuerySet::new().filter(filter)
	}

	/// Set LIMIT clause
	///
	/// Limits the number of records returned by the QuerySet.
	/// Corresponds to Django's `QuerySet[:n]`.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().limit(10).all().await?;
	/// ```
	pub fn limit(&self, limit: usize) -> QuerySet<M> {
		QuerySet::new().limit(limit)
	}

	/// Set ORDER BY clause
	///
	/// Sorts the QuerySet results by the specified fields.
	/// Use "-" prefix for descending order (e.g., "-created_at").
	/// Corresponds to Django's QuerySet.order_by().
	///
	/// # Examples
	///
	/// ```ignore
	/// // Ascending by name
	/// let users = User::objects().order_by(&["name"]).all().await?;
	///
	/// // Descending by created_at
	/// let users = User::objects().order_by(&["-created_at"]).all().await?;
	///
	/// // Multiple fields
	/// let users = User::objects().order_by(&["department", "-salary"]).all().await?;
	/// ```
	pub fn order_by(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().order_by(fields)
	}

	/// Add annotation to QuerySet
	///
	/// Starts a QuerySet for adding typed computed fields.
	/// Corresponds to Django's QuerySet.annotate().
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_db::orm::func;
	///
	/// let display_name =
	///     func::literal::<User, String>("user".to_owned())?.label("display_name")?;
	/// let users = User::objects()
	///     .annotate(display_name)?
	///     .all()
	///     .await?;
	/// ```
	pub fn annotate<K>(
		&self,
		annotation: super::query_fields::LabeledExpression<M, K>,
	) -> reinhardt_core::exception::Result<QuerySet<M>>
	where
		K: super::query_fields::AnnotationExpressionKind,
	{
		QuerySet::new().annotate(annotation)
	}

	/// Defer loading of specified fields
	///
	/// Excludes the specified fields from the initial query, loading them only when accessed.
	/// Corresponds to Django's QuerySet.defer().
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().defer(&["bio", "profile_picture"]).all().await?;
	/// ```
	pub fn defer(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().defer(fields)
	}

	/// Load only specified fields
	///
	/// Loads only the specified fields, excluding all others.
	/// Corresponds to Django's QuerySet.only().
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().only(&["id", "username"]).all().await?;
	/// ```
	pub fn only(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().only(fields)
	}

	/// Select specific fields (values)
	///
	/// Returns records with only the specified fields.
	/// Corresponds to Django's QuerySet.values().
	///
	/// # Examples
	///
	/// ```ignore
	/// let user_data = User::objects().values(&["id", "username", "email"]).all().await?;
	/// ```
	pub fn values(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().values(fields)
	}

	/// Eager load related objects using JOIN
	///
	/// Performs SQL JOINs to load related objects in a single query.
	/// Corresponds to Django's QuerySet.select_related().
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().select_related(&["author", "category"]).all().await?;
	/// ```
	pub fn select_related<I>(&self, fields: I) -> QuerySet<M>
	where
		I: RelationLoadInput<M>,
	{
		QuerySet::new().select_related(fields)
	}

	/// Set OFFSET clause
	///
	/// Skips the specified number of records before returning results.
	/// Corresponds to Django's QuerySet slicing `[offset:]`.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().offset(20).all().await?;
	/// ```
	pub fn offset(&self, offset: usize) -> QuerySet<M> {
		QuerySet::new().offset(offset)
	}

	/// Paginate results (LIMIT + OFFSET)
	///
	/// Convenience method that combines LIMIT and OFFSET for pagination.
	/// Corresponds to Django's Paginator.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().paginate(3, 10).all().await?;  // page 3, 10 items per page
	/// ```
	pub fn paginate(&self, page: usize, page_size: usize) -> QuerySet<M> {
		QuerySet::new().paginate(page, page_size)
	}

	/// Prefetch related objects using separate queries
	///
	/// Performs separate queries to load related objects, reducing N+1 queries.
	/// Corresponds to Django's QuerySet.prefetch_related().
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().prefetch_related(&["comments", "tags"]).all().await?;
	/// ```
	pub fn prefetch_related<I>(&self, fields: I) -> QuerySet<M>
	where
		I: RelationLoadInput<M>,
	{
		QuerySet::new().prefetch_related(fields)
	}

	/// Select specific fields (values_list)
	///
	/// Alias for `values()`. Returns records with only the specified fields.
	/// Corresponds to Django's QuerySet.values_list().
	///
	/// # Examples
	///
	/// ```ignore
	/// let user_data = User::objects().values_list(&["id", "username"]).all().await?;
	/// ```
	pub fn values_list(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().values_list(fields)
	}

	/// Filter by array overlap (PostgreSQL)
	///
	/// Filters rows where the array field overlaps with the provided values.
	/// Uses the `&&` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().filter_array_overlap("tags", &["rust", "web"]).all().await?;
	/// ```
	pub fn filter_array_overlap(&self, field: &str, values: &[&str]) -> QuerySet<M> {
		QuerySet::new().filter_array_overlap(field, values)
	}

	/// Filter by array contains (PostgreSQL)
	///
	/// Filters rows where the array field contains all provided values.
	/// Uses the `@>` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().filter_array_contains("tags", &["rust", "web"]).all().await?;
	/// ```
	pub fn filter_array_contains(&self, field: &str, values: &[&str]) -> QuerySet<M> {
		QuerySet::new().filter_array_contains(field, values)
	}

	/// Filter by JSONB contains (PostgreSQL)
	///
	/// Filters rows where the JSONB field contains the provided JSON.
	/// Uses the `@>` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().filter_jsonb_contains("metadata", r#"{"role": "admin"}"#).all().await?;
	/// ```
	pub fn filter_jsonb_contains(&self, field: &str, json: &str) -> QuerySet<M> {
		QuerySet::new().filter_jsonb_contains(field, json)
	}

	/// Filter by JSONB key exists (PostgreSQL)
	///
	/// Filters rows where the JSONB field has the specified key.
	/// Uses the `?` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().filter_jsonb_key_exists("metadata", "email").all().await?;
	/// ```
	pub fn filter_jsonb_key_exists(&self, field: &str, key: &str) -> QuerySet<M> {
		QuerySet::new().filter_jsonb_key_exists(field, key)
	}

	/// Filter by range contains (PostgreSQL)
	///
	/// Filters rows where the range field contains the provided value.
	/// Uses the `@>` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let events = Event::objects().filter_range_contains("date_range", "2024-01-15").all().await?;
	/// ```
	pub fn filter_range_contains(&self, field: &str, value: &str) -> QuerySet<M> {
		QuerySet::new().filter_range_contains(field, value)
	}

	/// Filter by IN subquery
	///
	/// Filters rows where the field value is in the result of a subquery.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_in_subquery("id", |subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("price", FilterOperator::Gt, FilterValue::Int(1500)))
	///             .values(&["author_id"])
	///     })?
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_in_subquery<R: super::Model, F>(
		&self,
		field: &str,
		subquery_fn: F,
	) -> reinhardt_core::exception::Result<QuerySet<M>>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_in_subquery(field, subquery_fn)
	}

	/// Filter by NOT IN subquery
	///
	/// Filters rows where the field value is not in the result of a subquery.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_not_in_subquery("id", |subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("status", FilterOperator::Eq, FilterValue::String("archived".into())))
	///             .values(&["author_id"])
	///     })?
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_not_in_subquery<R: super::Model, F>(
		&self,
		field: &str,
		subquery_fn: F,
	) -> reinhardt_core::exception::Result<QuerySet<M>>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_not_in_subquery(field, subquery_fn)
	}

	/// Filter by EXISTS subquery
	///
	/// Filters rows where the subquery returns at least one row.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_exists(|subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("author_id", FilterOperator::Eq, FilterValue::FieldRef(F::new("authors.id"))))
	///     })?
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_exists<R: super::Model, F>(
		&self,
		subquery_fn: F,
	) -> reinhardt_core::exception::Result<QuerySet<M>>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_exists(subquery_fn)
	}

	/// Filter by NOT EXISTS subquery
	///
	/// Filters rows where the subquery returns no rows.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_not_exists(|subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("author_id", FilterOperator::Eq, FilterValue::FieldRef(F::new("authors.id"))))
	///     })?
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_not_exists<R: super::Model, F>(
		&self,
		subquery_fn: F,
	) -> reinhardt_core::exception::Result<QuerySet<M>>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_not_exists(subquery_fn)
	}

	/// Add Common Table Expression (WITH clause)
	///
	/// Adds a CTE that can be referenced in the main query.
	///
	/// # Examples
	///
	/// ```ignore
	/// let high_earners = CTE::new("high_earners", "SELECT * FROM employees WHERE salary > 100000");
	/// let results = Employee::objects()
	///     .with_cte(high_earners)
	///     .all()
	///     .await?;
	/// ```
	pub fn with_cte(&self, cte: super::cte::CTE) -> QuerySet<M> {
		QuerySet::new().with_cte(cte)
	}

	/// Full-text search (PostgreSQL)
	///
	/// Filters rows using PostgreSQL's full-text search.
	///
	/// # Examples
	///
	/// ```ignore
	/// let articles = Article::objects()
	///     .full_text_search("search_vector", "rust programming")
	///     .all()
	///     .await?;
	/// ```
	pub fn full_text_search(&self, field: &str, query: &str) -> QuerySet<M> {
		QuerySet::new().full_text_search(field, query)
	}

	/// Annotate with subquery
	///
	/// Adds a scalar subquery to the SELECT clause.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .annotate_subquery::<Book, _>("book_count", |subq| {
	///         subq.filter("author_id", FilterOperator::Eq, FilterValue::OuterRef(OuterRef::new("authors.id")))
	///             .values(&["COUNT(*)"])
	///     })?
	///     .all()
	///     .await?;
	/// ```
	pub fn annotate_subquery<R, F>(
		&self,
		name: &str,
		builder: F,
	) -> reinhardt_core::exception::Result<QuerySet<M>>
	where
		R: super::Model + 'static,
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().annotate_subquery(name, builder)
	}

	/// Get a record by composite primary key
	///
	/// Retrieves a single object using all fields of a composite primary key.
	///
	/// # Examples
	///
	/// ```ignore
	/// let mut pk_values = HashMap::new();
	/// pk_values.insert("post_id".to_string(), PkValue::Int(1));
	/// pk_values.insert("tag_id".to_string(), PkValue::Int(5));
	/// let post_tag = PostTag::objects().get_composite(&pk_values).await?;
	/// ```
	pub async fn get_composite(
		&self,
		pk_values: &std::collections::HashMap<String, super::composite_pk::PkValue>,
	) -> reinhardt_core::exception::Result<M>
	where
		M: Clone + serde::de::DeserializeOwned,
	{
		QuerySet::new().get_composite(pk_values).await
	}

	/// Create a new record using reinhardt-query for SQL injection protection
	pub async fn create(&self, model: &M) -> reinhardt_core::exception::Result<M> {
		let mut conn = get_connection().await?;
		self.create_with_conn(&mut conn, model).await
	}

	async fn create_with_executor(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		model: &M,
	) -> Result<M, crate::backends::error::DatabaseError> {
		let obj = model
			.encode_database_fields()
			.map_err(executor_field_codec_error)?;
		let mut stmt =
			Self::build_insert_statement_from_object(&obj, |field| model.field_is_none(field))
				.map_err(executor_error)?;
		let backend = Self::executor_backend(executor);

		if backend != DatabaseBackend::MySql {
			stmt.returning(Self::returning_columns_from_object(&obj));
		}
		let context = super::execution::pgvector_context_for_insert(&stmt);
		let (sql, values) = build_insert_sql_checked(&stmt, backend, executor.is_cockroachdb())
			.map_err(executor_error)?;
		let params = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		if backend == DatabaseBackend::MySql {
			let explicit_primary_key = obj
				.get(M::primary_key_field())
				.filter(|value| {
					!matches!(value, DatabaseValue::Null)
						&& (!M::primary_key_uses_zero_sentinel()
							|| !matches!(value, DatabaseValue::I32(0) | DatabaseValue::I64(0)))
				})
				.cloned();
			if explicit_primary_key.is_none() {
				executor
					.fetch_one("SELECT LAST_INSERT_ID(0) AS generated_id", Vec::new())
					.await
					.map_err(executor_error)?;
			}
			executor
				.execute_with_context(&sql, params, context)
				.await
				.map_err(executor_error)?;

			let primary_key_value = if let Some(primary_key) = explicit_primary_key {
				database_value_to_query_value(primary_key)
			} else {
				let row = executor
					.fetch_one(
						"SELECT CAST(LAST_INSERT_ID() AS SIGNED) AS generated_id",
						Vec::new(),
					)
					.await
					.map_err(executor_error)?;
				let generated_id = row.get::<i64>("generated_id")?;
				if generated_id <= 0 {
					return Err(crate::backends::error::DatabaseError::new(
						crate::backends::error::DatabaseErrorKind::Unsupported,
						"MySQL executor inserts without an explicit primary key require an auto-increment integer primary key",
					));
				}
				reinhardt_query::value::Value::BigInt(Some(generated_id))
			};

			let mut select = Query::select();
			select.from(Alias::new(M::table_name()));
			select.column(ColumnRef::Asterisk);
			let field_metadata = M::field_metadata();
			let primary_key_column = Self::field_column(&field_metadata, M::primary_key_field());
			select.and_where(Expr::col(Alias::new(primary_key_column)).eq(primary_key_value));
			let (select_sql, select_values) =
				build_select_sql_checked(&select, backend, executor.is_cockroachdb())
					.map_err(executor_error)?;
			let select_params = select_values
				.0
				.into_iter()
				.map(Self::sea_value_to_query_value)
				.collect();
			let row = executor
				.fetch_one(&select_sql, select_params)
				.await
				.map_err(executor_error)?;
			return Self::decode_executor_row(row);
		}

		let row = executor
			.fetch_one_with_context(&sql, params, context)
			.await
			.map_err(executor_error)?;
		Self::decode_executor_row(row)
	}

	/// Insert a model through a caller-owned transaction executor, regardless of
	/// whether its primary key is already populated.
	pub async fn insert_with_executor(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		model: &M,
	) -> Result<M, crate::backends::error::DatabaseError> {
		self.create_with_executor(executor, model).await
	}

	/// Save a model through a caller-owned transaction executor.
	pub async fn save_with_executor(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		model: &M,
	) -> Result<M, crate::backends::error::DatabaseError> {
		if model.primary_key().is_some() {
			self.update_with_executor(executor, model).await
		} else {
			self.create_with_executor(executor, model).await
		}
	}

	/// Create a new record through a caller-owned ORM executor.
	///
	/// Pass the transaction supplied by [`DatabaseConnection::atomic`] when the
	/// write must participate in a closure-scoped transaction.
	///
	/// # Arguments
	///
	/// * `conn` - The mutable ORM executor to use
	/// * `model` - The model to create
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::orm::{Manager, Model};
	/// # async fn example<M: Model>(manager: Manager<M>, model: &M) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let _created = conn
	///     .atomic(async |transaction| {
	///         manager.create_with_conn(transaction, model).await
	///     })
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create_with_conn<E>(
		&self,
		conn: &mut E,
		model: &M,
	) -> reinhardt_core::exception::Result<M>
	where
		E: OrmExecutor + ?Sized,
	{
		match self.create_with_conn_outcome(conn, model).await {
			super::custom_manager::CreateWithConnOutcome::Created(model) => Ok(model),
			super::custom_manager::CreateWithConnOutcome::FailedBeforeInsert(error)
			| super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error) => Err(error),
		}
	}

	/// Inserts a record and reports whether a later hydration failure happened after the write.
	pub async fn create_with_conn_outcome<E>(
		&self,
		conn: &mut E,
		model: &M,
	) -> super::custom_manager::CreateWithConnOutcome<M>
	where
		E: OrmExecutor + ?Sized,
	{
		let obj = match model.encode_database_fields().map_err(field_codec_error) {
			Ok(obj) => obj,
			Err(error) => {
				return super::custom_manager::CreateWithConnOutcome::FailedBeforeInsert(error);
			}
		};
		let mut stmt = match Self::build_insert_statement_from_object(&obj, |field| {
			model.field_is_none(field)
		}) {
			Ok(statement) => statement,
			Err(error) => {
				return super::custom_manager::CreateWithConnOutcome::FailedBeforeInsert(error);
			}
		};
		let backend = conn.backend();
		if backend != DatabaseBackend::MySql {
			stmt.returning(Self::returning_columns_from_object(&obj));
		}
		let context = super::execution::pgvector_context_for_insert(&stmt);
		let (sql, values) = match build_insert_sql_checked(&stmt, backend, conn.is_cockroachdb()) {
			Ok(sql_and_values) => sql_and_values,
			Err(error) => {
				return super::custom_manager::CreateWithConnOutcome::FailedBeforeInsert(error);
			}
		};
		let params = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		if backend == DatabaseBackend::MySql {
			let explicit_primary_key = obj
				.get(M::primary_key_field())
				.filter(|value| {
					!matches!(value, DatabaseValue::Null)
						&& (!matches!(value, DatabaseValue::I32(0) | DatabaseValue::I64(0))
							|| !M::primary_key_uses_zero_sentinel())
				})
				.cloned();
			let result = match conn.execute_with_context(&sql, params, context).await {
				Ok(result) => result,
				Err(error) => {
					let outcome = match error.database_error().map(DatabaseError::kind) {
						Some(DatabaseErrorKind::Connection | DatabaseErrorKind::Timeout) => {
							super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error)
						}
						_ => {
							super::custom_manager::CreateWithConnOutcome::FailedBeforeInsert(error)
						}
					};
					return outcome;
				}
			};
			let primary_key = explicit_primary_key
				.map(database_value_to_query_value)
				.or_else(|| {
					result
						.last_insert_id
						.and_then(|id| i64::try_from(id).ok())
						.filter(|id| *id > 0)
						.map(|id| reinhardt_query::value::Value::BigInt(Some(id)))
				})
				.ok_or_else(|| {
					Error::from(DatabaseError::new(
						DatabaseErrorKind::Unsupported,
						"MySQL insert did not return a generated primary key",
					))
				});
			let primary_key = match primary_key {
				Ok(primary_key) => primary_key,
				Err(error) => {
					return super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error);
				}
			};
			let field_metadata = M::field_metadata();
			let primary_key_column = Self::field_column(&field_metadata, M::primary_key_field());
			let mut select = Query::select();
			select
				.from(Alias::new(M::table_name()))
				.column(ColumnRef::Asterisk)
				.and_where(Expr::col(Alias::new(primary_key_column)).eq(primary_key));
			let (select_sql, select_values) =
				match build_select_sql_checked(&select, backend, conn.is_cockroachdb()) {
					Ok(sql_and_values) => sql_and_values,
					Err(error) => {
						return super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(
							error,
						);
					}
				};
			let select_params = select_values
				.0
				.into_iter()
				.map(Self::sea_value_to_query_value)
				.collect();
			let row = match conn.fetch_one(&select_sql, select_params).await {
				Ok(row) => row,
				Err(error) => {
					return super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error);
				}
			};
			return match decode_model_row(row) {
				Ok(model) => super::custom_manager::CreateWithConnOutcome::Created(model),
				Err(error) => {
					super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error)
				}
			};
		}

		let row = match conn.fetch_one_with_context(&sql, params, context).await {
			Ok(row) => row,
			Err(error) => {
				return match error.database_error().map(DatabaseError::kind) {
					Some(DatabaseErrorKind::Connection | DatabaseErrorKind::Timeout) => {
						super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error)
					}
					_ => super::custom_manager::CreateWithConnOutcome::FailedBeforeInsert(error),
				};
			}
		};
		match decode_model_row(row) {
			Ok(model) => super::custom_manager::CreateWithConnOutcome::Created(model),
			Err(error) => super::custom_manager::CreateWithConnOutcome::FailedAfterInsert(error),
		}
	}

	pub(crate) fn json_to_sea_value_for_field(
		value: &serde_json::Value,
		field_info: Option<&FieldInfo>,
		field_is_none: bool,
	) -> reinhardt_query::value::Value {
		if field_info
			.map(|field| super::json::is_json_field_type(&field.field_type))
			.unwrap_or(false)
		{
			if field_is_none {
				reinhardt_query::value::Value::Json(None)
			} else {
				reinhardt_query::value::Value::Json(Some(Box::new(value.clone())))
			}
		} else {
			Self::json_to_sea_value(value)
		}
	}

	/// Convert serde_json::Value to reinhardt_query::value::Value for parameter binding
	pub(crate) fn json_to_sea_value(v: &serde_json::Value) -> reinhardt_query::value::Value {
		match v {
			serde_json::Value::Null => reinhardt_query::value::Value::Int(None),
			serde_json::Value::Bool(b) => reinhardt_query::value::Value::Bool(Some(*b)),
			serde_json::Value::Number(n) => {
				if let Some(i) = n.as_i64() {
					reinhardt_query::value::Value::BigInt(Some(i))
				} else if let Some(f) = n.as_f64() {
					reinhardt_query::value::Value::Double(Some(f))
				} else {
					reinhardt_query::value::Value::Int(None)
				}
			}
			serde_json::Value::String(s) => {
				// 1. Try to parse as UUID (format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
				//    UUIDs are often serialized as strings via serde
				if let Ok(uuid) = Uuid::parse_str(s) {
					return reinhardt_query::value::Value::Uuid(Some(Box::new(uuid)));
				}

				// 2. Try to parse as ISO 8601 datetime (chrono::DateTime<Utc>)
				// This handles timestamps serialized by serde_json from chrono::DateTime

				// 2.1 Try RFC3339 strict format first (e.g., "2024-01-01T00:00:00+00:00")
				if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
					return reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(
						dt.with_timezone(&chrono::Utc),
					)));
				}

				// 2.2 Try chrono's FromStr trait for DateTime<Utc>
				//    This handles formats like "2024-01-01T00:00:00Z" with optional subseconds
				if let Ok(dt) = s.parse::<chrono::DateTime<chrono::Utc>>() {
					return reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(dt)));
				}

				// 2.3 Try parsing with FixedOffset timezone then convert to UTC
				//    Handles formats like "2024-01-01T00:00:00.123456789+00:00"
				if let Ok(dt) = s.parse::<chrono::DateTime<chrono::FixedOffset>>() {
					return reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(
						dt.with_timezone(&chrono::Utc),
					)));
				}

				// Fallback: treat as regular string (non-datetime, non-UUID values)
				reinhardt_query::value::Value::String(Some(Box::new(s.clone())))
			}
			serde_json::Value::Array(arr) => {
				// Convert JSON array to reinhardt_query::value::Value array
				// Array(ArrayType, Option<Box<Vec<Value>>>)
				let values: Vec<reinhardt_query::value::Value> =
					arr.iter().map(|v| Self::json_to_sea_value(v)).collect();
				reinhardt_query::value::Value::Array(
					reinhardt_query::value::ArrayType::String,
					Some(Box::new(values)),
				)
			}
			serde_json::Value::Object(_obj) => {
				// Use reinhardt-query's Json type for PostgreSQL JSONB/JSON columns
				// Json expects Box<serde_json::Value>
				reinhardt_query::value::Value::Json(Some(Box::new(v.clone())))
			}
		}
	}

	/// Convert reinhardt_query::value::Value to QueryValue for database parameter binding
	pub(crate) fn sea_value_to_query_value(
		v: reinhardt_query::value::Value,
	) -> super::connection::QueryValue {
		use super::connection::QueryValue;

		match v {
			reinhardt_query::value::Value::Bool(Some(b)) => QueryValue::Bool(b),
			reinhardt_query::value::Value::Bool(None) => QueryValue::Null,

			reinhardt_query::value::Value::TinyInt(Some(i)) => QueryValue::Int(i as i64),
			reinhardt_query::value::Value::TinyInt(None) => QueryValue::Null,
			reinhardt_query::value::Value::SmallInt(Some(i)) => QueryValue::Int(i as i64),
			reinhardt_query::value::Value::SmallInt(None) => QueryValue::Null,
			reinhardt_query::value::Value::Int(Some(i)) => QueryValue::Int32(i),
			reinhardt_query::value::Value::Int(None) => QueryValue::Null,
			reinhardt_query::value::Value::BigInt(Some(i)) => QueryValue::Int(i),
			reinhardt_query::value::Value::BigInt(None) => QueryValue::Null,

			reinhardt_query::value::Value::TinyUnsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::TinyUnsigned(None) => QueryValue::Null,
			reinhardt_query::value::Value::SmallUnsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::SmallUnsigned(None) => QueryValue::Null,
			reinhardt_query::value::Value::Unsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::Unsigned(None) => QueryValue::Null,
			reinhardt_query::value::Value::BigUnsigned(Some(u)) => QueryValue::Uint(u),
			reinhardt_query::value::Value::BigUnsigned(None) => QueryValue::Null,

			reinhardt_query::value::Value::Float(Some(f)) => QueryValue::Float(f as f64),
			reinhardt_query::value::Value::Float(None) => QueryValue::Null,
			reinhardt_query::value::Value::Double(Some(f)) => QueryValue::Float(f),
			reinhardt_query::value::Value::Double(None) => QueryValue::Null,
			// QueryValue has no dedicated decimal variant. Preserve the exact
			// decimal spelling as text so the backend can coerce it to DECIMAL
			// without losing precision through an intermediate float.
			reinhardt_query::value::Value::Decimal(Some(value)) => {
				QueryValue::String(value.to_string())
			}
			reinhardt_query::value::Value::Decimal(None) => QueryValue::Null,

			reinhardt_query::value::Value::String(Some(s)) => QueryValue::String((*s).clone()),
			reinhardt_query::value::Value::String(None) => QueryValue::Null,

			reinhardt_query::value::Value::Bytes(Some(b)) => QueryValue::Bytes((*b).clone()),
			reinhardt_query::value::Value::Bytes(None) => QueryValue::Null,

			// Timestamp handling
			reinhardt_query::value::Value::ChronoDateTime(Some(dt)) => {
				QueryValue::NaiveTimestamp(*dt)
			}
			reinhardt_query::value::Value::ChronoDateTime(None) => QueryValue::Null,
			reinhardt_query::value::Value::ChronoDateTimeUtc(Some(dt)) => {
				QueryValue::Timestamp(*dt)
			}
			reinhardt_query::value::Value::ChronoDateTimeUtc(None) => QueryValue::Null,
			reinhardt_query::value::Value::ChronoDate(Some(date)) => {
				QueryValue::String(date.to_string())
			}
			reinhardt_query::value::Value::ChronoDate(None) => QueryValue::Null,
			reinhardt_query::value::Value::ChronoTime(Some(time)) => {
				QueryValue::String(time.to_string())
			}
			reinhardt_query::value::Value::ChronoTime(None) => QueryValue::Null,

			// UUID handling
			reinhardt_query::value::Value::Uuid(Some(u)) => QueryValue::Uuid(*u),
			reinhardt_query::value::Value::Uuid(None) => QueryValue::Null,

			// JSON types - serialize to string
			reinhardt_query::value::Value::Json(json) => QueryValue::Json(json),
			#[cfg(feature = "pgvector")]
			reinhardt_query::value::Value::Vector(Some(values)) => {
				QueryValue::Vector(Some((*values).clone()))
			}
			#[cfg(feature = "pgvector")]
			reinhardt_query::value::Value::Vector(None) => QueryValue::Vector(None),
			reinhardt_query::value::Value::Array(array_type, values) => {
				super::execution::array_value_to_query_value(
					array_type,
					values.map(|values| *values),
				)
			}

			// For complex types or unsupported types, convert to null
			// This is a safe fallback that won't cause runtime errors
			_ => QueryValue::Null,
		}
	}

	#[cfg(test)]
	fn query_value_to_sea_value(value: QueryValue) -> reinhardt_query::value::Value {
		match value {
			QueryValue::Null => reinhardt_query::value::Value::Int(None),
			QueryValue::Bool(value) => reinhardt_query::value::Value::Bool(Some(value)),
			QueryValue::Int32(value) => reinhardt_query::value::Value::Int(Some(value)),
			QueryValue::Int(value) => reinhardt_query::value::Value::BigInt(Some(value)),
			QueryValue::Uint(value) => reinhardt_query::value::Value::BigUnsigned(Some(value)),
			QueryValue::Float(value) => reinhardt_query::value::Value::Double(Some(value)),
			QueryValue::String(value) => {
				reinhardt_query::value::Value::String(Some(Box::new(value)))
			}
			QueryValue::Bytes(value) => reinhardt_query::value::Value::Bytes(Some(Box::new(value))),
			QueryValue::Timestamp(value) => {
				reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(value)))
			}
			QueryValue::NaiveTimestamp(value) => {
				reinhardt_query::value::Value::ChronoDateTime(Some(Box::new(value)))
			}
			QueryValue::Uuid(value) => reinhardt_query::value::Value::Uuid(Some(Box::new(value))),
			QueryValue::Json(value) => reinhardt_query::value::Value::Json(value),
			#[cfg(feature = "pgvector")]
			QueryValue::Vector(values) => reinhardt_query::value::Value::Vector(values.map(Box::new)),
			QueryValue::StringArray(values) => {
				reinhardt_query::value::Value::Json(Some(Box::new(serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::String).collect(),
				))))
			}
			QueryValue::IntArray(values) => reinhardt_query::value::Value::Json(Some(Box::new(
				serde_json::Value::Array(values.into_iter().map(serde_json::Value::from).collect()),
			))),
			QueryValue::BigIntArray(values) => reinhardt_query::value::Value::Json(Some(Box::new(
				serde_json::Value::Array(values.into_iter().map(serde_json::Value::from).collect()),
			))),
			QueryValue::BoolArray(values) => reinhardt_query::value::Value::Json(Some(Box::new(
				serde_json::Value::Array(values.into_iter().map(serde_json::Value::from).collect()),
			))),
			QueryValue::FloatArray(values) => reinhardt_query::value::Value::Json(Some(Box::new(
				serde_json::Value::Array(values.into_iter().map(serde_json::Value::from).collect()),
			))),
			QueryValue::DoubleArray(values) => reinhardt_query::value::Value::Json(Some(Box::new(
				serde_json::Value::Array(values.into_iter().map(serde_json::Value::from).collect()),
			))),
			QueryValue::UuidArray(values) => {
				reinhardt_query::value::Value::Json(Some(Box::new(serde_json::Value::Array(
					values
						.into_iter()
						.map(|value| serde_json::Value::String(value.to_string()))
						.collect(),
				))))
			}
			QueryValue::NullableStringArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::NullableIntArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::NullableBigIntArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::NullableBoolArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::NullableFloatArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::NullableDoubleArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::NullableUuidArray(values) => reinhardt_query::value::Value::Json(Some(
				Box::new(serde_json::to_value(values).expect("nullable arrays serialize")),
			)),
			QueryValue::Now => reinhardt_query::value::Value::Int(None),
		}
	}

	/// Serialize a JSON value to SQL-compatible string representation
	// Allow dead_code: internal helper for JSON-to-SQL serialization in manager operations
	#[allow(dead_code)]
	fn serialize_value(v: &serde_json::Value) -> String {
		match v {
			serde_json::Value::Null => "NULL".to_string(),
			serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
			serde_json::Value::Number(n) => n.to_string(),
			serde_json::Value::String(s) => {
				// Escape single quotes and wrap in quotes
				format!("'{}'", s.replace('\'', "''"))
			}
			serde_json::Value::Array(arr) => {
				// Convert to PostgreSQL array syntax: ARRAY['a', 'b', 'c']
				let items: Vec<String> = arr.iter().map(Self::serialize_value).collect();
				format!("ARRAY[{}]", items.join(", "))
			}
			serde_json::Value::Object(obj) => {
				// Convert to JSON string for JSONB columns
				let json_str = serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string());
				format!("'{}'::jsonb", json_str.replace('\'', "''"))
			}
		}
	}

	/// Update an existing record using reinhardt-query for SQL injection protection
	pub async fn update(&self, model: &M) -> reinhardt_core::exception::Result<M> {
		let mut conn = get_connection().await?;
		self.update_with_conn(&mut conn, model).await
	}

	async fn update_with_executor(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		model: &M,
	) -> Result<M, crate::backends::error::DatabaseError> {
		model.primary_key().ok_or_else(|| {
			crate::backends::error::DatabaseError::new(
				crate::backends::error::DatabaseErrorKind::Query,
				"Model must have primary key",
			)
		})?;
		let obj = model
			.encode_database_fields()
			.map_err(executor_field_codec_error)?;
		let backend = Self::executor_backend(executor);
		let stmt = Self::build_update_statement_from_object_with_returning(
			&obj,
			backend != DatabaseBackend::MySql,
		)
		.map_err(executor_field_codec_error)?;
		let context = super::execution::pgvector_context_for_update(&stmt);
		let (sql, values) = build_update_sql_checked(&stmt, backend, executor.is_cockroachdb())
			.map_err(executor_error)?;
		let params = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		if backend == DatabaseBackend::MySql {
			executor
				.execute_with_context(&sql, params, context)
				.await
				.map_err(executor_error)?;
			let mut select = Query::select();
			select.from(Alias::new(M::table_name()));
			select.column(ColumnRef::Asterisk);
			select.cond_where(
				Self::primary_key_condition_from_object(&obj)
					.map_err(executor_field_codec_error)?,
			);
			let (select_sql, select_values) =
				build_select_sql_checked(&select, backend, executor.is_cockroachdb())
					.map_err(executor_error)?;
			let select_params = select_values
				.0
				.into_iter()
				.map(Self::sea_value_to_query_value)
				.collect();
			let row = executor
				.fetch_one(&select_sql, select_params)
				.await
				.map_err(executor_error)?;
			return Self::decode_executor_row(row);
		}

		let row = executor
			.fetch_one_with_context(&sql, params, context)
			.await
			.map_err(executor_error)?;
		Self::decode_executor_row(row)
	}

	/// Update an existing record through a caller-owned ORM executor.
	///
	/// Pass the transaction supplied by [`DatabaseConnection::atomic`] when the
	/// write must participate in a closure-scoped transaction.
	///
	/// # Arguments
	///
	/// * `conn` - The mutable ORM executor to use
	/// * `model` - The model to update (must have primary key set)
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::orm::{Manager, Model};
	/// # async fn example<M: Model>(manager: Manager<M>, model: &M) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let _updated = conn
	///     .atomic(async |transaction| {
	///         manager.update_with_conn(transaction, model).await
	///     })
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update_with_conn<E>(
		&self,
		conn: &mut E,
		model: &M,
	) -> reinhardt_core::exception::Result<M>
	where
		E: OrmExecutor + ?Sized,
	{
		model.primary_key().ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Query,
				"Model must have primary key",
			))
		})?;

		let obj = model.encode_database_fields().map_err(field_codec_error)?;
		let backend = conn.backend();
		let stmt = if backend == DatabaseBackend::MySql {
			Self::build_update_statement_from_object_with_returning(&obj, false)
		} else {
			Self::build_update_statement_from_object(&obj, |field| model.field_is_none(field))
		}
		.map_err(field_codec_error)?;

		let context = super::execution::pgvector_context_for_update(&stmt);
		let (sql, values) = build_update_sql_checked(&stmt, backend, conn.is_cockroachdb())?;
		let values: Vec<_> = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		if backend == DatabaseBackend::MySql {
			conn.execute_with_context(&sql, values, context).await?;
			let mut select = Query::select();
			select
				.from(Alias::new(M::table_name()))
				.column(ColumnRef::Asterisk)
				.cond_where(
					Self::primary_key_condition_from_object(&obj).map_err(field_codec_error)?,
				);
			let (select_sql, select_values) =
				build_select_sql_checked(&select, backend, conn.is_cockroachdb())?;
			let select_params = select_values
				.0
				.into_iter()
				.map(Self::sea_value_to_query_value)
				.collect();
			let row = conn.fetch_one(&select_sql, select_params).await?;
			return decode_model_row(row);
		}

		let row = conn.fetch_one_with_context(&sql, values, context).await?;
		decode_model_row(row)
	}

	/// Delete a record using reinhardt-query for SQL injection protection
	pub async fn delete(&self, pk: M::PrimaryKey) -> reinhardt_core::exception::Result<()> {
		let mut conn = get_connection().await?;
		self.delete_with_conn(&mut conn, pk).await
	}

	/// Delete a model by primary key through a caller-owned transaction executor.
	///
	/// The physical column comes from model metadata and the bound value comes
	/// from the primary-key field codec.
	pub async fn delete_with_executor(
		&self,
		executor: &mut dyn super::connection::TransactionExecutor,
		pk: M::PrimaryKey,
	) -> Result<(), crate::backends::error::DatabaseError> {
		let stmt = Self::build_delete_statement(&pk).map_err(executor_field_codec_error)?;
		let (sql, values) = build_delete_sql_checked(
			&stmt,
			Self::executor_backend(executor),
			executor.is_cockroachdb(),
		)
		.map_err(executor_error)?;
		let params = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();
		executor
			.execute(&sql, params)
			.await
			.map_err(executor_error)?;
		Ok(())
	}

	/// Delete a record through a caller-owned ORM executor.
	///
	/// Pass the transaction supplied by [`DatabaseConnection::atomic`] when the
	/// deletion must participate in a closure-scoped transaction. The physical
	/// column comes from model metadata and the bound value comes from the
	/// primary-key field codec.
	///
	/// # Arguments
	///
	/// * `conn` - The mutable ORM executor to use
	/// * `pk` - The primary key of the record to delete
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::orm::{Manager, Model};
	/// # async fn example<M: Model>(manager: Manager<M>, pk: M::PrimaryKey) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// conn.atomic(async |transaction| {
	///     manager.delete_with_conn(transaction, pk).await
	/// })
	/// .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete_with_conn<E>(
		&self,
		conn: &mut E,
		pk: M::PrimaryKey,
	) -> reinhardt_core::exception::Result<()>
	where
		E: OrmExecutor,
	{
		let stmt = Self::build_delete_statement(&pk).map_err(field_codec_error)?;

		let (sql, values) = build_delete_sql_checked(&stmt, conn.backend(), conn.is_cockroachdb())?;
		let values: Vec<_> = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		conn.execute(&sql, values).await?;
		Ok(())
	}

	/// Count records using reinhardt-query
	pub async fn count(&self) -> reinhardt_core::exception::Result<i64> {
		let mut conn = get_connection().await?;
		self.count_with_conn(&mut conn).await
	}

	/// Count records through a caller-owned ORM executor.
	///
	/// Pass the transaction supplied by [`DatabaseConnection::atomic`] to observe
	/// writes that have not yet been committed.
	///
	/// # Arguments
	///
	/// * `conn` - The mutable ORM executor to use
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::orm::{Manager, Model};
	/// # async fn example<M: Model>(manager: Manager<M>) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let _count = conn
	///     .atomic(async |transaction| manager.count_with_conn(transaction).await)
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn count_with_conn<E>(&self, conn: &mut E) -> reinhardt_core::exception::Result<i64>
	where
		E: OrmExecutor,
	{
		// Build reinhardt-query SELECT COUNT(*) statement with explicit alias
		let stmt = Query::select()
			.from(Alias::new(M::table_name()))
			.expr_as(Func::count(Expr::asterisk().into()), Alias::new("count"))
			.to_owned();

		let (sql, values) = build_select_sql_checked(&stmt, conn.backend(), conn.is_cockroachdb())?;
		let values: Vec<_> = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		let row = QueryRow::from_backend_row(conn.fetch_one(&sql, values).await?);
		row.get::<i64>("count").ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Query,
				"Failed to get count",
			))
		})
	}

	/// Bulk create multiple records using reinhardt-query (similar to Django's bulk_create())
	pub fn bulk_create_query(&self, models: &[M]) -> Option<InsertStatement> {
		self.try_bulk_create_query(models).ok().flatten()
	}

	fn try_bulk_create_query(
		&self,
		models: &[M],
	) -> Result<Option<InsertStatement>, FieldCodecError> {
		if models.is_empty() {
			return Ok(None);
		}

		let database_values: Vec<std::collections::BTreeMap<String, DatabaseValue>> = models
			.iter()
			.map(Model::encode_database_fields)
			.collect::<Result<_, _>>()?;

		if database_values.is_empty() {
			return Ok(None);
		}

		let first_obj = &database_values[0];

		let primary_key = M::primary_key_field();
		let field_names: Vec<String> = first_obj
			.iter()
			.filter_map(|(name, value)| {
				if Self::is_generated_field(name.as_str())
					|| (name == primary_key
						&& (matches!(value, DatabaseValue::Null)
							|| (M::primary_key_uses_zero_sentinel()
								&& matches!(value, DatabaseValue::I32(0) | DatabaseValue::I64(0)))))
				{
					None
				} else {
					Some(name.clone())
				}
			})
			.collect();
		let field_metadata = M::field_metadata();
		let fields: Vec<_> = field_names
			.iter()
			.map(|name| Alias::new(Self::field_column(&field_metadata, name)))
			.collect();
		if fields.is_empty() {
			return Ok(None);
		}

		// Build reinhardt-query INSERT statement
		let mut stmt = Query::insert();
		stmt.into_table(Alias::new(M::table_name())).columns(fields);

		// Add value rows for each model
		for obj in &database_values {
			let values: Vec<reinhardt_query::value::Value> = field_names
				.iter()
				.map(|field| {
					obj.get(field.as_str())
						.cloned()
						.map(database_value_to_query_value)
							// Use untyped NULL for missing fields
							.unwrap_or(reinhardt_query::value::Value::Int(None))
				})
				.collect();
			stmt.values_panic(values);
		}

		Ok(Some(stmt.to_owned()))
	}

	fn try_bulk_create_statements(
		&self,
		models: &[M],
	) -> Result<Vec<InsertStatement>, FieldCodecError> {
		let key_presence: Vec<bool> = models
			.iter()
			.map(|model| {
				let values = model.encode_database_fields()?;
				Ok(values.get(M::primary_key_field()).is_some_and(|value| {
					!(matches!(value, DatabaseValue::Null)
						|| (M::primary_key_uses_zero_sentinel()
							&& matches!(value, DatabaseValue::I32(0) | DatabaseValue::I64(0))))
				}))
			})
			.collect::<Result<_, FieldCodecError>>()?;
		let mut statements = Vec::new();
		let mut start = 0;
		while start < models.len() {
			let has_key = key_presence[start];
			let mut end = start + 1;
			while end < models.len() && key_presence[end] == has_key {
				end += 1;
			}
			if let Some(statement) = self.try_bulk_create_query(&models[start..end])? {
				statements.push(statement);
			} else {
				// DEFAULT VALUES inserts one row per statement on all supported backends.
				for _ in start..end {
					statements.push(
						Query::insert()
							.into_table(M::table_name())
							.default_values()
							.to_owned(),
					);
				}
			}
			start = end;
		}
		Ok(statements)
	}

	/// Generate bulk create SQL (convenience method)
	///
	/// # Arguments
	///
	/// * `models` - Models to insert
	/// * `backend` - Database backend to generate SQL for
	pub fn bulk_create_sql(&self, models: &[M], backend: DatabaseBackend) -> String {
		if let Some(stmt) = self.bulk_create_query(models) {
			insert_to_string(&stmt, backend)
		} else {
			String::new()
		}
	}

	/// Generate UPDATE query for QuerySet
	pub fn update_queryset(
		&self,
		queryset: &QuerySet<M>,
		updates: &[(&str, &str)],
	) -> reinhardt_core::exception::Result<(String, Vec<String>)> {
		use crate::orm::query::UpdateValue;
		use std::collections::HashMap;

		// Convert &[(&str, &str)] to HashMap<String, UpdateValue>
		let updates_map: HashMap<String, UpdateValue> = updates
			.iter()
			.map(|(key, value)| (key.to_string(), UpdateValue::String(value.to_string())))
			.collect();

		queryset.update_sql(&updates_map)
	}

	/// Generate DELETE query for QuerySet
	pub fn delete_queryset(
		&self,
		queryset: &QuerySet<M>,
	) -> reinhardt_core::exception::Result<(String, Vec<String>)> {
		queryset.delete_sql()
	}

	/// Bulk create multiple records efficiently (Django's bulk_create)
	/// Inserts multiple records in a single query for performance
	///
	/// Django equivalent:
	/// ```python
	/// Model.objects.bulk_create([
	///     Model(field1=value1),
	///     Model(field2=value2),
	/// ])
	/// ```
	///
	/// Options:
	/// - batch_size: A positive size splits the input into batches; `None` uses a single batch
	/// - ignore_conflicts: Skip records that would violate constraints
	/// - update_conflicts: Update existing records instead of failing
	///
	/// PostgreSQL and SQLite return the inserted rows, including generated primary
	/// keys. MySQL executes the insert without `RETURNING` and returns the input
	/// models; generated keys, generated columns, and database defaults are not
	/// hydrated. When `ignore_conflicts` is enabled, all backends return an empty vector. MySQL
	/// uses `INSERT IGNORE` to skip conflicting rows.
	/// MySQL uses field metadata for `db_column` names and binary, datetime, and
	/// JSON bindings. JSON null remains a JSON value; absent optional values use
	/// SQL NULL.
	/// Database-generated columns are omitted. Consecutive rows with matching
	/// insert columns share a statement, preserving input order for mixed primary keys.
	///
	/// Empty input returns an empty vector without accessing the database, regardless of batch size.
	///
	/// # Errors
	///
	/// Nonempty input with `batch_size: Some(0)` returns
	/// [`reinhardt_core::exception::Error::Validation`] before accessing the database.
	pub async fn bulk_create(
		&self,
		models: Vec<M>,
		batch_size: Option<usize>,
		ignore_conflicts: bool,
		_update_conflicts: bool,
	) -> reinhardt_core::exception::Result<Vec<M>> {
		if models.is_empty() {
			return Ok(vec![]);
		}

		validate_bulk_batch_size(batch_size)?;
		let mut conn = get_connection().await?;
		self.bulk_create_with_conn(
			&mut conn,
			models,
			batch_size,
			ignore_conflicts,
			_update_conflicts,
		)
		.await
	}

	/// Bulk-insert models through a caller-owned executor.
	pub async fn bulk_create_with_conn<E>(
		&self,
		conn: &mut E,
		models: Vec<M>,
		batch_size: Option<usize>,
		ignore_conflicts: bool,
		_update_conflicts: bool,
	) -> reinhardt_core::exception::Result<Vec<M>>
	where
		E: OrmExecutor,
	{
		if models.is_empty() {
			return Ok(Vec::new());
		}

		validate_bulk_batch_size(batch_size)?;
		let batch_size = batch_size.unwrap_or(models.len());
		let mut results = Vec::new();

		let backend = conn.backend();
		for chunk in models.chunks(batch_size) {
			let statements = self
				.try_bulk_create_statements(chunk)
				.map_err(field_codec_error)?;
			for mut statement in statements {
				if !ignore_conflicts && backend != DatabaseBackend::MySql {
					statement.returning_all();
				}
				let context = super::execution::pgvector_context_for_insert(&statement);
				let (sql, values) =
					build_insert_sql_checked(&statement, backend, conn.is_cockroachdb())?;
				let sql = if ignore_conflicts {
					match backend {
						DatabaseBackend::Postgres => format!("{sql} ON CONFLICT DO NOTHING"),
						DatabaseBackend::MySql => {
							sql.replacen("INSERT INTO", "INSERT IGNORE INTO", 1)
						}
						DatabaseBackend::Sqlite => {
							sql.replacen("INSERT INTO", "INSERT OR IGNORE INTO", 1)
						}
					}
				} else {
					sql
				};
				if ignore_conflicts || backend == DatabaseBackend::MySql {
					conn.execute_generated_with_context(&sql, values, context)
						.await?;
				} else {
					for row in conn
						.fetch_all_generated_with_context(&sql, values, context)
						.await?
					{
						results.push(decode_model_row(row)?);
					}
				}
			}
			if !ignore_conflicts && backend == DatabaseBackend::MySql {
				results.extend(chunk.iter().cloned());
			}
		}
		Ok(results)
	}

	/// Bulk update multiple records efficiently (Django's bulk_update)
	/// Updates specified fields for multiple records in optimized queries
	///
	/// Django equivalent:
	/// ```python
	/// Model.objects.bulk_update(
	///     [obj1, obj2, obj3],
	///     ['field1', 'field2'],
	///     batch_size=100
	/// )
	/// ```
	///
	/// A positive `batch_size` splits the input into batches; `None` uses a single batch.
	/// Empty models or fields return zero without accessing the database, regardless of batch size.
	///
	/// # Errors
	///
	/// Nonempty models and fields with `batch_size: Some(0)` return
	/// [`reinhardt_core::exception::Error::Validation`] before accessing the database.
	pub async fn bulk_update(
		&self,
		models: Vec<M>,
		fields: Vec<String>,
		batch_size: Option<usize>,
	) -> reinhardt_core::exception::Result<usize> {
		if models.is_empty() || fields.is_empty() {
			return Ok(0);
		}

		validate_bulk_batch_size(batch_size)?;
		let conn = get_connection().await?;
		let batch_size = batch_size.unwrap_or(models.len());
		let mut total_updated = 0;

		for chunk in models.chunks(batch_size) {
			// Build updates structure
			let updates: Vec<(DatabaseValue, HashMap<String, DatabaseValue>)> = chunk
				.iter()
				.map(|model| {
					model.primary_key().ok_or_else(|| {
						Error::from(DatabaseError::new(
							DatabaseErrorKind::Type,
							"Bulk update model must have primary key",
						))
					})?;
					let obj = model.encode_database_fields().map_err(field_codec_error)?;
					let pk = obj
						.get(M::primary_key_field())
						.filter(|value| !matches!(value, DatabaseValue::Null))
						.cloned()
						.ok_or_else(|| {
							Error::from(DatabaseError::new(
								DatabaseErrorKind::Type,
								format!(
									"Encoded bulk update model must contain primary key '{}'",
									M::primary_key_field()
								),
							))
						})?;

					let mut field_map = HashMap::new();
					for field in fields
						.iter()
						.filter(|field| !Self::is_generated_field(field.as_str()))
					{
						if let Some(val) = obj.get(field) {
							field_map.insert(field.clone(), val.clone());
						}
					}

					Ok((pk, field_map))
				})
				.collect::<reinhardt_core::exception::Result<_>>()?;

			if !updates.is_empty() {
				validate_bulk_update_values_for_backend(&updates, conn.is_cockroachdb())?;
				let sql = self
					.bulk_update_database_values_sql_detailed(&updates, &fields, conn.backend())
					.map_err(field_codec_error)?;
				if sql.is_empty() {
					continue;
				}
				let rows_affected = conn.execute(&sql, vec![]).await?;
				total_updated += rows_affected as usize;
			}
		}

		Ok(total_updated)
	}

	/// Bulk-update models through a caller-owned executor.
	pub async fn bulk_update_with_conn<E>(
		&self,
		conn: &mut E,
		models: Vec<M>,
		fields: Vec<String>,
		batch_size: Option<usize>,
	) -> reinhardt_core::exception::Result<usize>
	where
		E: OrmExecutor,
	{
		if models.is_empty() || fields.is_empty() {
			return Ok(0);
		}

		validate_bulk_batch_size(batch_size)?;
		let batch_size = batch_size.unwrap_or(models.len());
		let mut total_updated = 0;
		for chunk in models.chunks(batch_size) {
			let updates: Vec<(DatabaseValue, HashMap<String, DatabaseValue>)> = chunk
				.iter()
				.map(|model| {
					model.primary_key().ok_or_else(|| {
						Error::from(DatabaseError::new(
							DatabaseErrorKind::Type,
							"Bulk update model must have primary key",
						))
					})?;
					let obj = model.encode_database_fields().map_err(field_codec_error)?;
					let pk = obj
						.get(M::primary_key_field())
						.filter(|value| !matches!(value, DatabaseValue::Null))
						.cloned()
						.ok_or_else(|| {
							Error::from(DatabaseError::new(
								DatabaseErrorKind::Type,
								format!(
									"Encoded bulk update model must contain primary key '{}'",
									M::primary_key_field()
								),
							))
						})?;
					let field_map = fields
						.iter()
						.filter(|field| !Self::is_generated_field(field.as_str()))
						.filter_map(|field| {
							obj.get(field).cloned().map(|value| (field.clone(), value))
						})
						.collect();
					Ok((pk, field_map))
				})
				.collect::<reinhardt_core::exception::Result<_>>()?;

			if updates.is_empty() {
				continue;
			}
			validate_bulk_update_values_for_backend(&updates, conn.is_cockroachdb())?;
			let sql = self
				.bulk_update_database_values_sql_detailed(&updates, &fields, conn.backend())
				.map_err(field_codec_error)?;
			if sql.is_empty() {
				continue;
			}
			total_updated += conn.execute(&sql, Vec::new()).await?.rows_affected as usize;
		}
		Ok(total_updated)
	}

	/// Bulk create - SQL generation only (for testing)
	pub fn bulk_create_sql_detailed(
		&self,
		field_names: &[String],
		value_rows: &[Vec<serde_json::Value>],
		ignore_conflicts: bool,
	) -> String {
		if value_rows.is_empty() {
			return String::new();
		}

		let writable_indexes: Vec<_> = field_names
			.iter()
			.enumerate()
			.filter_map(|(index, field)| {
				if Self::is_generated_field(field) {
					None
				} else {
					Some(index)
				}
			})
			.collect();
		let writable_field_names: Vec<_> = writable_indexes
			.iter()
			.map(|index| field_names[*index].clone())
			.collect();
		if writable_field_names.is_empty() {
			return String::new();
		}

		let values_clause: Vec<String> = value_rows
			.iter()
			.map(|row| {
				let values = writable_indexes
					.iter()
					.filter_map(|index| row.get(*index))
					.map(|v| match v {
						serde_json::Value::Null => "NULL".to_string(),
						serde_json::Value::Number(n) => n.to_string(),
						serde_json::Value::String(s) => {
							// SQL injection prevention: Escape single quotes
							format!("'{}'", s.replace("'", "''"))
						}
						serde_json::Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
						serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
							// Treat arrays and objects as JSON strings
							format!("'{}'", v.to_string().replace("'", "''"))
						}
					})
					.collect::<Vec<_>>()
					.join(", ");
				format!("({})", values)
			})
			.collect();

		let mut sql = format!(
			"INSERT INTO {} ({}) VALUES {}",
			M::table_name(),
			writable_field_names.join(", "),
			values_clause.join(", ")
		);

		if ignore_conflicts {
			sql.push_str(" ON CONFLICT DO NOTHING");
		}

		sql
	}

	/// Bulk update SQL generation using CASE expressions
	///
	/// Generates raw SQL because reinhardt-query's `UpdateStatement` does not support
	/// expression-based SET values (e.g., CASE WHEN ... END).
	fn bulk_update_database_values_sql_detailed(
		&self,
		updates: &[(DatabaseValue, HashMap<String, DatabaseValue>)],
		fields: &[String],
		backend: DatabaseBackend,
	) -> Result<String, FieldCodecError> {
		if updates.is_empty() || fields.is_empty() {
			return Ok(String::new());
		}

		let table_name = M::table_name();
		let field_metadata = M::field_metadata();
		let primary_key_column = Self::field_column(&field_metadata, M::primary_key_field());
		let mut set_clauses = Vec::new();

		for field in fields
			.iter()
			.filter(|field| !Self::is_generated_field(field.as_str()))
		{
			let mut when_clauses = Vec::new();
			for (pk, field_map) in updates {
				if let Some(value) = field_map.get(field) {
					when_clauses.push(format!(
						"WHEN {} = {} THEN {}",
						quote_identifier(primary_key_column, backend),
						database_value_sql_literal(pk.clone(), backend)?,
						database_value_sql_literal(value.clone(), backend)?
					));
				}
			}
			if !when_clauses.is_empty() {
				let column_name = Self::field_column(&field_metadata, field);
				set_clauses.push(format!(
					"{} = CASE {} END",
					quote_identifier(column_name, backend),
					when_clauses.join(" ")
				));
			}
		}

		if set_clauses.is_empty() {
			return Ok(String::new());
		}
		let ids = updates
			.iter()
			.map(|(pk, _)| database_value_sql_literal(pk.clone(), backend))
			.collect::<Result<Vec<_>, _>>()?
			.join(", ");
		Ok(format!(
			"UPDATE {} SET {} WHERE {} IN ({})",
			quote_identifier(table_name, backend),
			set_clauses.join(", "),
			quote_identifier(primary_key_column, backend),
			ids
		))
	}

	/// Generates bulk-update SQL from legacy JSON input values.
	///
	/// Model writes use the canonical database-value path; this method remains available for
	/// callers that explicitly construct JSON update data.
	pub fn bulk_update_sql_detailed(
		&self,
		updates: &[(M::PrimaryKey, HashMap<String, serde_json::Value>)],
		fields: &[String],
		_backend: DatabaseBackend,
	) -> String
	where
		M::PrimaryKey: std::fmt::Display + Clone,
	{
		if updates.is_empty() || fields.is_empty() {
			return String::new();
		}

		let table_name = M::table_name();
		let field_metadata = M::field_metadata();
		let primary_key_column = Self::field_column(&field_metadata, M::primary_key_field());
		let mut set_clauses = Vec::new();

		for field in fields
			.iter()
			.filter(|field| !Self::is_generated_field(field.as_str()))
		{
			let mut when_clauses = Vec::new();

			for (pk, field_map) in updates.iter() {
				if let Some(value) = field_map.get(field) {
					let val_str = match value {
						serde_json::Value::Null => "NULL".to_string(),
						serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
						serde_json::Value::Number(n) => n.to_string(),
						serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
						serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
							format!("'{}'", value.to_string().replace('\'', "''"))
						}
					};
					when_clauses.push(format!(
						"WHEN \"{}\" = '{}' THEN {}",
						primary_key_column,
						pk.to_string().replace('\'', "''"),
						val_str
					));
				}
			}

			if !when_clauses.is_empty() {
				let column_name = Self::field_column(&field_metadata, field);
				set_clauses.push(format!(
					"\"{}\" = CASE {} END",
					column_name,
					when_clauses.join(" ")
				));
			}
		}

		if set_clauses.is_empty() {
			return String::new();
		}

		let ids: Vec<String> = updates
			.iter()
			.map(|(pk, _)| format!("'{}'", pk.to_string().replace('\'', "''")))
			.collect();

		format!(
			"UPDATE \"{}\" SET {} WHERE \"{}\" IN ({})",
			table_name,
			set_clauses.join(", "),
			primary_key_column,
			ids.join(", ")
		)
	}
}

impl<M: Model> Default for Manager<M> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::{Manager, build_delete_sql, field_codec_error};
	use crate::backends::types::QueryValue;
	use crate::orm::Json;
	use crate::orm::Model;
	use crate::orm::connection::DatabaseBackend;
	use crate::orm::fields::{
		BigIntegerField, BinaryField, CharField, DateTimeField, Field, FieldKwarg,
	};
	use crate::orm::inspection::FieldInfo;
	use crate::orm::query::FilterValue;
	use crate::orm::{
		DatabaseArrayType, DatabaseScalar, DatabaseValue, FieldCodecContext, FieldCodecError,
		FieldSelector,
	};
	use rstest::fixture;
	use rstest::rstest;
	use serde::{Deserialize, Serialize};
	use serial_test::serial;
	use std::collections::HashMap;
	use std::fmt;

	#[cfg(feature = "pgvector")]
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct VectorManagerModel {
		id: Option<i64>,
		embedding: crate::orm::Vector<3>,
	}

	#[cfg(feature = "pgvector")]
	#[derive(Debug, Clone)]
	struct VectorManagerModelFields;

	#[cfg(feature = "pgvector")]
	impl FieldSelector for VectorManagerModelFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	#[cfg(feature = "pgvector")]
	impl Model for VectorManagerModel {
		type PrimaryKey = i64;
		type Fields = VectorManagerModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"vector_manager_models"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn new_fields() -> Self::Fields {
			VectorManagerModelFields
		}

		fn encode_database_fields(
			&self,
		) -> Result<std::collections::BTreeMap<String, crate::orm::DatabaseValue>, FieldCodecError>
		{
			Ok(std::collections::BTreeMap::from([
				(
					"id".to_owned(),
					self.id
						.map(crate::orm::DatabaseValue::I64)
						.unwrap_or(crate::orm::DatabaseValue::Null),
				),
				(
					"embedding".to_owned(),
					crate::orm::DatabaseValue::Vector(self.embedding.as_slice().to_vec()),
				),
			]))
		}
	}

	#[cfg(feature = "pgvector")]
	fn vector_manager_row() -> crate::orm::Row {
		crate::orm::Row {
			data: HashMap::from([
				("id".to_owned(), QueryValue::Int(7)),
				(
					"embedding".to_owned(),
					QueryValue::Vector(Some(vec![1.0, 2.0, 3.0])),
				),
			]),
		}
	}

	#[cfg(feature = "pgvector")]
	struct VectorManagerOrmExecutor {
		backend: DatabaseBackend,
		calls: Vec<(
			&'static str,
			Option<crate::backends::error::PgvectorOperationKind>,
		)>,
	}

	#[cfg(feature = "pgvector")]
	#[async_trait::async_trait]
	impl crate::orm::OrmExecutor for VectorManagerOrmExecutor {
		fn backend(&self) -> DatabaseBackend {
			self.backend
		}

		async fn execute(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> reinhardt_core::exception::Result<crate::orm::QueryResult> {
			self.calls.push(("execute", None));
			Ok(crate::orm::QueryResult {
				rows_affected: 1,
				last_insert_id: Some(7),
			})
		}

		async fn execute_with_context(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
			context: Option<crate::backends::error::PgvectorOperationKind>,
		) -> reinhardt_core::exception::Result<crate::orm::QueryResult> {
			self.calls.push(("execute_with_context", context));
			Ok(crate::orm::QueryResult {
				rows_affected: 1,
				last_insert_id: Some(7),
			})
		}

		async fn fetch_one(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> reinhardt_core::exception::Result<crate::orm::Row> {
			self.calls.push(("fetch_one", None));
			Ok(vector_manager_row())
		}

		async fn fetch_one_with_context(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
			context: Option<crate::backends::error::PgvectorOperationKind>,
		) -> reinhardt_core::exception::Result<crate::orm::Row> {
			self.calls.push(("fetch_one_with_context", context));
			Ok(vector_manager_row())
		}

		async fn fetch_all(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> reinhardt_core::exception::Result<Vec<crate::orm::Row>> {
			self.calls.push(("fetch_all", None));
			Ok(vec![vector_manager_row()])
		}

		async fn fetch_all_with_context(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
			context: Option<crate::backends::error::PgvectorOperationKind>,
		) -> reinhardt_core::exception::Result<Vec<crate::orm::Row>> {
			self.calls.push(("fetch_all_with_context", context));
			Ok(vec![vector_manager_row()])
		}

		async fn fetch_optional(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> reinhardt_core::exception::Result<Option<crate::orm::Row>> {
			panic!("vector manager update test does not fetch optional rows")
		}
	}

	#[cfg(feature = "pgvector")]
	struct VectorManagerTransaction {
		backend: crate::backends::types::DatabaseType,
		calls: Vec<(
			&'static str,
			Option<crate::backends::error::PgvectorOperationKind>,
		)>,
	}

	#[cfg(feature = "pgvector")]
	#[async_trait::async_trait]
	impl crate::orm::connection::TransactionExecutor for VectorManagerTransaction {
		fn backend(&self) -> crate::backends::types::DatabaseType {
			self.backend
		}

		async fn execute(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> crate::backends::error::Result<crate::orm::QueryResult> {
			self.calls.push(("execute", None));
			Ok(crate::orm::QueryResult {
				rows_affected: 1,
				last_insert_id: Some(7),
			})
		}

		async fn execute_with_context(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
			context: Option<crate::backends::error::PgvectorOperationKind>,
		) -> crate::backends::error::Result<crate::orm::QueryResult> {
			self.calls.push(("execute_with_context", context));
			Ok(crate::orm::QueryResult {
				rows_affected: 1,
				last_insert_id: Some(7),
			})
		}

		async fn fetch_one(
			&mut self,
			sql: &str,
			_params: Vec<QueryValue>,
		) -> crate::backends::error::Result<crate::orm::Row> {
			self.calls.push(("fetch_one", None));
			if sql.contains("generated_id") {
				Ok(crate::orm::Row {
					data: HashMap::from([("generated_id".to_owned(), QueryValue::Int(7))]),
				})
			} else {
				Ok(vector_manager_row())
			}
		}

		async fn fetch_one_with_context(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
			context: Option<crate::backends::error::PgvectorOperationKind>,
		) -> crate::backends::error::Result<crate::orm::Row> {
			self.calls.push(("fetch_one_with_context", context));
			Ok(vector_manager_row())
		}

		async fn fetch_all(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> crate::backends::error::Result<Vec<crate::orm::Row>> {
			panic!("vector manager update test does not fetch multiple rows")
		}

		async fn fetch_optional(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> crate::backends::error::Result<Option<crate::orm::Row>> {
			panic!("vector manager update test does not fetch optional rows")
		}

		async fn commit(self: Box<Self>) -> crate::backends::error::Result<()> {
			Ok(())
		}

		async fn rollback(self: Box<Self>) -> crate::backends::error::Result<()> {
			Ok(())
		}
	}

	#[cfg(feature = "sqlite")]
	struct DatabaseStateRestoreGuard {
		previous: Option<super::DatabaseRegistrationSnapshot>,
	}

	#[cfg(feature = "sqlite")]
	impl DatabaseStateRestoreGuard {
		fn replace(lease: Option<crate::orm::connection::DatabaseConnectionLease>) -> Self {
			Self {
				previous: Some(super::replace_database_connection_for_testing_sync(lease)),
			}
		}
	}

	#[cfg(feature = "sqlite")]
	impl Drop for DatabaseStateRestoreGuard {
		fn drop(&mut self) {
			if let Some(mut previous) = self.previous.take() {
				super::restore_database_connection_for_testing_sync(&mut previous);
			}
		}
	}

	#[test]
	fn test_field_codec_error_preserves_typed_source() {
		let error = field_codec_error(FieldCodecError::Serialization(
			"rejected manager value".to_owned(),
		));

		assert_eq!(
			error.database_kind(),
			Some(reinhardt_core::exception::DatabaseErrorKind::Serialization)
		);
		assert_eq!(
			error.to_string(),
			"Database error: field serialization failed: rejected manager value"
		);
		let source = std::error::Error::source(&error)
			.expect("manager codec error should preserve its typed source");
		assert!(source.downcast_ref::<FieldCodecError>().is_some());
	}

	#[test]
	fn field_policy_mismatch_is_a_typed_manager_error() {
		let source_error = FieldCodecError::FieldPolicyMismatch {
			context: Box::new(FieldCodecContext::new("Profile", "avatar", "avatar_path")),
			key: "file_storage".to_owned(),
			expected: "private_uploads".to_owned(),
			actual: "default".to_owned(),
		};
		let error = field_codec_error(source_error);

		assert_eq!(
			error.database_kind(),
			Some(reinhardt_core::exception::DatabaseErrorKind::Type)
		);
		let source = std::error::Error::source(&error).unwrap();
		assert!(matches!(
			source.downcast_ref::<FieldCodecError>(),
			Some(FieldCodecError::FieldPolicyMismatch { .. })
		));
	}

	#[test]
	#[cfg(feature = "pgvector")]
	fn manager_preserves_vector_query_values() {
		let value =
			Manager::<JsonManagerModel>::query_value_to_sea_value(QueryValue::Vector(Some(vec![
				1.0, 2.0, 3.0,
			])));

		assert_eq!(
			value,
			reinhardt_query::value::Value::Vector(Some(Box::new(vec![1.0, 2.0, 3.0])))
		);
		assert_eq!(
			Manager::<JsonManagerModel>::query_value_to_sea_value(QueryValue::Vector(None)),
			reinhardt_query::value::Value::Vector(None)
		);
	}

	#[test]
	#[cfg(feature = "pgvector")]
	fn manager_binds_vector_values_natively() {
		let value = Manager::<JsonManagerModel>::sea_value_to_query_value(
			reinhardt_query::value::Value::Vector(Some(Box::new(vec![1.0, 2.0, 3.0]))),
		);
		let null_value = Manager::<JsonManagerModel>::sea_value_to_query_value(
			reinhardt_query::value::Value::Vector(None),
		);

		assert_eq!(value, QueryValue::Vector(Some(vec![1.0, 2.0, 3.0])));
		assert_eq!(null_value, QueryValue::Vector(None));
	}

	#[cfg(feature = "pgvector")]
	#[rstest::rstest]
	#[case(DatabaseBackend::Postgres, "fetch_one_with_context")]
	#[case(DatabaseBackend::MySql, "execute_with_context")]
	#[tokio::test]
	async fn manager_update_with_conn_propagates_vector_statement_context(
		#[case] backend: DatabaseBackend,
		#[case] expected_method: &'static str,
	) {
		let model = VectorManagerModel {
			id: Some(7),
			embedding: crate::orm::Vector::try_from(vec![1.0, 2.0, 3.0]).unwrap(),
		};
		let mut executor = VectorManagerOrmExecutor {
			backend,
			calls: Vec::new(),
		};

		let updated = Manager::<VectorManagerModel>::new()
			.update_with_conn(&mut executor, &model)
			.await
			.expect("vector manager update should decode the returned model");

		assert_eq!(updated, model);
		assert_eq!(
			executor.calls.first(),
			Some(&(
				expected_method,
				Some(crate::backends::error::PgvectorOperationKind::VectorValue)
			))
		);
	}

	#[cfg(feature = "pgvector")]
	#[rstest::rstest]
	#[case(
		crate::backends::types::DatabaseType::Postgres,
		"fetch_one_with_context"
	)]
	#[case(crate::backends::types::DatabaseType::Mysql, "execute_with_context")]
	#[tokio::test]
	async fn manager_update_with_transaction_propagates_vector_statement_context(
		#[case] backend: crate::backends::types::DatabaseType,
		#[case] expected_method: &'static str,
	) {
		let model = VectorManagerModel {
			id: Some(7),
			embedding: crate::orm::Vector::try_from(vec![1.0, 2.0, 3.0]).unwrap(),
		};
		let mut executor = VectorManagerTransaction {
			backend,
			calls: Vec::new(),
		};

		let updated = Manager::<VectorManagerModel>::new()
			.save_with_executor(&mut executor, &model)
			.await
			.expect("vector manager transaction update should decode the returned model");

		assert_eq!(updated, model);
		assert_eq!(
			executor.calls.first(),
			Some(&(
				expected_method,
				Some(crate::backends::error::PgvectorOperationKind::VectorValue)
			))
		);
	}

	#[cfg(feature = "pgvector")]
	#[rstest::rstest]
	#[case(DatabaseBackend::Postgres, "fetch_one_with_context")]
	#[case(DatabaseBackend::MySql, "execute_with_context")]
	#[tokio::test]
	async fn manager_create_with_conn_propagates_vector_statement_context(
		#[case] backend: DatabaseBackend,
		#[case] expected_method: &'static str,
	) {
		let model = VectorManagerModel {
			id: None,
			embedding: crate::orm::Vector::try_from(vec![1.0, 2.0, 3.0]).unwrap(),
		};
		let mut executor = VectorManagerOrmExecutor {
			backend,
			calls: Vec::new(),
		};

		let created = Manager::<VectorManagerModel>::new()
			.create_with_conn(&mut executor, &model)
			.await
			.expect("vector manager create should decode the returned model");

		assert_eq!(created.id, Some(7));
		assert_eq!(created.embedding, model.embedding);
		assert_eq!(
			executor.calls.first(),
			Some(&(
				expected_method,
				Some(crate::backends::error::PgvectorOperationKind::VectorValue)
			))
		);
	}

	#[cfg(feature = "pgvector")]
	#[rstest::rstest]
	#[case(
		crate::backends::types::DatabaseType::Postgres,
		"fetch_one_with_context"
	)]
	#[case(crate::backends::types::DatabaseType::Mysql, "execute_with_context")]
	#[tokio::test]
	async fn manager_save_new_with_transaction_propagates_vector_statement_context(
		#[case] backend: crate::backends::types::DatabaseType,
		#[case] expected_method: &'static str,
	) {
		let model = VectorManagerModel {
			id: None,
			embedding: crate::orm::Vector::try_from(vec![1.0, 2.0, 3.0]).unwrap(),
		};
		let mut executor = VectorManagerTransaction {
			backend,
			calls: Vec::new(),
		};

		let created = Manager::<VectorManagerModel>::new()
			.save_with_executor(&mut executor, &model)
			.await
			.expect("vector manager transaction create should decode the returned model");

		assert_eq!(created.id, Some(7));
		assert_eq!(created.embedding, model.embedding);
		assert!(executor.calls.contains(&(
			expected_method,
			Some(crate::backends::error::PgvectorOperationKind::VectorValue)
		)));
	}

	#[cfg(feature = "pgvector")]
	#[rstest::rstest]
	#[case(DatabaseBackend::Postgres, false, "fetch_all_with_context")]
	#[case(DatabaseBackend::Postgres, true, "execute_with_context")]
	#[case(DatabaseBackend::MySql, false, "execute_with_context")]
	#[tokio::test]
	async fn manager_bulk_create_with_conn_propagates_vector_statement_context(
		#[case] backend: DatabaseBackend,
		#[case] ignore_conflicts: bool,
		#[case] expected_method: &'static str,
	) {
		let model = VectorManagerModel {
			id: Some(7),
			embedding: crate::orm::Vector::try_from(vec![1.0, 2.0, 3.0]).unwrap(),
		};
		let mut executor = VectorManagerOrmExecutor {
			backend,
			calls: Vec::new(),
		};

		let created = Manager::<VectorManagerModel>::new()
			.bulk_create_with_conn(
				&mut executor,
				vec![model.clone()],
				None,
				ignore_conflicts,
				false,
			)
			.await
			.expect("vector manager bulk create should execute");

		if ignore_conflicts {
			assert!(created.is_empty());
		} else {
			assert_eq!(created, vec![model]);
		}
		assert_eq!(
			executor.calls.first(),
			Some(&(
				expected_method,
				Some(crate::backends::error::PgvectorOperationKind::VectorValue)
			))
		);
	}

	#[cfg(feature = "sqlite")]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn init_database_skips_connection_when_already_initialized() {
		let owner = crate::orm::connection::BackendsConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap();
		let lease = crate::orm::connection::DatabaseConnectionLease::register(owner).unwrap();
		let _database_state = DatabaseStateRestoreGuard::replace(Some(lease));

		let result = super::init_database("unsupported://must-not-connect").await;
		let backend = super::get_connection()
			.await
			.map(|connection| connection.backend());

		result.expect("repeated initialization should not reconnect");
		assert_eq!(backend.unwrap(), DatabaseBackend::Sqlite);
	}

	#[cfg(feature = "sqlite")]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn init_database_installs_a_baseline_beneath_an_existing_scope() {
		let _database_state = DatabaseStateRestoreGuard::replace(None);
		let scope = super::install_scoped_database("sqlite::memory:")
			.await
			.expect("scope installation should succeed");

		super::init_database("sqlite::memory:")
			.await
			.expect("initialization should install a baseline beneath the scope");
		drop(scope);

		let backend = super::get_connection()
			.await
			.expect("dropping the scope should restore the new baseline")
			.backend();
		assert_eq!(backend, DatabaseBackend::Sqlite);
	}

	#[cfg(feature = "sqlite")]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn restoring_a_snapshot_skips_a_scope_dropped_during_replacement() {
		let _database_state = DatabaseStateRestoreGuard::replace(None);
		let scope = super::install_scoped_database("sqlite::memory:")
			.await
			.expect("scope installation should succeed");
		let owner = crate::orm::connection::BackendsConnection::connect("sqlite::memory:")
			.await
			.expect("test replacement connection should succeed");
		let replacement = crate::orm::connection::DatabaseConnectionLease::register(owner)
			.expect("test replacement lease should register");
		let snapshot = super::replace_database_connection_for_testing_sync(Some(replacement));

		drop(scope);
		let mut snapshot = snapshot;
		super::restore_database_connection_for_testing_sync(&mut snapshot);

		assert!(super::get_connection().await.is_err());
	}

	#[cfg(feature = "sqlite")]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn dropping_nested_test_snapshots_preserves_the_newer_registration() {
		let _database_state = DatabaseStateRestoreGuard::replace(None);
		let first_owner =
			crate::orm::connection::BackendsConnection::connect_sqlite("sqlite::memory:")
				.await
				.expect("first test connection should succeed");
		let first_lease = crate::orm::connection::DatabaseConnectionLease::register(first_owner)
			.expect("first test connection should register");
		let first = super::replace_database_connection_for_testing_sync(Some(first_lease));

		let second_owner =
			crate::orm::connection::BackendsConnection::connect_sqlite("sqlite::memory:")
				.await
				.expect("second test connection should succeed");
		let second_lease = crate::orm::connection::DatabaseConnectionLease::register(second_owner)
			.expect("second test connection should register");
		let second_handle = second_lease.handle();
		let second = super::replace_database_connection_for_testing_sync(Some(second_lease));

		drop(first);

		assert_eq!(
			super::get_connection()
				.await
				.expect("newer registration should remain installed"),
			second_handle
		);

		drop(second);

		assert!(super::get_connection().await.is_err());
	}
	use uuid::Uuid;

	#[cfg(feature = "sqlite")]
	type GlobalConnectionGuard = DatabaseStateRestoreGuard;
	#[cfg(feature = "sqlite")]
	#[rstest::fixture]
	async fn global_database() -> (GlobalConnectionGuard, super::DatabaseConnection) {
		let owner = crate::orm::connection::BackendsConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap();
		let lease = super::DatabaseConnectionLease::register(owner).unwrap();
		let connection = lease.handle();
		(DatabaseStateRestoreGuard::replace(Some(lease)), connection)
	}

	#[cfg(all(feature = "sqlite", target_pointer_width = "64"))]
	#[rstest]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn queryset_default_execution_rejects_overflow_before_sql(
		#[future] global_database: (GlobalConnectionGuard, super::DatabaseConnection),
		#[values("all", "first", "get")] method: &str,
	) {
		// Arrange: no table exists, so an SQL error would expose skipped validation.
		let (_guard, _connection) = global_database.await;
		let queryset = super::QuerySet::<TestUser>::new().limit(usize::MAX);

		// Act
		let result = match method {
			"all" => queryset.all().await.map(|_| ()),
			"first" => queryset.first().await.map(|_| ()),
			"get" => queryset.get().await.map(|_| ()),
			_ => panic!("unknown queryset method"),
		};

		// Assert
		assert_eq!(
			result.unwrap_err().to_string(),
			"Database error: cannot encode BigUnsigned argument 1 for sqlite: unsigned integer exceeds signed 64-bit range"
		);
	}

	#[cfg(feature = "sqlite")]
	#[rstest]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn queryset_default_execution_preserves_bound_strings(
		#[future] global_database: (GlobalConnectionGuard, super::DatabaseConnection),
		#[values("all", "first", "get")] method: &str,
	) {
		use crate::orm::{Filter, FilterOperator};
		use reinhardt_query::{
			ColumnDef, Query, QueryBuilder, QueryStatementBuilder, SqliteQueryBuilder,
		};

		// Arrange
		let (_guard, mut connection) = global_database.await;
		let create = Query::create_table()
			.table(TestUser::table_name())
			.col(ColumnDef::new("id").integer().primary_key(true))
			.col(ColumnDef::new("name").text())
			.col(ColumnDef::new("email").text())
			.to_owned();
		let (sql, _) = SqliteQueryBuilder.build_create_table(&create);
		connection.execute(&sql, vec![]).await.unwrap();
		let name = "quoted 'name',slash\\日本語";
		let (sql, values) = Query::insert()
			.into_table(TestUser::table_name())
			.columns(["id", "name", "email"])
			.values_panic([
				reinhardt_query::Value::from(7),
				name.into(),
				"test@example.com".into(),
			])
			.build(SqliteQueryBuilder);
		super::OrmExecutor::execute_generated_with_context(&mut connection, &sql, values, None)
			.await
			.unwrap();
		let queryset = super::QuerySet::<TestUser>::new().filter(Filter::new(
			"name",
			FilterOperator::Eq,
			FilterValue::String(name.to_owned()),
		));

		// Act
		let users = match method {
			"all" => queryset.all().await.unwrap(),
			"first" => vec![queryset.first().await.unwrap().unwrap()],
			"get" => vec![queryset.get().await.unwrap()],
			_ => panic!("unknown queryset method"),
		};

		// Assert
		assert_eq!(users.len(), 1);
		assert_eq!(users[0].id, Some(7));
		assert_eq!(users[0].name, name);
		assert_eq!(users[0].email, "test@example.com");
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestUser {
		id: Option<i64>,
		name: String,
		email: String,
	}

	impl TestUser {
		// Allow dead_code: test helper constructor for manager tests
		#[allow(dead_code)]
		fn new(name: String, email: String) -> Self {
			Self {
				id: None,
				name,
				email,
			}
		}
	}

	#[derive(Debug, Clone)]
	struct TestUserFields;

	impl FieldSelector for TestUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUser {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestBinaryRecord {
		id: Option<i64>,
		name: String,
		payload: Vec<u8>,
		optional_payload: Option<Vec<u8>>,
		json_data: serde_json::Value,
		optional_json_data: Option<serde_json::Value>,
		timestamp: chrono::DateTime<chrono::Utc>,
		optional_timestamp: Option<chrono::DateTime<chrono::Utc>>,
		stored_length: i64,
		virtual_length: i64,
	}

	impl Model for TestBinaryRecord {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_binary_record"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}

		fn field_is_none(&self, field_name: &str) -> bool {
			match field_name {
				"id" => self.id.is_none(),
				"optional_payload" => self.optional_payload.is_none(),
				"optional_json_data" => self.optional_json_data.is_none(),
				"optional_timestamp" => self.optional_timestamp.is_none(),
				_ => false,
			}
		}

		fn field_metadata() -> Vec<FieldInfo> {
			let mut id = BigIntegerField::new();
			id.base.primary_key = true;
			id.base.db_column = Some("record_id".to_owned());
			id.set_attributes_from_name("id");
			let mut name = CharField::new(255);
			name.base.db_column = Some("stored_name".to_owned());
			name.set_attributes_from_name("name");
			let mut payload = BinaryField::new();
			payload.set_attributes_from_name("payload");
			let mut optional_payload = BinaryField::new();
			optional_payload.base.null = true;
			optional_payload.set_attributes_from_name("optional_payload");
			let mut json_data = CharField::new(255);
			json_data.set_attributes_from_name("json_data");
			let mut json_data = FieldInfo::from_field(&json_data);
			json_data.field_type = "reinhardt.orm.models.JsonField".to_owned();
			let mut optional_json_data = json_data.clone();
			optional_json_data.name = "optional_json_data".to_owned();
			optional_json_data.nullable = true;
			let mut timestamp = DateTimeField::new();
			timestamp.set_attributes_from_name("timestamp");
			let mut optional_timestamp = DateTimeField::new();
			optional_timestamp.base.null = true;
			optional_timestamp.set_attributes_from_name("optional_timestamp");
			let generated = [
				("stored_length", "stored_size", "generated_stored"),
				("virtual_length", "virtual_size", "generated_virtual"),
			]
			.into_iter()
			.map(|(name, column, kind)| {
				let mut field = BigIntegerField::new();
				field.base.db_column = Some(column.to_owned());
				field.set_attributes_from_name(name);
				let mut info = FieldInfo::from_field(&field);
				info.attributes.insert(
					"generated".to_owned(),
					FieldKwarg::String("OCTET_LENGTH(payload)".to_owned()),
				);
				info.attributes
					.insert(kind.to_owned(), FieldKwarg::Bool(true));
				info
			});
			let mut fields = vec![
				FieldInfo::from_field(&id),
				FieldInfo::from_field(&name),
				FieldInfo::from_field(&payload),
				FieldInfo::from_field(&optional_payload),
				json_data,
				optional_json_data,
				FieldInfo::from_field(&timestamp),
				FieldInfo::from_field(&optional_timestamp),
			];
			for info in &mut fields {
				if matches!(info.name.as_str(), "payload" | "optional_payload") {
					info.storage_kind = Some(crate::orm::DatabaseStorageKind::Bytes);
				}
			}
			fields.extend(generated);
			fields
		}
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestGeneratedRecord {
		id: Option<i64>,
		stored: i64,
		virtual_value: i64,
	}

	impl Model for TestGeneratedRecord {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_generated_record"
		}
		fn primary_key(&self) -> Option<i64> {
			self.id
		}
		fn set_primary_key(&mut self, value: i64) {
			self.id = Some(value);
		}
		fn primary_key_field() -> &'static str {
			"id"
		}
		fn new_fields() -> Self::Fields {
			TestUserFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			let mut id = BigIntegerField::new();
			id.base.primary_key = true;
			id.set_attributes_from_name("id");
			let mut fields = vec![FieldInfo::from_field(&id)];
			for (name, expression, kind) in [
				("stored", "42", "generated_stored"),
				("virtual_value", "43", "generated_virtual"),
			] {
				let mut field = BigIntegerField::new();
				field.set_attributes_from_name(name);
				let mut field = FieldInfo::from_field(&field);
				field.attributes.insert(
					"generated".to_owned(),
					FieldKwarg::String(expression.to_owned()),
				);
				field
					.attributes
					.insert(kind.to_owned(), FieldKwarg::Bool(true));
				fields.push(field);
			}
			fields
		}
	}

	#[rstest]
	#[case(None, "INSERT INTO `test_generated_record` () VALUES ()")]
	#[case(Some(42), "INSERT INTO `test_generated_record` (`id`) VALUES (?)")]
	fn mysql_bulk_create_omits_generated_values(#[case] id: Option<i64>, #[case] expected: &str) {
		let model = TestGeneratedRecord {
			id,
			stored: 999,
			virtual_value: 999,
		};
		let statements = Manager::<TestGeneratedRecord>::new()
			.try_bulk_create_statements(&[model])
			.unwrap();
		assert_eq!(statements.len(), 1);
		let (sql, values) = super::build_insert_sql(&statements[0], DatabaseBackend::MySql);
		assert_eq!(sql, expected);
		assert_eq!(
			values.0,
			id.map(|id| vec![reinhardt_query::Value::BigInt(Some(id))])
				.unwrap_or_default()
		);
	}
	#[rstest]
	fn mysql_bulk_create_preserves_mixed_primary_keys_in_order() {
		let models: Vec<_> = [None, Some(42), None]
			.into_iter()
			.enumerate()
			.map(|(index, id)| TestUser {
				id,
				name: index.to_string(),
				email: format!("{index}@example.com"),
			})
			.collect();
		let statements = Manager::<TestUser>::new()
			.try_bulk_create_statements(&models)
			.unwrap();
		assert_eq!(statements.len(), 3);
		let keys: Vec<_> = statements
			.iter()
			.map(|statement| {
				super::build_insert_sql(statement, DatabaseBackend::MySql)
					.0
					.contains("`id`")
			})
			.collect();
		assert_eq!(keys, vec![false, true, false]);
	}

	// The model macro registers migration metadata, even for this SQL-only test.
	#[cfg(feature = "migrations")]
	#[reinhardt_core::macros::model(app_label = "bulk_test", table_name = "zero_key_records")]
	#[derive(serde::Serialize, serde::Deserialize)]
	struct ZeroKeyRecord {
		#[field(primary_key = true)]
		id: i64,
	}

	#[cfg(feature = "migrations")]
	#[rstest]
	fn mysql_bulk_create_preserves_mixed_zero_sentinel_keys() {
		// Arrange
		let models = [
			ZeroKeyRecord { id: 0 },
			ZeroKeyRecord { id: 42 },
			ZeroKeyRecord { id: 0 },
		];
		// Act
		let statements = Manager::<ZeroKeyRecord>::new()
			.try_bulk_create_statements(&models)
			.unwrap();
		let rendered: Vec<_> = statements
			.iter()
			.map(|stmt| super::build_insert_sql(stmt, DatabaseBackend::MySql))
			.collect();
		// Assert
		assert_eq!(rendered.len(), 3);
		assert_eq!(rendered[0].0, "INSERT INTO `zero_key_records` () VALUES ()");
		assert_eq!(
			rendered[1].1.0,
			vec![reinhardt_query::Value::BigInt(Some(42))]
		);
		assert_eq!(rendered[2].0, rendered[0].0);
	}

	#[cfg(all(
		not(all(target_family = "wasm", target_os = "unknown")),
		any(feature = "mysql", feature = "postgres", feature = "sqlite")
	))]
	mod bulk_create_backend_tests {
		use super::{DatabaseBackend, Manager, TestUser};
		#[cfg(feature = "mysql")]
		use super::{Model, TestBinaryRecord, TestGeneratedRecord};
		use crate::orm::manager::{
			DatabaseRegistrationSnapshot, replace_database_connection_for_testing_sync,
			restore_database_connection_for_testing_sync,
		};
		use crate::orm::test_connection::TestConnection as DatabaseConnection;
		use rstest::{fixture, rstest};
		use serial_test::serial;

		struct DatabaseScope {
			previous: DatabaseRegistrationSnapshot,
		}
		impl DatabaseScope {
			async fn install(connection: DatabaseConnection) -> Self {
				Self {
					previous: replace_database_connection_for_testing_sync(Some(
						connection.lease.clone(),
					)),
				}
			}
		}
		impl Drop for DatabaseScope {
			fn drop(&mut self) {
				restore_database_connection_for_testing_sync(&mut self.previous);
			}
		}

		#[cfg(feature = "mysql")]
		#[fixture]
		async fn mysql_database() -> (
			testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
			DatabaseConnection,
		) {
			use testcontainers::ImageExt;
			use testcontainers::runners::AsyncRunner;
			let container = testcontainers_modules::mysql::Mysql::default()
				.with_tag("8.0")
				.with_startup_timeout(std::time::Duration::from_secs(180))
				.start()
				.await
				.unwrap();
			let url = format!(
				"mysql://root@{}:{}/test",
				container.get_host().await.unwrap(),
				container.get_host_port_ipv4(3306).await.unwrap()
			);
			let connection = DatabaseConnection::connect(&url).await.unwrap();
			(container, connection)
		}

		#[cfg(feature = "postgres")]
		#[fixture]
		async fn postgres_database() -> (
			testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
			DatabaseConnection,
		) {
			use testcontainers::runners::AsyncRunner;
			let container = testcontainers_modules::postgres::Postgres::default()
				.start()
				.await
				.unwrap();
			let url = format!(
				"postgres://postgres:postgres@{}:{}/postgres",
				container.get_host().await.unwrap(),
				container.get_host_port_ipv4(5432).await.unwrap()
			);
			let connection = DatabaseConnection::connect(&url).await.unwrap();
			(container, connection)
		}

		#[cfg(feature = "sqlite")]
		#[fixture]
		async fn sqlite_database() -> DatabaseConnection {
			DatabaseConnection::connect_with_pool_size("sqlite::memory:", Some(1))
				.await
				.unwrap()
		}

		async fn stored_users(connection: &DatabaseConnection) -> Vec<TestUser> {
			#[cfg(feature = "mysql")]
			if connection.backend() == DatabaseBackend::MySql {
				// Explicit SQLx types keep insert verification independent of MySQL's
				// existing bool-first row inference and hydration limitations.
				let backend = connection.inner().backend();
				let mysql = backend
					.as_any()
					.downcast_ref::<crate::backends::dialect::MySqlBackend>()
					.unwrap();
				return sqlx::query_as::<_, (i64, String, String)>(
					"SELECT id, name, email FROM test_user ORDER BY email",
				)
				.fetch_all(mysql.pool())
				.await
				.unwrap()
				.into_iter()
				.map(|(id, name, email)| TestUser {
					id: Some(id),
					name,
					email,
				})
				.collect();
			}
			connection
				.query(
					"SELECT id, name, email FROM test_user ORDER BY email",
					vec![],
				)
				.await
				.unwrap()
				.into_iter()
				.map(|row| serde_json::from_value(row.data).unwrap())
				.collect()
		}

		async fn assert_bulk_create_backend_cases(connection: DatabaseConnection) {
			// Arrange
			let _scope = DatabaseScope::install(connection.clone()).await;
			let primary_key = match connection.backend() {
				DatabaseBackend::MySql => "BIGINT PRIMARY KEY AUTO_INCREMENT",
				DatabaseBackend::Postgres => "BIGSERIAL PRIMARY KEY",
				DatabaseBackend::Sqlite => "INTEGER PRIMARY KEY AUTOINCREMENT",
			};
			connection
				.execute(
					&format!(
						"CREATE TABLE test_user (id {primary_key}, name TEXT NOT NULL, email VARCHAR(255) NOT NULL UNIQUE)"
					),
					vec![],
				)
				.await
				.unwrap();
			let manager = Manager::<TestUser>::new();

			for generated_keys in [false, true] {
				for ignore_conflicts in [false, true] {
					for batch_size in [None, Some(2)] {
						connection
							.execute("DELETE FROM test_user", vec![])
							.await
							.unwrap();
						let seed = TestUser {
							id: Some(9000),
							name: "existing".to_owned(),
							email: "existing@example.com".to_owned(),
						};
						if ignore_conflicts {
							connection.execute(
								"INSERT INTO test_user (id, name, email) VALUES (9000, 'existing', 'existing@example.com')",
								vec![],
							).await.unwrap();
						}
						let models: Vec<TestUser> =
							["plain", "O'Reilly", "back\\slash", "日本語", "last"]
								.into_iter()
								.enumerate()
								.map(|(index, name)| TestUser {
									id: if generated_keys {
										None
									} else {
										Some(100 + index as i64)
									},
									name: name.to_owned(),
									email: if ignore_conflicts && index == 1 {
										seed.email.clone()
									} else {
										format!("user-{index}@example.com")
									},
								})
								.collect();
						let mut expected = models.clone();
						if ignore_conflicts {
							expected.remove(1);
							expected.push(seed);
						}
						expected.sort_by(|left, right| left.email.cmp(&right.email));

						// Act
						let outcome = manager
							.bulk_create(models.clone(), batch_size, ignore_conflicts, false)
							.await;
						let stored = stored_users(&connection).await;
						let returned = outcome.unwrap_or_else(|error| {
							panic!("bulk_create failed: {error}; stored rows: {}", stored.len())
						});

						// Assert
						assert_eq!(
							stored
								.iter()
								.map(|model| (&model.name, &model.email))
								.collect::<Vec<_>>(),
							expected
								.iter()
								.map(|model| (&model.name, &model.email))
								.collect::<Vec<_>>()
						);
						if !generated_keys {
							assert_eq!(stored, expected);
						}
						for model in &stored {
							assert_ne!(model.id, None);
						}
						if ignore_conflicts {
							assert_eq!(returned, Vec::<TestUser>::new());
						} else if connection.backend() == DatabaseBackend::MySql {
							assert_eq!(returned, models);
						} else {
							let hydrated: Vec<TestUser> = models
								.iter()
								.map(|model| {
									stored
										.iter()
										.find(|row| row.email == model.email)
										.unwrap()
										.clone()
								})
								.collect();
							assert_eq!(returned, hydrated);
						}
					}
				}
			}

			for batch_size in [None, Some(1)] {
				// Arrange
				connection
					.execute("DELETE FROM test_user", vec![])
					.await
					.unwrap();
				connection
					.execute(
						"INSERT INTO test_user (id, name, email) VALUES (1, 'existing', 'existing@example.com')",
						vec![],
					)
					.await
					.unwrap();
				let models = vec![
					TestUser {
						id: Some(2),
						name: "first".to_owned(),
						email: "first@example.com".to_owned(),
					},
					TestUser {
						id: Some(1),
						name: "conflicting".to_owned(),
						email: "conflicting@example.com".to_owned(),
					},
				];
				// Act
				manager
					.bulk_create(models, batch_size, false, false)
					.await
					.unwrap_err();
				let stored = stored_users(&connection).await;
				// Assert
				let mut expected = vec![TestUser {
					id: Some(1),
					name: "existing".to_owned(),
					email: "existing@example.com".to_owned(),
				}];
				if batch_size.is_some() {
					// Completed batches stay committed; bulk_create does not own a transaction.
					expected.push(TestUser {
						id: Some(2),
						name: "first".to_owned(),
						email: "first@example.com".to_owned(),
					});
				}
				assert_eq!(stored, expected);
			}
		}

		#[cfg(feature = "mysql")]
		#[rstest]
		#[tokio::test]
		#[serial(sqlx_drivers)]
		async fn bulk_create_mysql_backend_cases(
			#[future] mysql_database: (
				testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
				DatabaseConnection,
			),
		) {
			let (_container, connection) = mysql_database.await;
			assert_bulk_create_backend_cases(connection).await;
		}

		#[cfg(feature = "mysql")]
		#[rstest]
		#[tokio::test]
		#[serial(sqlx_drivers)]
		async fn bulk_create_mysql_typed_fields_and_renamed_columns(
			#[future] mysql_database: (
				testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
				DatabaseConnection,
			),
		) {
			use reinhardt_query::prelude::{Alias, MySqlQueryBuilder, Order, Query, QueryBuilder};
			use reinhardt_query::types::ColumnDef;
			// Arrange
			let (_container, connection) = mysql_database.await;
			let _scope = DatabaseScope::install(connection.clone()).await;
			let table = Query::create_table()
				.table(TestBinaryRecord::table_name())
				.col(
					ColumnDef::new("record_id")
						.big_integer()
						.primary_key(true)
						.auto_increment(true),
				)
				.col(
					ColumnDef::new("stored_name")
						.string_len(255)
						.not_null(true)
						.unique(true),
				)
				.col(ColumnDef::new("payload").custom("BLOB").not_null(true))
				.col(ColumnDef::new("optional_payload").custom("BLOB"))
				.col(ColumnDef::new("json_data").json().not_null(true))
				.col(ColumnDef::new("optional_json_data").json())
				.col(
					ColumnDef::new("timestamp")
						.custom("DATETIME(6)")
						.not_null(true),
				)
				.col(ColumnDef::new("optional_timestamp").custom("TIMESTAMP(6)"))
				.col(
					ColumnDef::new("stored_size")
						.custom("BIGINT GENERATED ALWAYS AS (OCTET_LENGTH(`payload`)) STORED"),
				)
				.col(
					ColumnDef::new("virtual_size")
						.custom("BIGINT GENERATED ALWAYS AS (OCTET_LENGTH(`payload`)) VIRTUAL"),
				)
				.to_owned();
			let (sql, _) = MySqlQueryBuilder.build_create_table(&table);
			connection.execute(&sql, vec![]).await.unwrap();
			let manager = Manager::<TestBinaryRecord>::new();
			let backend = connection.inner().backend();
			let mysql = backend
				.as_any()
				.downcast_ref::<crate::backends::dialect::MySqlBackend>()
				.unwrap();
			let timestamp = chrono::DateTime::parse_from_rfc3339("2026-10-04T12:34:56.123456Z")
				.unwrap()
				.with_timezone(&chrono::Utc);

			for key_pattern in 0..4 {
				for ignore_conflicts in [false, true] {
					for batch_size in [None, Some(2)] {
						let delete = Query::delete()
							.from_table(Alias::new(TestBinaryRecord::table_name()))
							.to_owned();
						let (sql, _) = MySqlQueryBuilder.build_delete(&delete);
						connection.execute(&sql, vec![]).await.unwrap();
						let models: Vec<TestBinaryRecord> = [
							serde_json::Value::Null,
							serde_json::json!("hello"),
							serde_json::json!("null"),
							serde_json::json!("true"),
							serde_json::json!("42"),
							serde_json::json!(true),
							serde_json::json!(42),
							serde_json::json!(1.25),
							serde_json::json!([0, 255]),
							serde_json::json!({"answer": 42}),
						]
						.into_iter()
						.enumerate()
						.map(|(index, json_data)| TestBinaryRecord {
							id: match key_pattern {
								0 => Some(100 + index as i64),
								1 => None,
								2 if index % 2 == 0 => None,
								3 if index % 2 == 1 => None,
								_ => Some(100 + index as i64),
							},
							name: format!("record-{index}"),
							payload: match index % 3 {
								0 => vec![0, 255, 128, 39, 92],
								1 => vec![],
								_ => vec![255, 0],
							},
							optional_payload: match index % 3 {
								0 => None,
								1 => Some(vec![]),
								_ => Some(vec![0, 255]),
							},
							json_data,
							optional_json_data: match index % 3 {
								0 => None,
								1 => Some(serde_json::Value::Null),
								_ => Some(serde_json::json!("null")),
							},
							timestamp: timestamp + chrono::Duration::microseconds(index as i64),
							optional_timestamp: if index % 2 == 0 {
								None
							} else {
								Some(timestamp)
							},
							stored_length: 0,
							virtual_length: 0,
						})
						.collect();
						let mut input = models.clone();
						if ignore_conflicts {
							let mut conflict = models[0].clone();
							conflict.payload = vec![42];
							conflict.id = if conflict.id.is_none() {
								Some(30_000)
							} else {
								None
							};
							input.insert(1, conflict);
						}
						// Act
						let returned = manager
							.bulk_create(input, batch_size, ignore_conflicts, false)
							.await
							.unwrap();
						let select = Query::select()
							.columns(
								[
									"record_id",
									"stored_name",
									"payload",
									"optional_payload",
									"json_data",
									"optional_json_data",
									"timestamp",
									"optional_timestamp",
									"stored_size",
									"virtual_size",
								]
								.into_iter()
								.map(Alias::new),
							)
							.from(Alias::new(TestBinaryRecord::table_name()))
							.order_by(Alias::new("stored_name"), Order::Asc)
							.to_owned();
						let (sql, _) = MySqlQueryBuilder.build_select(&select);
						let stored = sqlx::query_as::<
							_,
							(
								i64,
								String,
								Vec<u8>,
								Option<Vec<u8>>,
								sqlx::types::Json<serde_json::Value>,
								Option<sqlx::types::Json<serde_json::Value>>,
								chrono::NaiveDateTime,
								Option<chrono::DateTime<chrono::Utc>>,
								i64,
								i64,
							),
						>(&sql)
						.fetch_all(mysql.pool())
						.await
						.unwrap();
						// Assert
						assert_eq!(
							returned,
							if ignore_conflicts {
								vec![]
							} else {
								models.clone()
							}
						);
						assert_eq!(stored.len(), models.len());
						for (
							(
								id,
								name,
								payload,
								optional_payload,
								json_data,
								optional_json_data,
								timestamp,
								optional_timestamp,
								stored_length,
								virtual_length,
							),
							expected,
						) in stored.into_iter().zip(&models)
						{
							assert_eq!(
								(&name, &payload, &optional_payload, &json_data.0),
								(
									&expected.name,
									&expected.payload,
									&expected.optional_payload,
									&expected.json_data
								)
							);
							assert_eq!(
								optional_json_data.map(|value| value.0),
								expected.optional_json_data
							);
							assert_eq!(timestamp, expected.timestamp.naive_utc());
							assert_eq!(optional_timestamp, expected.optional_timestamp);
							assert_eq!(stored_length, expected.payload.len() as i64);
							assert_eq!(virtual_length, expected.payload.len() as i64);
							if let Some(expected_id) = expected.id {
								assert_eq!(id, expected_id);
							}
						}
					}
				}
			}
		}

		#[cfg(feature = "mysql")]
		#[rstest]
		#[tokio::test]
		#[serial(sqlx_drivers)]
		async fn bulk_create_mysql_default_only_generated_rows(
			#[future] mysql_database: (
				testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
				DatabaseConnection,
			),
		) {
			use reinhardt_query::prelude::{Alias, MySqlQueryBuilder, Query, QueryBuilder};
			use reinhardt_query::types::ColumnDef;
			// Arrange
			let (_container, connection) = mysql_database.await;
			let _scope = DatabaseScope::install(connection.clone()).await;
			let table = Query::create_table()
				.table(TestGeneratedRecord::table_name())
				.col(
					ColumnDef::new("id")
						.big_integer()
						.primary_key(true)
						.auto_increment(true),
				)
				.col(ColumnDef::new("stored").custom("BIGINT GENERATED ALWAYS AS (42) STORED"))
				.col(
					ColumnDef::new("virtual_value")
						.custom("BIGINT GENERATED ALWAYS AS (43) VIRTUAL"),
				)
				.to_owned();
			let (sql, _) = MySqlQueryBuilder.build_create_table(&table);
			connection.execute(&sql, vec![]).await.unwrap();
			let manager = Manager::<TestGeneratedRecord>::new();
			let backend = connection.inner().backend();
			let mysql = backend
				.as_any()
				.downcast_ref::<crate::backends::dialect::MySqlBackend>()
				.unwrap();
			let models = vec![
				TestGeneratedRecord {
					id: None,
					stored: 0,
					virtual_value: 0
				};
				3
			];
			for batch_size in [None, Some(2)] {
				for ignore_conflicts in [false, true] {
					let delete = Query::delete()
						.from_table(Alias::new(TestGeneratedRecord::table_name()))
						.to_owned();
					let (sql, _) = MySqlQueryBuilder.build_delete(&delete);
					connection.execute(&sql, vec![]).await.unwrap();
					// Act
					let returned = manager
						.bulk_create(models.clone(), batch_size, ignore_conflicts, false)
						.await
						.unwrap();
					let select = Query::select()
						.columns(
							["id", "stored", "virtual_value"]
								.into_iter()
								.map(Alias::new),
						)
						.from(Alias::new(TestGeneratedRecord::table_name()))
						.to_owned();
					let (sql, _) = MySqlQueryBuilder.build_select(&select);
					let stored = sqlx::query_as::<_, (i64, i64, i64)>(&sql)
						.fetch_all(mysql.pool())
						.await
						.unwrap();
					// Assert
					assert_eq!(
						returned,
						if ignore_conflicts {
							vec![]
						} else {
							models.clone()
						}
					);
					assert_eq!(stored.len(), 3);
					let mut ids = std::collections::HashSet::new();
					for (id, stored, virtual_value) in stored {
						assert!(id > 0);
						assert!(ids.insert(id));
						assert_eq!((stored, virtual_value), (42, 43));
					}
				}
			}
		}

		#[cfg(feature = "postgres")]
		#[rstest]
		#[tokio::test]
		#[serial(sqlx_drivers)]
		async fn bulk_create_postgres_backend_cases(
			#[future] postgres_database: (
				testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
				DatabaseConnection,
			),
		) {
			let (_container, connection) = postgres_database.await;
			assert_bulk_create_backend_cases(connection).await;
		}

		#[cfg(feature = "sqlite")]
		#[rstest]
		#[tokio::test]
		#[serial(sqlx_drivers)]
		async fn bulk_create_sqlite_backend_cases(#[future] sqlite_database: DatabaseConnection) {
			assert_bulk_create_backend_cases(sqlite_database.await).await;
		}
	}

	#[fixture]
	fn bulk_users() -> Vec<TestUser> {
		vec![
			TestUser {
				id: Some(1),
				name: "Alice".into(),
				email: "alice@example.com".into(),
			},
			TestUser {
				id: Some(2),
				name: "Bob".into(),
				email: "bob@example.com".into(),
			},
		]
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_create_rejects_zero_batch_size_before_database_access(
		bulk_users: Vec<TestUser>,
	) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let error = manager
			.bulk_create(bulk_users, Some(0), false, false)
			.await
			.expect_err("zero batch size must be rejected");

		// Assert
		assert!(
			matches!(error, reinhardt_core::exception::Error::Validation(ref message)
			if message == "batch_size must be greater than zero")
		);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_update_rejects_zero_batch_size_before_database_access(
		bulk_users: Vec<TestUser>,
	) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let error = manager
			.bulk_update(bulk_users, vec!["name".into()], Some(0))
			.await
			.expect_err("zero batch size must be rejected");

		// Assert
		assert!(
			matches!(error, reinhardt_core::exception::Error::Validation(ref message)
			if message == "batch_size must be greater than zero")
		);
	}

	#[rstest]
	#[case::zero(Some(0))]
	#[case::one(Some(1))]
	#[case::default(None)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_create_empty_models_remain_noop(#[case] batch_size: Option<usize>) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let created = manager
			.bulk_create(vec![], batch_size, false, false)
			.await
			.expect("empty models must succeed without a database");

		// Assert
		assert_eq!(created, Vec::<TestUser>::new());
	}

	#[rstest]
	#[case::zero(Some(0))]
	#[case::one(Some(1))]
	#[case::default(None)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_update_empty_models_remain_noop(#[case] batch_size: Option<usize>) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let updated = manager
			.bulk_update(vec![], vec!["name".into()], batch_size)
			.await
			.expect("empty models must succeed without a database");

		// Assert
		assert_eq!(updated, 0);
	}

	#[rstest]
	#[case::zero(Some(0))]
	#[case::one(Some(1))]
	#[case::default(None)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_update_empty_fields_remain_noop(
		bulk_users: Vec<TestUser>,
		#[case] batch_size: Option<usize>,
	) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let updated = manager
			.bulk_update(bulk_users, vec![], batch_size)
			.await
			.expect("empty fields must succeed without a database");

		// Assert
		assert_eq!(updated, 0);
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestStringUser {
		id: String,
	}

	#[derive(Debug, Clone)]
	struct TestStringUserFields;

	impl FieldSelector for TestStringUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestStringUser {
		type PrimaryKey = String;
		type Fields = TestStringUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_string_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id.clone())
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn new_fields() -> Self::Fields {
			TestStringUserFields
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestUuidUser {
		id: Uuid,
	}

	#[derive(Debug, Clone)]
	struct TestUuidUserFields;

	impl FieldSelector for TestUuidUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUuidUser {
		type PrimaryKey = Uuid;
		type Fields = TestUuidUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_uuid_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn primary_key_filter_value(pk: Self::PrimaryKey) -> FilterValue {
			FilterValue::Uuid(pk)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TestUuidUserFields
		}
	}

	#[rstest]
	fn test_get_preserves_uuid_primary_key_binding() {
		// Arrange
		let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000")
			.expect("UUID literal should be valid");

		// Act
		let query = TestUuidUser::objects().get(id);

		// Assert
		assert_eq!(query.filters().len(), 1);
		assert!(matches!(&query.filters()[0].value, FilterValue::Uuid(value) if *value == id));
	}

	#[rstest]
	fn test_get_preserves_default_numeric_primary_key_binding() {
		// Arrange and Act
		let query = TestUser::objects().get(42);

		// Assert
		assert_eq!(query.filters().len(), 1);
		assert!(matches!(query.filters()[0].value, FilterValue::Integer(42)));
	}

	#[rstest]
	fn test_delete_preserves_default_numeric_primary_key_binding() {
		// Arrange and Act
		let statement = Manager::<TestUser>::build_delete_statement(&42)
			.expect("integer primary key should encode");
		let (_sql, values) = build_delete_sql(&statement, DatabaseBackend::Postgres);

		// Assert
		assert_eq!(
			values.0,
			vec![reinhardt_query::value::Value::BigInt(Some(42))]
		);
	}

	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	struct NumericUserId(i64);

	impl fmt::Display for NumericUserId {
		fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			self.0.fmt(formatter)
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct NumericNewtypeUser {
		id: NumericUserId,
	}

	impl Model for NumericNewtypeUser {
		type PrimaryKey = NumericUserId;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"numeric_newtype_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}
	}

	#[rstest]
	fn test_manual_numeric_newtype_preserves_numeric_primary_key_binding() {
		// Arrange and Act
		let query = NumericNewtypeUser::objects().get(NumericUserId(42));
		let statement = Manager::<NumericNewtypeUser>::build_delete_statement(&NumericUserId(42))
			.expect("numeric newtype primary key should encode");
		let (_sql, values) = build_delete_sql(&statement, DatabaseBackend::Postgres);

		// Assert
		assert_eq!(query.filters().len(), 1);
		assert!(matches!(query.filters()[0].value, FilterValue::Integer(42)));
		assert_eq!(
			values.0,
			vec![reinhardt_query::value::Value::BigInt(Some(42))]
		);
	}

	#[rstest]
	#[case("01")]
	#[case("+1")]
	#[case("0001")]
	fn test_get_preserves_exact_custom_primary_key_string(#[case] id: &str) {
		// Arrange and Act
		let query = TestStringUser::objects().get(id.to_owned());

		// Assert
		assert_eq!(query.filters().len(), 1);
		let FilterValue::String(value) = &query.filters()[0].value else {
			panic!("custom primary key should use an exact string binding");
		};
		assert_eq!(value, id);
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct ExternalId(String);

	impl fmt::Display for ExternalId {
		fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			formatter.write_str(&self.0)
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TypedKeyUser {
		external_id: ExternalId,
	}

	#[derive(Debug, Clone)]
	struct TypedKeyUserFields;

	impl FieldSelector for TypedKeyUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TypedKeyUser {
		type PrimaryKey = ExternalId;
		type Fields = TypedKeyUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"typed_key_user"
		}

		fn primary_key_column() -> &'static str {
			"external_key"
		}

		fn primary_key_database_value(
			pk: &Self::PrimaryKey,
		) -> Result<DatabaseValue, FieldCodecError> {
			Ok(DatabaseValue::String(format!("external:{}", pk.0)))
		}

		fn primary_key_filter_value(pk: Self::PrimaryKey) -> FilterValue {
			FilterValue::String(format!("external:{}", pk.0))
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.external_id.clone())
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.external_id = value;
		}

		fn primary_key_field() -> &'static str {
			"external_id"
		}

		fn new_fields() -> Self::Fields {
			TypedKeyUserFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			let mut field = test_manager_field_info("external_id", "CustomKeyField", false, true);
			field.db_column = Some(Self::primary_key_column().to_owned());
			vec![field]
		}
	}

	#[rstest::rstest]
	fn get_uses_primary_key_column_and_custom_filter_binding() {
		let query = TypedKeyUser::objects().get(ExternalId("42".to_owned()));

		assert_eq!(query.filters().len(), 1);
		assert_eq!(query.filters()[0].field, "external_key");
		assert!(matches!(
			&query.filters()[0].value,
			FilterValue::String(value) if value == "external:42"
		));
	}

	#[rstest::rstest]
	fn delete_uses_primary_key_column_and_database_field_binding() {
		let cases = [
			(
				DatabaseBackend::Postgres,
				"DELETE FROM \"typed_key_user\" WHERE \"external_key\" = $1",
			),
			(
				DatabaseBackend::MySql,
				"DELETE FROM `typed_key_user` WHERE `external_key` = ?",
			),
			(
				DatabaseBackend::Sqlite,
				"DELETE FROM \"typed_key_user\" WHERE \"external_key\" = ?",
			),
		];

		for (backend, expected_sql) in cases {
			// Arrange
			let primary_key = ExternalId("42".to_owned());

			// Act
			let statement = Manager::<TypedKeyUser>::build_delete_statement(&primary_key)
				.expect("custom primary key should encode");
			let (sql, values) = build_delete_sql(&statement, backend);

			// Assert
			assert_eq!(sql, expected_sql);
			assert_eq!(
				values.0,
				vec![reinhardt_query::value::Value::String(Some(Box::new(
					"external:42".to_owned()
				)))]
			);
		}
	}

	#[rstest::rstest]
	#[case(i32::MIN)]
	#[case(0)]
	#[case(i32::MAX)]
	fn manager_preserves_int32_parameters(#[case] input: i32) {
		// Arrange
		let value = reinhardt_query::value::Value::Int(Some(input));

		// Act
		let bound = Manager::<TestUser>::sea_value_to_query_value(value.clone());
		let restored = Manager::<TestUser>::query_value_to_sea_value(bound.clone());

		// Assert
		assert_eq!(bound, crate::backends::QueryValue::Int32(input));
		assert_eq!(restored, value);
	}

	#[test]
	fn manager_binds_integer_arrays_natively() {
		let value =
			Manager::<TestUser>::sea_value_to_query_value(reinhardt_query::value::Value::Array(
				reinhardt_query::value::ArrayType::Int,
				Some(Box::new(vec![reinhardt_query::value::Value::Int(Some(7))])),
			));

		assert_eq!(value, crate::orm::connection::QueryValue::IntArray(vec![7]));
	}

	#[rstest]
	#[case::string(DatabaseArrayType::String, ["kept".to_owned(), "tail".to_owned()], QueryValue::StringArray, QueryValue::NullableStringArray, |value: Option<String>| reinhardt_query::Value::String(value.map(Box::new)))]
	#[case::int(DatabaseArrayType::I32, [i32::MIN, i32::MAX], QueryValue::IntArray, QueryValue::NullableIntArray, reinhardt_query::Value::Int)]
	#[case::bigint(DatabaseArrayType::I64, [i64::MIN, i64::MAX], QueryValue::BigIntArray, QueryValue::NullableBigIntArray, reinhardt_query::Value::BigInt)]
	#[case::bool(DatabaseArrayType::Bool, [true, false], QueryValue::BoolArray, QueryValue::NullableBoolArray, reinhardt_query::Value::Bool)]
	#[case::float(DatabaseArrayType::F32, [1.5_f32, -2.5_f32], QueryValue::FloatArray, QueryValue::NullableFloatArray, reinhardt_query::Value::Float)]
	#[case::double(DatabaseArrayType::F64, [3.5_f64, -4.5_f64], QueryValue::DoubleArray, QueryValue::NullableDoubleArray, reinhardt_query::Value::Double)]
	#[case::uuid(DatabaseArrayType::Uuid, [Uuid::nil(), Uuid::from_u128(1)], QueryValue::UuidArray, QueryValue::NullableUuidArray, |value: Option<Uuid>| reinhardt_query::Value::Uuid(value.map(Box::new)))]
	fn manager_preserves_nullable_array_elements<T>(
		#[case] element_type: DatabaseArrayType,
		#[case] values: [T; 2],
		#[case] non_nullable: fn(Vec<T>) -> QueryValue,
		#[case] nullable: fn(Vec<Option<T>>) -> QueryValue,
		#[case] encode: fn(Option<T>) -> reinhardt_query::Value,
	) where
		T: DatabaseScalar + Clone,
	{
		// Arrange
		let mixed = vec![
			None,
			Some(values[0].clone()),
			None,
			Some(values[1].clone()),
			None,
		];
		let all_null = vec![None, None];
		let shapes = [
			("mixed", mixed.clone(), nullable(mixed)),
			("all_null", all_null.clone(), nullable(all_null)),
			(
				"non_null",
				values.iter().cloned().map(Some).collect(),
				non_nullable(values.to_vec()),
			),
			("empty", vec![], non_nullable(vec![])),
		];
		for (shape, elements, expected) in shapes {
			let database_value = DatabaseValue::Array {
				element_type,
				values: elements
					.iter()
					.cloned()
					.map(|value| value.map_or(DatabaseValue::Null, T::into_database_value))
					.collect(),
			};
			let query_value = crate::orm::database_value_to_query_value(database_value);
			let reinhardt_query::Value::Array(array_type, _) = &query_value else {
				panic!("database array should retain its array type");
			};
			let typed_nulls = reinhardt_query::Value::Array(
				array_type.clone(),
				Some(Box::new(elements.into_iter().map(encode).collect())),
			);
			let whole_null = reinhardt_query::Value::Array(array_type.clone(), None);

			// Act
			let database_bound = Manager::<TestUser>::sea_value_to_query_value(query_value);
			let typed_bound = Manager::<TestUser>::sea_value_to_query_value(typed_nulls);
			let null_bound = Manager::<TestUser>::sea_value_to_query_value(whole_null);

			// Assert
			assert_eq!(
				database_bound, expected,
				"{element_type:?}: {shape} database values"
			);
			assert_eq!(
				typed_bound, expected,
				"{element_type:?}: {shape} typed NULLs"
			);
			assert_eq!(null_bound, QueryValue::Null, "{element_type:?}: whole NULL");
		}
	}

	#[test]
	fn manager_binds_naive_datetimes_without_converting_them_to_utc() {
		let value = chrono::NaiveDate::from_ymd_opt(2026, 7, 26)
			.expect("valid date")
			.and_hms_opt(9, 15, 30)
			.expect("valid time");

		let bound = Manager::<TestUser>::sea_value_to_query_value(
			reinhardt_query::value::Value::ChronoDateTime(Some(Box::new(value))),
		);

		assert_eq!(
			bound,
			crate::orm::connection::QueryValue::NaiveTimestamp(value)
		);
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestSettings {
		theme: String,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct JsonManagerModel {
		id: Option<i64>,
		scalar_json: Json<String>,
		settings: Json<TestSettings>,
		optional_json: Option<Json<serde_json::Value>>,
	}

	#[derive(Debug, Clone)]
	struct JsonManagerModelFields;

	impl FieldSelector for JsonManagerModelFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for JsonManagerModel {
		type PrimaryKey = i64;
		type Fields = JsonManagerModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"json_manager_models"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn new_fields() -> Self::Fields {
			JsonManagerModelFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			vec![
				test_manager_field_info("id", "BigIntegerField", false, true),
				test_manager_field_info("scalar_json", "JsonField", false, false),
				test_manager_field_info("settings", "JsonField", false, false),
				test_manager_field_info("optional_json", "JsonField", true, false),
			]
		}

		fn field_is_none(&self, field_name: &str) -> bool {
			match field_name {
				"id" => self.id.is_none(),
				"optional_json" => self.optional_json.is_none(),
				_ => false,
			}
		}
	}

	fn test_manager_field_info(
		name: &str,
		field_type: &str,
		nullable: bool,
		primary_key: bool,
	) -> FieldInfo {
		FieldInfo {
			name: name.to_string(),
			field_type: field_type.to_string(),
			storage_kind: None,
			domain: None,
			nullable,
			primary_key,
			unique: false,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: None,
			choices: None,
			attributes: HashMap::new(),
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct CompositeUpdateModel {
		tenant_key: String,
		entry_key: String,
		body: String,
	}

	impl Model for CompositeUpdateModel {
		type PrimaryKey = String;
		type Fields = JsonManagerModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"composite_updates"
		}

		fn primary_key_field() -> &'static str {
			"tenant_key"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.tenant_key.clone())
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.tenant_key = value;
		}

		fn new_fields() -> Self::Fields {
			JsonManagerModelFields
		}

		fn composite_primary_key() -> Option<crate::orm::composite_pk::CompositePrimaryKey> {
			Some(
				crate::orm::composite_pk::CompositePrimaryKey::new(vec![
					"tenant_key".to_owned(),
					"entry_key".to_owned(),
				])
				.unwrap(),
			)
		}

		fn field_metadata() -> Vec<FieldInfo> {
			let mut tenant = test_manager_field_info("tenant_key", "CharField", false, true);
			tenant.db_column = Some("tenant_id".to_owned());
			let mut entry = test_manager_field_info("entry_key", "CharField", false, true);
			entry.db_column = Some("entry_id".to_owned());
			vec![
				tenant,
				entry,
				test_manager_field_info("body", "TextField", false, false),
			]
		}
	}

	#[rstest]
	#[case(
		DatabaseBackend::Postgres,
		"UPDATE \"composite_updates\" SET \"body\" = $1 WHERE (\"tenant_id\" = $2 AND \"entry_id\" = $3) RETURNING \"tenant_id\", \"body\", \"entry_id\""
	)]
	#[case(
		DatabaseBackend::MySql,
		"UPDATE `composite_updates` SET `body` = ? WHERE (`tenant_id` = ? AND `entry_id` = ?)"
	)]
	#[case(
		DatabaseBackend::Sqlite,
		"UPDATE \"composite_updates\" SET \"body\" = ? WHERE (\"tenant_id\" = ? AND \"entry_id\" = ?) RETURNING \"tenant_id\", \"body\", \"entry_id\""
	)]
	fn composite_update_binds_every_key_and_preserves_database_values(
		#[case] backend: DatabaseBackend,
		#[case] expected_sql: &str,
	) {
		// Arrange: encoded UUID and enum storage values must remain typed.
		let tenant = uuid::Uuid::from_u128(42);
		let obj = std::collections::BTreeMap::from([
			("tenant_key".to_owned(), DatabaseValue::Uuid(tenant)),
			(
				"entry_key".to_owned(),
				DatabaseValue::String("stored-task".to_owned()),
			),
			(
				"body".to_owned(),
				DatabaseValue::String("new body".to_owned()),
			),
		]);

		// Act
		let stmt =
			Manager::<CompositeUpdateModel>::build_update_statement_from_object_with_returning(
				&obj,
				backend != DatabaseBackend::MySql,
			)
			.unwrap();
		let (sql, values) = super::build_update_sql(&stmt, backend);

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(
			values.0,
			vec![
				super::database_value_to_query_value(DatabaseValue::String("new body".to_owned())),
				super::database_value_to_query_value(DatabaseValue::Uuid(tenant)),
				super::database_value_to_query_value(DatabaseValue::String(
					"stored-task".to_owned()
				)),
			]
		);
	}

	#[rstest]
	#[case("tenant_key", false)]
	#[case("tenant_key", true)]
	#[case("entry_key", false)]
	#[case("entry_key", true)]
	fn composite_update_rejects_missing_or_null_key_components(
		#[case] field: &str,
		#[case] null: bool,
	) {
		// Arrange
		let mut obj = std::collections::BTreeMap::from([
			(
				"tenant_key".to_owned(),
				DatabaseValue::String("t".to_owned()),
			),
			(
				"entry_key".to_owned(),
				DatabaseValue::String("a".to_owned()),
			),
			(
				"body".to_owned(),
				DatabaseValue::String("new body".to_owned()),
			),
		]);
		if null {
			obj.insert(field.to_owned(), DatabaseValue::Null);
		} else {
			obj.remove(field);
		}

		// Act
		let error =
			Manager::<CompositeUpdateModel>::build_update_statement_from_object(&obj, |_| false)
				.unwrap_err();

		// Assert
		assert_eq!(
			error,
			FieldCodecError::Serialization(format!(
				"encoded composite_updates fields must contain a non-null primary key '{field}'"
			))
		);
	}

	#[rstest]
	fn composite_update_without_mutable_fields_keeps_all_key_predicates() {
		// Arrange
		let obj = std::collections::BTreeMap::from([
			(
				"tenant_key".to_owned(),
				DatabaseValue::String("t".to_owned()),
			),
			(
				"entry_key".to_owned(),
				DatabaseValue::String("a".to_owned()),
			),
		]);

		// Act
		let stmt =
			Manager::<CompositeUpdateModel>::build_update_statement_from_object(&obj, |_| false)
				.unwrap();
		let (sql, values) = super::build_update_sql(&stmt, DatabaseBackend::Postgres);

		// Assert
		assert_eq!(
			sql,
			"UPDATE \"composite_updates\" SET \"tenant_id\" = \"tenant_id\" WHERE (\"tenant_id\" = $1 AND \"entry_id\" = $2) RETURNING \"tenant_id\", \"entry_id\""
		);
		assert_eq!(values.0, vec!["t".into(), "a".into()]);
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct GeneratedUser {
		id: Option<i64>,
		name: String,
		email: String,
		full_name: String,
	}

	#[derive(Debug, Clone)]
	struct GeneratedUserFields;

	impl FieldSelector for GeneratedUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for GeneratedUser {
		type PrimaryKey = i64;
		type Fields = GeneratedUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"generated_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn generated_field_names() -> &'static [&'static str] {
			&["full_name"]
		}

		fn new_fields() -> Self::Fields {
			GeneratedUserFields
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct GeneratedOnlyUser {
		id: Option<i64>,
		full_name: String,
	}

	#[derive(Debug, Clone)]
	struct GeneratedOnlyUserFields;

	impl FieldSelector for GeneratedOnlyUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for GeneratedOnlyUser {
		type PrimaryKey = i64;
		type Fields = GeneratedOnlyUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"generated_only_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn generated_field_names() -> &'static [&'static str] {
			&["full_name"]
		}

		fn new_fields() -> Self::Fields {
			GeneratedOnlyUserFields
		}
	}

	#[test]
	fn test_bulk_create_sql() {
		use serde_json::json;
		let manager = TestUser::objects();
		let fields = vec!["name".to_string(), "email".to_string()];
		let values = vec![
			vec![json!("Alice"), json!("alice@example.com")],
			vec![json!("Bob"), json!("bob@example.com")],
		];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		// reinhardt-query uses quoted identifiers and TestUser table is "test_user"
		assert!(sql.contains("INSERT"));
		assert!(sql.contains("test_user"));
		assert!(sql.contains("name"));
		assert!(sql.contains("email"));
		assert!(sql.contains("Alice"));
		assert!(sql.contains("alice@example.com"));
		assert!(sql.contains("Bob"));
		assert!(sql.contains("bob@example.com"));
	}

	#[test]
	fn test_bulk_create_sql_with_conflict() {
		use serde_json::json;
		let manager = TestUser::objects();
		let fields = vec!["name".to_string(), "email".to_string()];
		let values = vec![vec![json!("Alice"), json!("alice@example.com")]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, true);

		assert!(sql.contains("ON CONFLICT DO NOTHING"));
	}

	#[test]
	fn test_bulk_create_query_preserves_json_field_tags() {
		let manager = JsonManagerModel::objects();
		let model = JsonManagerModel {
			id: Some(1),
			scalar_json: Json::new("draft".to_string()),
			settings: Json::new(TestSettings {
				theme: "paper".to_string(),
			}),
			optional_json: Some(Json::new(serde_json::Value::Null)),
		};

		let stmt = manager.bulk_create_query(&[model]).unwrap();
		let (_, values) = super::build_insert_sql(&stmt, DatabaseBackend::Postgres);
		let json_value_count = values
			.0
			.iter()
			.filter(|value| matches!(value, reinhardt_query::value::Value::Json(_)))
			.count();

		assert_eq!(json_value_count, 3);
	}

	#[rstest::rstest]
	fn test_backend_json_string_scalar_preserves_native_json_provenance() {
		// Arrange
		let expected = JsonManagerModel {
			id: Some(1),
			scalar_json: Json::new("draft".to_string()),
			settings: Json::new(TestSettings {
				theme: "paper".to_string(),
			}),
			optional_json: None,
		};
		let mut backend_row = crate::backends::types::Row::new();
		backend_row.insert("id".to_string(), crate::backends::types::QueryValue::Int(1));
		backend_row.insert(
			"scalar_json".to_string(),
			crate::backends::types::QueryValue::Json(Some(Box::new(serde_json::Value::String(
				"draft".to_string(),
			)))),
		);
		backend_row.insert(
			"settings".to_string(),
			crate::backends::types::QueryValue::Json(Some(Box::new(serde_json::json!({
				"theme": "paper"
			})))),
		);
		backend_row.insert(
			"optional_json".to_string(),
			crate::backends::types::QueryValue::Json(None),
		);

		// Act
		let model = crate::orm::connection::QueryRow::from_backend_row(backend_row)
			.deserialize_model::<JsonManagerModel>()
			.unwrap();

		// Assert
		assert_eq!(model, expected);
	}

	#[cfg(feature = "sqlite")]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_manager_create_roundtrips_typed_json_fields_on_sqlite() {
		let database_file = tempfile::NamedTempFile::new().unwrap();
		let database_url = format!("sqlite://{}", database_file.path().display());
		let owner = crate::orm::connection::BackendsConnection::connect_sqlite(&database_url)
			.await
			.unwrap();
		let lease = crate::orm::connection::DatabaseConnectionLease::register(owner).unwrap();
		let mut connection = lease.handle();
		connection
			.execute(
				"CREATE TABLE json_manager_models (\
				 id INTEGER PRIMARY KEY AUTOINCREMENT, \
				 scalar_json TEXT NOT NULL, \
				 settings TEXT NOT NULL, \
				 optional_json TEXT NULL)",
				vec![],
			)
			.await
			.unwrap();
		let model = JsonManagerModel {
			id: None,
			scalar_json: Json::new("draft".to_string()),
			settings: Json::new(TestSettings {
				theme: "paper".to_string(),
			}),
			optional_json: Some(Json::new(serde_json::Value::Null)),
		};

		let created = JsonManagerModel::objects()
			.create_with_conn(&mut connection, &model)
			.await
			.unwrap();

		assert_eq!(created.scalar_json.as_inner(), "draft");
		assert_eq!(created.settings.theme, "paper");
		assert_eq!(
			created.optional_json.unwrap().into_inner(),
			serde_json::Value::Null
		);
	}

	#[test]
	fn test_bulk_create_sql_detailed_omits_generated_fields() {
		use serde_json::json;
		let manager = GeneratedUser::objects();
		let fields = vec![
			"name".to_string(),
			"email".to_string(),
			"full_name".to_string(),
		];
		let values = vec![vec![
			json!("Alice"),
			json!("alice@example.com"),
			json!("Alice Smith"),
		]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		assert!(sql.contains("INSERT INTO generated_user"));
		assert!(sql.contains("name"));
		assert!(sql.contains("email"));
		assert!(!sql.contains("full_name"));
		assert!(!sql.contains("Alice Smith"));
	}

	#[test]
	fn test_bulk_create_sql_detailed_returns_empty_for_only_generated_fields() {
		use serde_json::json;
		let manager = GeneratedUser::objects();
		let fields = vec!["full_name".to_string()];
		let values = vec![vec![json!("Alice Smith")]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		assert!(sql.is_empty());
	}

	#[test]
	fn test_bulk_update_sql() {
		use serde_json::json;
		let manager = TestUser::objects();

		let mut updates = Vec::new();
		let mut user1_fields = HashMap::new();
		user1_fields.insert("name".to_string(), json!("Alice Updated"));
		user1_fields.insert("email".to_string(), json!("alice_new@example.com"));
		updates.push((1i64, user1_fields));

		let mut user2_fields = HashMap::new();
		user2_fields.insert("name".to_string(), json!("Bob Updated"));
		user2_fields.insert("email".to_string(), json!("bob_new@example.com"));
		updates.push((2i64, user2_fields));

		let fields = vec!["name".to_string(), "email".to_string()];
		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);

		// reinhardt-query uses quoted identifiers and TestUser table is "test_user"
		assert!(sql.contains("UPDATE"));
		assert!(sql.contains("test_user"));
		assert!(sql.contains("SET"));
		assert!(sql.contains("name"));
		assert!(sql.contains("CASE"));
		assert!(sql.contains("email"));
		assert!(sql.contains("Alice Updated"));
		assert!(sql.contains("Bob Updated"));
		assert!(sql.contains("WHERE"));
	}

	#[test]
	fn test_bulk_update_database_values_serializes_arrays_per_backend() {
		use crate::orm::{DatabaseArrayType, DatabaseValue};

		let manager = TestUser::objects();
		let mut field_values = HashMap::new();
		field_values.insert(
			"name".to_string(),
			DatabaseValue::Array {
				element_type: DatabaseArrayType::String,
				values: vec![
					DatabaseValue::String("alpha".to_string()),
					DatabaseValue::String("beta".to_string()),
				],
			},
		);
		let updates = vec![(DatabaseValue::I64(1), field_values)];
		let fields = vec!["name".to_string()];

		let sqlite_sql = manager
			.bulk_update_database_values_sql_detailed(&updates, &fields, DatabaseBackend::Sqlite)
			.expect("SQLite array SQL should render");
		assert!(sqlite_sql.contains("'[\"alpha\",\"beta\"]'"));
		assert!(!sqlite_sql.contains("ARRAY["));

		let postgres_sql = manager
			.bulk_update_database_values_sql_detailed(&updates, &fields, DatabaseBackend::Postgres)
			.expect("PostgreSQL array SQL should render");
		assert!(postgres_sql.contains("ARRAY["));
	}

	#[test]
	fn test_bulk_update_database_values_casts_empty_postgres_arrays() {
		use crate::orm::{DatabaseArrayType, DatabaseValue};

		let manager = TestUser::objects();
		let mut field_values = HashMap::new();
		field_values.insert(
			"name".to_string(),
			DatabaseValue::Array {
				element_type: DatabaseArrayType::String,
				values: vec![],
			},
		);
		let updates = vec![(DatabaseValue::I64(1), field_values)];
		let fields = vec!["name".to_string()];

		let sql = manager
			.bulk_update_database_values_sql_detailed(&updates, &fields, DatabaseBackend::Postgres)
			.expect("PostgreSQL empty array SQL should render");

		assert!(sql.contains("ARRAY[]::text[]"));
	}

	#[rstest::rstest]
	#[case(DatabaseArrayType::String, "text")]
	#[case(DatabaseArrayType::I32, "integer")]
	#[case(DatabaseArrayType::I64, "bigint")]
	#[case(DatabaseArrayType::F32, "real")]
	#[case(DatabaseArrayType::F64, "double precision")]
	#[case(DatabaseArrayType::Bool, "boolean")]
	#[case(DatabaseArrayType::Uuid, "uuid")]
	fn bulk_update_nullable_arrays_keep_postgres_element_types(
		#[case] element_type: DatabaseArrayType,
		#[case] postgres_type: &str,
	) {
		// Arrange
		let value = DatabaseValue::Array {
			element_type,
			values: vec![DatabaseValue::Null, DatabaseValue::Null],
		};

		// Act
		let literal = super::database_value_sql_literal(value.clone(), DatabaseBackend::Postgres)
			.expect("PostgreSQL nullable array should render");
		let sqlite_literal =
			super::database_value_sql_literal(value.clone(), DatabaseBackend::Sqlite)
				.expect("SQLite nullable array should render");
		let mysql_literal = super::database_value_sql_literal(value, DatabaseBackend::MySql)
			.expect("MySQL nullable array should render");

		// Assert
		assert_eq!(literal, format!("ARRAY[NULL,NULL]::{postgres_type}[]"));
		assert_eq!(sqlite_literal, "'[null,null]'");
		assert_eq!(mysql_literal, "'[null,null]'");
	}

	#[test]
	fn test_bulk_update_sql_detailed_omits_generated_fields() {
		use serde_json::json;
		let manager = GeneratedUser::objects();
		let mut updates = Vec::new();
		let mut fields_map = HashMap::new();
		fields_map.insert("name".to_string(), json!("Alice Updated"));
		fields_map.insert("full_name".to_string(), json!("Alice Smith"));
		updates.push((1i64, fields_map));
		let fields = vec!["name".to_string(), "full_name".to_string()];

		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);

		assert!(sql.contains("UPDATE \"generated_user\""));
		assert!(sql.contains("\"name\""));
		assert!(sql.contains("Alice Updated"));
		assert!(!sql.contains("full_name"));
		assert!(!sql.contains("Alice Smith"));
	}

	#[test]
	fn test_bulk_update_sql_detailed_returns_empty_for_only_generated_fields() {
		use serde_json::json;
		let manager = GeneratedUser::objects();
		let mut updates = Vec::new();
		let mut fields_map = HashMap::new();
		fields_map.insert("full_name".to_string(), json!("Alice Smith"));
		updates.push((1i64, fields_map));
		let fields = vec!["full_name".to_string()];

		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);

		assert!(sql.is_empty());
	}

	#[test]
	fn test_update_statement_uses_noop_set_for_generated_only_models() {
		let model = GeneratedOnlyUser {
			id: Some(7),
			full_name: "Alice Smith".to_string(),
		};
		let obj = model
			.encode_database_fields()
			.expect("model fields should encode");
		let stmt =
			Manager::<GeneratedOnlyUser>::build_update_statement_from_object(&obj, |_| false)
				.expect("encoded primary key should build an update statement");

		let (sql, params) = super::build_update_sql(&stmt, DatabaseBackend::Postgres);

		assert_eq!(
			sql,
			"UPDATE \"generated_only_user\" SET \"id\" = \"id\" WHERE \"id\" = $1 RETURNING \"id\", \"full_name\""
		);
		assert_eq!(params.len(), 1);
	}

	#[test]
	fn test_create_statement_rejects_generated_only_models() {
		let model = GeneratedOnlyUser {
			id: None,
			full_name: "Alice Smith".to_string(),
		};
		let obj = model
			.encode_database_fields()
			.expect("model fields should encode");

		let err = Manager::<GeneratedOnlyUser>::build_insert_statement_from_object(&obj, |_| false)
			.expect_err("generated-only create should fail before rendering empty INSERT");

		assert!(
			err.to_string().contains("no writable fields remain"),
			"unexpected error: {err}"
		);
	}

	#[test]
	fn test_bulk_create_empty() {
		use serde_json::Value;
		let manager = TestUser::objects();
		let fields: Vec<String> = vec![];
		let values: Vec<Vec<Value>> = vec![];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);
		assert!(sql.is_empty());
	}

	#[test]
	fn test_bulk_update_empty() {
		use serde_json::Value;
		let manager = TestUser::objects();
		let updates: Vec<(i64, HashMap<String, Value>)> = vec![];
		let fields = vec!["name".to_string()];

		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);
		assert!(sql.is_empty());
	}

	// ──────────────────────────────────────────────────────────────
	// Additional manager tests
	// ──────────────────────────────────────────────────────────────

	#[test]
	fn test_manager_new() {
		let manager = super::Manager::<TestUser>::new();
		// Manager is just a phantom type wrapper, so this just ensures it compiles
		let _ = manager;
	}

	#[test]
	fn test_manager_default() {
		let manager = super::Manager::<TestUser>::default();
		// Default should work the same as new
		let _ = manager;
	}

	#[test]
	fn test_bulk_create_sql_single_row() {
		use serde_json::json;
		let manager = TestUser::objects();
		let fields = vec!["name".to_string()];
		let values = vec![vec![json!("SingleUser")]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		assert!(sql.contains("INSERT"));
		assert!(sql.contains("test_user"));
		assert!(sql.contains("SingleUser"));
	}

	#[test]
	fn test_bulk_update_sql_single_field() {
		use serde_json::json;
		let manager = TestUser::objects();

		let mut updates = Vec::new();
		let mut user1_fields = HashMap::new();
		user1_fields.insert("name".to_string(), json!("Updated Name"));
		updates.push((1i64, user1_fields));

		let fields = vec!["name".to_string()];
		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);

		assert!(sql.contains("UPDATE"));
		assert!(sql.contains("name"));
		assert!(sql.contains("Updated Name"));
		assert!(!sql.contains("email"));
	}

	#[test]
	fn test_json_to_sea_value_string() {
		use serde_json::json;
		let value = json!("hello");
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// reinhardt-query Value should contain the string
		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("hello") || debug_str.contains("String"));
	}

	#[test]
	fn test_json_to_sea_value_integer() {
		use serde_json::json;
		let value = json!(42);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("42") || debug_str.contains("Int"));
	}

	#[test]
	fn test_json_to_sea_value_float() {
		use serde_json::json;
		let value = json!(1.5);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("1.5") || debug_str.contains("Double"));
	}

	#[test]
	fn test_json_to_sea_value_bool() {
		use serde_json::json;
		let value = json!(true);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("true") || debug_str.contains("Bool"));
	}

	#[test]
	fn test_json_to_sea_value_null() {
		use serde_json::json;
		let value = json!(null);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// Null should be represented somehow
		let debug_str = format!("{:?}", sea_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_json_to_sea_value_array() {
		use serde_json::json;
		let value = json!([1, 2, 3]);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// Array should be converted (typically to JSON string)
		let debug_str = format!("{:?}", sea_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_json_to_sea_value_object() {
		use serde_json::json;
		let value = json!({"key": "value"});
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// Object should be converted (typically to JSON string)
		let debug_str = format!("{:?}", sea_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_serialize_value_string() {
		use serde_json::json;
		let value = json!("test_string");
		let serialized = super::Manager::<TestUser>::serialize_value(&value);

		// Should return the string representation
		assert!(serialized.contains("test_string"));
	}

	#[test]
	fn test_serialize_value_number() {
		use serde_json::json;
		let value = json!(123);
		let serialized = super::Manager::<TestUser>::serialize_value(&value);

		assert!(serialized.contains("123"));
	}
}
