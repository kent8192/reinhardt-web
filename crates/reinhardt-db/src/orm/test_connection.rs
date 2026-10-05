//! Owned native connections for ORM regression tests.
use super::connection::{
	BackendsConnection, DatabaseConnection, DatabaseConnectionLease, OrmExecutor, QueryResult,
};
use reinhardt_core::exception::Result;
use reinhardt_query::Values;
#[derive(Clone)]
pub(crate) struct TestConnection {
	owner: BackendsConnection,
	pub(crate) lease: DatabaseConnectionLease,
	handle: DatabaseConnection,
}
impl TestConnection {
	pub(crate) async fn connect(url: &str) -> Result<Self> {
		Self::connect_with_pool_size(url, Some(1)).await
	}
	pub(crate) async fn connect_with_pool_size(url: &str, size: Option<u32>) -> Result<Self> {
		let owner = super::engine::connect_backend_with_pool_size(url, size).await?;
		let lease = DatabaseConnectionLease::register(owner.clone())?;
		let handle = lease.handle();
		Ok(Self {
			owner,
			lease,
			handle,
		})
	}
	pub(crate) fn inner(&self) -> &BackendsConnection {
		&self.owner
	}
	pub(crate) async fn execute_generated(
		&mut self,
		sql: &str,
		values: Values,
	) -> Result<QueryResult> {
		self.handle
			.execute_generated_with_context(sql, values, None)
			.await
	}
}
impl std::ops::Deref for TestConnection {
	type Target = DatabaseConnection;
	fn deref(&self) -> &Self::Target {
		&self.handle
	}
}
impl std::ops::DerefMut for TestConnection {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.handle
	}
}
