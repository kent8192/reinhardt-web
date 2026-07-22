use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;
use reinhardt_core::exception::{DatabaseError, DatabaseErrorKind};

use super::connection::{BackendsConnection, DatabaseBackend};

pub(crate) type ConnectionSlot = u32;
pub(crate) type Generation = u64;

struct RegistryEntry {
	generation: Generation,
	owner: Option<Arc<BackendsConnection>>,
	retired: bool,
}

#[derive(Default)]
struct RegistryState {
	entries: Vec<RegistryEntry>,
	free: Vec<ConnectionSlot>,
}

struct DatabaseConnectionRegistry {
	state: RwLock<RegistryState>,
}

impl DatabaseConnectionRegistry {
	fn new() -> Self {
		Self {
			state: RwLock::new(RegistryState::default()),
		}
	}

	fn insert(
		&self,
		owner: Arc<BackendsConnection>,
	) -> std::result::Result<(ConnectionSlot, Generation), DatabaseError> {
		let mut state = self.state.write();
		while let Some(slot) = state.free.pop() {
			let entry = &mut state.entries[slot as usize];
			match entry.generation.checked_add(1) {
				Some(generation) => {
					entry.generation = generation;
					entry.owner = Some(owner);
					return Ok((slot, generation));
				}
				None => entry.retired = true,
			}
		}

		let slot = ConnectionSlot::try_from(state.entries.len()).map_err(|_| {
			DatabaseError::new(
				DatabaseErrorKind::Connection,
				"Database connection registry exhausted its slot space",
			)
		})?;
		state.entries.push(RegistryEntry {
			generation: 0,
			owner: Some(owner),
			retired: false,
		});
		Ok((slot, 0))
	}

	fn resolve(
		&self,
		slot: ConnectionSlot,
		generation: Generation,
	) -> std::result::Result<Arc<BackendsConnection>, DatabaseError> {
		self.state
			.read()
			.entries
			.get(slot as usize)
			.filter(|entry| entry.generation == generation)
			.and_then(|entry| entry.owner.as_ref())
			.cloned()
			.ok_or_else(expired_handle_error)
	}

	fn remove_exact(&self, slot: ConnectionSlot, generation: Generation) {
		let mut state = self.state.write();
		let Some(entry) = state.entries.get_mut(slot as usize) else {
			return;
		};
		if entry.generation != generation || entry.owner.take().is_none() {
			return;
		}
		if !entry.retired {
			state.free.push(slot);
		}
	}
}

#[derive(Clone)]
pub(crate) struct RegisteredConnection {
	state: Arc<RegistrationState>,
	backend: DatabaseBackend,
}

struct RegistrationState {
	slot: ConnectionSlot,
	generation: Generation,
}

impl Drop for RegistrationState {
	fn drop(&mut self) {
		registry().remove_exact(self.slot, self.generation);
	}
}

impl RegisteredConnection {
	pub(crate) fn handle_parts(&self) -> (ConnectionSlot, Generation, DatabaseBackend) {
		(self.state.slot, self.state.generation, self.backend)
	}
}

pub(crate) fn register(
	owner: BackendsConnection,
) -> std::result::Result<RegisteredConnection, DatabaseError> {
	let backend = owner.database_type().into();
	let (slot, generation) = registry().insert(Arc::new(owner))?;
	Ok(RegisteredConnection {
		state: Arc::new(RegistrationState { slot, generation }),
		backend,
	})
}

pub(crate) fn resolve(
	slot: ConnectionSlot,
	generation: Generation,
) -> std::result::Result<Arc<BackendsConnection>, DatabaseError> {
	registry().resolve(slot, generation)
}

fn registry() -> &'static DatabaseConnectionRegistry {
	static REGISTRY: OnceLock<DatabaseConnectionRegistry> = OnceLock::new();
	REGISTRY.get_or_init(DatabaseConnectionRegistry::new)
}

fn expired_handle_error() -> DatabaseError {
	DatabaseError::new(
		DatabaseErrorKind::ConnectionHandleExpired,
		"The injected database connection is no longer available because its DI scope has ended",
	)
}

#[cfg(test)]
mod tests {
	use std::sync::{Arc, Barrier};

	use async_trait::async_trait;
	use parking_lot::Mutex;
	use reinhardt_core::exception::{DatabaseErrorKind, Result};

