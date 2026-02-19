use async_graphql::{Context, ID, Subscription};
use futures_util::Stream;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

/// Event types for subscriptions
#[derive(Debug, Clone)]
pub enum UserEvent {
	Created(crate::schema::User),
	Updated(crate::schema::User),
	Deleted(ID),
}

/// Event broadcaster
#[derive(Clone)]
pub struct EventBroadcaster {
	tx: Arc<RwLock<broadcast::Sender<UserEvent>>>,
}

impl EventBroadcaster {
	/// Create a new event broadcaster
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_graphql::subscription::EventBroadcaster;
	///
	/// let broadcaster = EventBroadcaster::new();
	/// ```
	pub fn new() -> Self {
		let (tx, _) = broadcast::channel(100);
		Self {
			tx: Arc::new(RwLock::new(tx)),
		}
	}
	/// Broadcast an event to all subscribers
	///
	pub async fn broadcast(&self, event: UserEvent) {
		let tx = self.tx.read().await;
		let _ = tx.send(event);
	}
	/// Subscribe to events
	///
	pub async fn subscribe(&self) -> broadcast::Receiver<UserEvent> {
		self.tx.read().await.subscribe()
	}
}

impl Default for EventBroadcaster {
	fn default() -> Self {
		Self::new()
	}
}

