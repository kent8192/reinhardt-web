//! Database-backed OAuth state integration tests.
//!
//! Each `DatabaseSessionBackend` owns its own connection, standing in for one
//! application replica. The replicas share a PostgreSQL `sessions` table.
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_auth::sessions::backends::database::DatabaseSessionBackend;
use reinhardt_auth::social::flow::SessionStateStore;
use reinhardt_auth::social::{ContextualStateData, SocialAuthError, StateData, StateStore};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::sync::Barrier;

const CONTENDERS: usize = 16;

/// Connects two replicas to one PostgreSQL database with a fresh table.
async fn replicas(
	database_url: &str,
) -> (
	SessionStateStore<DatabaseSessionBackend>,
	SessionStateStore<DatabaseSessionBackend>,
) {
	let first = DatabaseSessionBackend::new(database_url)
		.await
		.expect("first replica should connect");
	let second = DatabaseSessionBackend::new(database_url)
		.await
		.expect("second replica should connect");
	first
		.create_table()
		.await
		.expect("sessions table should be created");
	(
		SessionStateStore::new(first),
		SessionStateStore::new(second),
	)
}

fn contextual_record(state: &str) -> ContextualStateData {
	ContextualStateData::new(
		StateData::new(state.to_string(), None, Some("verifier".to_string())),
		"github".to_string(),
		b"browser-a",
		b"link-user-42".to_vec(),
	)
	.expect("binding is not empty")
}

#[rstest]
#[case::legacy(false)]
#[case::contextual(true)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial(sessions_db_oauth_state)]
async fn state_is_consumed_once_across_database_replicas(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] contextual: bool,
) {
	// Arrange
	let (_container, _pool, _port, database_url) = postgres_container.await;
	let (first, second) = replicas(&database_url).await;
	if contextual {
		first
			.store_contextual(contextual_record("shared-state"))
			.await
			.unwrap();
	} else {
		first
			.store(StateData::new("shared-state".to_string(), None, None))
			.await
			.unwrap();
	}
	let stores = [Arc::new(first), Arc::new(second)];
	let barrier = Arc::new(Barrier::new(CONTENDERS));

	// Act
	let handles: Vec<_> = (0..CONTENDERS)
		.map(|index| {
			let store = Arc::clone(&stores[index % stores.len()]);
			let barrier = Arc::clone(&barrier);
			tokio::spawn(async move {
				barrier.wait().await;
				if contextual {
					store
						.consume_contextual("shared-state")
						.await
						.map(|record| record.state_data().state.clone())
				} else {
					store
						.consume("shared-state")
						.await
						.map(|record| record.state)
				}
			})
		})
		.collect();
	let mut winners = Vec::new();
	for handle in handles {
		match handle.await.expect("consumer task should not panic") {
			Ok(state) => winners.push(state),
			Err(SocialAuthError::InvalidState) => {}
			Err(other) => panic!("unexpected consume error: {other:?}"),
		}
	}

	// Assert
	assert_eq!(winners, vec!["shared-state".to_string()]);
}

#[rstest]
#[tokio::test]
#[serial(sessions_db_oauth_state)]
async fn contextual_state_round_trips_and_rejects_replay_across_database_replicas(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, _pool, _port, database_url) = postgres_container.await;
	let (first, second) = replicas(&database_url).await;
	first
		.store_contextual(contextual_record("bound-state"))
		.await
		.unwrap();

	// Act
	let consumed = second.consume_contextual("bound-state").await.unwrap();
	let replay = first.consume_contextual("bound-state").await;

	// Assert
	assert_eq!(consumed.provider_name(), "github");
	assert_eq!(consumed.context(), b"link-user-42");
	assert_eq!(
		consumed.state_data().code_verifier.as_deref(),
		Some("verifier")
	);
	assert!(consumed.binding_matches(b"browser-a"));
	assert!(!consumed.binding_matches(b"browser-b"));
	assert!(matches!(replay, Err(SocialAuthError::InvalidState)));
}