	use super::{register, resolve};
	use crate::backends::{
		backend::DatabaseBackend,
		connection::DatabaseConnection as BackendsConnection,
		types::{DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor},
	};

	static TEST_LOCK: Mutex<()> = Mutex::new(());

	struct TestBackend(DatabaseType);

	#[async_trait]
	impl DatabaseBackend for TestBackend {
		fn database_type(&self) -> DatabaseType {
			self.0
		}
		fn placeholder(&self, index: usize) -> String {
			format!("${index}")
		}
		fn supports_returning(&self) -> bool {
			true
		}
		fn supports_on_conflict(&self) -> bool {
			true
		}
		async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			unreachable!()
		}
		async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			unreachable!()
		}
		async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			unreachable!()
		}
		async fn fetch_optional(
			&self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			unreachable!()
		}
		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			unreachable!()
		}
		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
	}

	fn mock_backend_connection(database_type: DatabaseType) -> BackendsConnection {
		BackendsConnection::new(Arc::new(TestBackend(database_type)))
	}

	#[test]
	fn live_registration_resolves_owner() {
		let _test_guard = TEST_LOCK.lock();
		let registration = register(mock_backend_connection(DatabaseType::Sqlite)).unwrap();
		let (slot, generation, backend) = registration.handle_parts();

		assert_eq!(backend, super::super::connection::DatabaseBackend::Sqlite);
		assert_eq!(
			resolve(slot, generation).unwrap().database_type(),
			DatabaseType::Sqlite
		);
	}

	#[test]
	fn stale_generation_never_resolves_reused_slot() {
		let _test_guard = TEST_LOCK.lock();
		let first = register(mock_backend_connection(DatabaseType::Sqlite)).unwrap();
		let (slot, generation, _) = first.handle_parts();
		drop(first);

		let second = register(mock_backend_connection(DatabaseType::Postgres)).unwrap();

		let error = resolve(slot, generation)
			.err()
			.expect("the stale generation must not resolve");
		assert_eq!(error.kind(), DatabaseErrorKind::ConnectionHandleExpired);
		assert_eq!(second.handle_parts().0, slot);
		assert_ne!(second.handle_parts().1, generation);
	}

	#[test]
	fn resolved_owner_survives_registration_drop() {
		let _test_guard = TEST_LOCK.lock();
		let registration = register(mock_backend_connection(DatabaseType::Sqlite)).unwrap();
		let (slot, generation, _) = registration.handle_parts();
		let owner = resolve(slot, generation).unwrap();

		drop(registration);

		assert_eq!(owner.database_type(), DatabaseType::Sqlite);
		assert_eq!(
			resolve(slot, generation)
				.err()
				.expect("the dropped registration must expire")
				.kind(),
			DatabaseErrorKind::ConnectionHandleExpired
		);
	}

	#[test]
	fn owner_expires_only_after_last_registration_clone_drops() {
		let _test_guard = TEST_LOCK.lock();
		let registration = register(mock_backend_connection(DatabaseType::Mysql)).unwrap();
		let first_clone = registration.clone();
		let last_clone = registration.clone();
		let (slot, generation, _) = registration.handle_parts();

		drop(registration);
		drop(first_clone);
		assert_eq!(
			resolve(slot, generation).unwrap().database_type(),
			DatabaseType::Mysql
		);
		drop(last_clone);
		assert_eq!(
			resolve(slot, generation)
				.err()
				.expect("the final dropped clone must expire")
				.kind(),
			DatabaseErrorKind::ConnectionHandleExpired
		);
	}

	#[test]
	fn concurrent_resolution_remains_valid_until_last_clone_drops() {
		let _test_guard = TEST_LOCK.lock();
		let registration = register(mock_backend_connection(DatabaseType::Postgres)).unwrap();
		let retained = registration.clone();
		let (slot, generation, _) = registration.handle_parts();
		let barrier = Arc::new(Barrier::new(2));
		let worker_barrier = Arc::clone(&barrier);
		let worker = std::thread::spawn(move || {
			worker_barrier.wait();
			resolve(slot, generation).unwrap().database_type()
		});

		drop(registration);
		barrier.wait();
		assert_eq!(worker.join().unwrap(), DatabaseType::Postgres);
		drop(retained);
		assert_eq!(
			resolve(slot, generation)
				.err()
				.expect("the final dropped clone must expire")
				.kind(),
			DatabaseErrorKind::ConnectionHandleExpired
		);
	}
}
