use reinhardt_core::exception::DatabaseErrorKind;
use reinhardt_db::{
	backends::DatabaseConnection as BackendsConnection,
	orm::{
		DatabaseConnection, DatabaseConnectionLease, get_connection_registration,
		register_request_database, reinitialize_database,
	},
};
use reinhardt_di::{Injectable, InjectionContext, SingletonScope};

async fn sqlite_owner() -> BackendsConnection {
	BackendsConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap()
}

#[tokio::test]
async fn cloned_context_keeps_database_registration_alive_until_last_drop() {
	let lease = DatabaseConnectionLease::register(sqlite_owner().await).unwrap();
	let handle = lease.handle();
	let context = InjectionContext::builder(SingletonScope::new())
		.singleton(lease)
		.singleton(handle)
		.build();
	let cloned = context.clone();
	let injected = DatabaseConnection::inject(&context).await.unwrap();

	drop(context);
	injected.execute("SELECT 1", vec![]).await.unwrap();
	drop(cloned);

	let error = injected.execute("SELECT 1", vec![]).await.unwrap_err();
	assert_eq!(
		error.database_error().unwrap().kind(),
		DatabaseErrorKind::ConnectionHandleExpired
	);
}

#[tokio::test]
async fn request_registration_lives_with_context_clone() {
	let context = InjectionContext::builder(SingletonScope::new()).build();
	let handle = register_request_database(&context, sqlite_owner().await).unwrap();
	let cloned = context.clone();

	drop(context);
	handle.execute("SELECT 1", vec![]).await.unwrap();
	drop(cloned);

	let error = handle.execute("SELECT 1", vec![]).await.unwrap_err();
	assert_eq!(
		error.database_error().unwrap().kind(),
		DatabaseErrorKind::ConnectionHandleExpired
	);
}

#[tokio::test]
async fn fresh_request_fork_does_not_inherit_request_database() {
	let context = InjectionContext::builder(SingletonScope::new()).build();
	let handle = register_request_database(&context, sqlite_owner().await).unwrap();
	let fork = context.fork();

	let error = DatabaseConnection::inject(&fork).await.unwrap_err();
	assert!(matches!(error, reinhardt_di::DiError::NotRegistered { .. }));
	drop(context);
	let error = handle.execute("SELECT 1", vec![]).await.unwrap_err();
	assert_eq!(
		error.database_error().unwrap().kind(),
		DatabaseErrorKind::ConnectionHandleExpired
	);
}

#[tokio::test]
async fn fresh_request_fork_inherits_singleton_database() {
	let lease = DatabaseConnectionLease::register(sqlite_owner().await).unwrap();
	let handle = lease.handle();
	let context = InjectionContext::builder(SingletonScope::new())
		.singleton(lease)
		.singleton(handle)
		.build();
	let fork = context.fork();

	let injected = DatabaseConnection::inject(&fork).await.unwrap();
	assert_eq!(injected, handle);
	injected.execute("SELECT 1", vec![]).await.unwrap();
}

#[tokio::test]
async fn singleton_database_takes_precedence_over_request_database() {
	let singleton_lease = DatabaseConnectionLease::register(sqlite_owner().await).unwrap();
	let singleton_handle = singleton_lease.handle();
	let context = InjectionContext::builder(SingletonScope::new())
		.singleton(singleton_lease)
		.singleton(singleton_handle)
		.build();
	let request_handle = register_request_database(&context, sqlite_owner().await).unwrap();

	let injected = DatabaseConnection::inject(&context).await.unwrap();
	assert_eq!(injected, singleton_handle);
	assert_ne!(injected, request_handle);
}

#[serial_test::serial(database_connection_registration)]
#[tokio::test]
async fn global_registration_accessor_returns_matching_lease_and_handle() {
	reinitialize_database("sqlite::memory:").await.unwrap();

	let (lease, handle) = get_connection_registration().await.unwrap();

	assert_eq!(lease.handle(), handle);
}

#[serial_test::serial(database_connection_registration)]
#[tokio::test]
async fn concurrent_reinitialize_and_registration_reads_remain_coherent() {
	use std::sync::Arc;

	use tokio::sync::{Barrier, mpsc};

	reinitialize_database("sqlite::memory:").await.unwrap();
	let barrier = Arc::new(Barrier::new(2));
	let (sender, mut receiver) = mpsc::channel(32);
	let writer_barrier = Arc::clone(&barrier);
	let writer = tokio::spawn(async move {
		writer_barrier.wait().await;
		for _ in 0..32 {
			reinitialize_database("sqlite::memory:").await.unwrap();
			sender.send(()).await.unwrap();
		}
	});
	let reader_barrier = Arc::clone(&barrier);
	let reader = tokio::spawn(async move {
		reader_barrier.wait().await;
		while receiver.recv().await.is_some() {
			let (lease, handle) = get_connection_registration().await.unwrap();
			assert_eq!(lease.handle(), handle);
		}
	});

	writer.await.unwrap();
	reader.await.unwrap();
}