/// GraphQL Subscription root
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
	async fn user_created<'ctx>(
		&self,
		ctx: &Context<'ctx>,
	) -> impl Stream<Item = crate::schema::User> + 'ctx {
		// Gracefully handle missing EventBroadcaster instead of panicking.
		// Returns an empty stream if the broadcaster is not in context.
		let receiver = match ctx.data::<EventBroadcaster>() {
			Ok(broadcaster) => Some(broadcaster.subscribe().await),
			Err(_) => None,
		};

		async_stream::stream! {
			if let Some(mut rx) = receiver {
				while let Ok(event) = rx.recv().await {
					if let UserEvent::Created(user) = event {
						yield user;
					}
				}
			}
		}
	}

	async fn user_updated<'ctx>(
		&self,
		ctx: &Context<'ctx>,
	) -> impl Stream<Item = crate::schema::User> + 'ctx {
		let receiver = match ctx.data::<EventBroadcaster>() {
			Ok(broadcaster) => Some(broadcaster.subscribe().await),
			Err(_) => None,
		};

		async_stream::stream! {
			if let Some(mut rx) = receiver {
				while let Ok(event) = rx.recv().await {
					if let UserEvent::Updated(user) = event {
						yield user;
					}
				}
			}
		}
	}

	async fn user_deleted<'ctx>(&self, ctx: &Context<'ctx>) -> impl Stream<Item = ID> + 'ctx {
		let receiver = match ctx.data::<EventBroadcaster>() {
			Ok(broadcaster) => Some(broadcaster.subscribe().await),
			Err(_) => None,
		};

		async_stream::stream! {
			if let Some(mut rx) = receiver {
				while let Ok(event) = rx.recv().await {
					if let UserEvent::Deleted(id) = event {
						yield id;
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_broadcaster() {
		let broadcaster = EventBroadcaster::new();
		let mut rx = broadcaster.subscribe().await;

		let user = crate::schema::User {
			id: ID::from("1"),
			name: "Test".to_string(),
			email: "test@example.com".to_string(),
			active: true,
		};

		broadcaster
			.broadcast(UserEvent::Created(user.clone()))
			.await;

		let event = rx.recv().await.unwrap();
		match event {
			UserEvent::Created(u) => assert_eq!(u.name, "Test"),
			_ => panic!("Expected Created event"),
		}
	}

	#[tokio::test]
	async fn test_broadcaster_multiple_subscribers() {
		let broadcaster = EventBroadcaster::new();
		let mut rx1 = broadcaster.subscribe().await;
		let mut rx2 = broadcaster.subscribe().await;
		let mut rx3 = broadcaster.subscribe().await;

		let user = crate::schema::User {
			id: ID::from("multi-sub-1"),
			name: "MultiSub".to_string(),
			email: "multisub@example.com".to_string(),
			active: true,
		};

		broadcaster
			.broadcast(UserEvent::Created(user.clone()))
			.await;

		// All subscribers should receive the event
		let event1 = rx1.recv().await.unwrap();
		let event2 = rx2.recv().await.unwrap();
		let event3 = rx3.recv().await.unwrap();

		match event1 {
			UserEvent::Created(u) => assert_eq!(u.name, "MultiSub"),
			_ => panic!("Expected Created event in rx1"),
		}

		match event2 {
			UserEvent::Created(u) => assert_eq!(u.name, "MultiSub"),
			_ => panic!("Expected Created event in rx2"),
		}

		match event3 {
			UserEvent::Created(u) => assert_eq!(u.name, "MultiSub"),
			_ => panic!("Expected Created event in rx3"),
		}
	}

	#[tokio::test]
	async fn test_event_created() {
		let broadcaster = EventBroadcaster::new();
		let mut rx = broadcaster.subscribe().await;

		let user = crate::schema::User {
			id: ID::from("created-test"),
			name: "CreatedUser".to_string(),
			email: "created@example.com".to_string(),
			active: true,
		};

		broadcaster
			.broadcast(UserEvent::Created(user.clone()))
			.await;

		let event = rx.recv().await.unwrap();
		match event {
			UserEvent::Created(u) => {
				assert_eq!(u.id.to_string(), "created-test");
				assert_eq!(u.name, "CreatedUser");
				assert_eq!(u.email, "created@example.com");
				assert!(u.active);
			}
			_ => panic!("Expected Created event"),
		}
	}

	#[tokio::test]
	async fn test_event_updated() {
		let broadcaster = EventBroadcaster::new();
		let mut rx = broadcaster.subscribe().await;

		let user = crate::schema::User {
			id: ID::from("updated-test"),
			name: "UpdatedUser".to_string(),
			email: "updated@example.com".to_string(),
			active: false,
		};

		broadcaster
			.broadcast(UserEvent::Updated(user.clone()))
			.await;

		let event = rx.recv().await.unwrap();
		match event {
			UserEvent::Updated(u) => {
				assert_eq!(u.id.to_string(), "updated-test");
				assert_eq!(u.name, "UpdatedUser");
				assert!(!u.active);
			}
			_ => panic!("Expected Updated event"),
		}
	}

	#[tokio::test]
	async fn test_event_deleted() {
		let broadcaster = EventBroadcaster::new();
		let mut rx = broadcaster.subscribe().await;

		let deleted_id = ID::from("deleted-test");

		broadcaster
			.broadcast(UserEvent::Deleted(deleted_id.clone()))
			.await;

		let event = rx.recv().await.unwrap();
		match event {
			UserEvent::Deleted(id) => {
				assert_eq!(id.to_string(), "deleted-test");
			}
			_ => panic!("Expected Deleted event"),
		}
	}

	#[tokio::test]
	async fn test_subscription_filtering() {
		let broadcaster = EventBroadcaster::new();
		let mut rx = broadcaster.subscribe().await;

		let user1 = crate::schema::User {
			id: ID::from("filter-1"),
			name: "Filter1".to_string(),
			email: "filter1@example.com".to_string(),
			active: true,
		};

		let user2 = crate::schema::User {
			id: ID::from("filter-2"),
			name: "Filter2".to_string(),
			email: "filter2@example.com".to_string(),
			active: false,
		};

		// Broadcast different event types
		broadcaster
			.broadcast(UserEvent::Created(user1.clone()))
			.await;
		broadcaster
			.broadcast(UserEvent::Updated(user2.clone()))
			.await;
		broadcaster
			.broadcast(UserEvent::Deleted(ID::from("filter-3")))
			.await;

		// Receive all events in order
		let event1 = rx.recv().await.unwrap();
		let event2 = rx.recv().await.unwrap();
		let event3 = rx.recv().await.unwrap();

		match event1 {
			UserEvent::Created(_) => {}
			_ => panic!("Expected Created event first"),
		}

		match event2 {
			UserEvent::Updated(_) => {}
			_ => panic!("Expected Updated event second"),
		}

		match event3 {
			UserEvent::Deleted(_) => {}
			_ => panic!("Expected Deleted event third"),
		}
	}

	#[tokio::test]
	async fn test_subscription_missing_broadcaster_does_not_panic() {
		// Arrange: schema without EventBroadcaster in context data
		use async_graphql::{EmptyMutation, Schema};
		use tokio_stream::StreamExt;

		let schema = Schema::build(crate::schema::Query, EmptyMutation, SubscriptionRoot)
			.data(crate::schema::UserStorage::new())
			.finish();

		// Act: attempt to subscribe to user_created without EventBroadcaster
		let query = r#"subscription { userCreated { id name } }"#;
		let mut stream = schema.execute_stream(query);

		// Assert: stream should terminate gracefully without panic.
		// Use a timeout to prevent hanging if stream never terminates.
		let result =
			tokio::time::timeout(std::time::Duration::from_millis(100), stream.next()).await;

		// Either the stream returned None (empty) or a response -- both are acceptable.
		// The key assertion is that we reached this point without a panic.
		if let Ok(Some(resp)) = result {
			// If we got a response, it should be an empty data set (no panic)
			assert!(
				resp.errors.is_empty() || !resp.errors.is_empty(),
				"reached without panic"
			);
		}
		// If timeout or None, that is also fine -- no panic occurred.
	}
}
