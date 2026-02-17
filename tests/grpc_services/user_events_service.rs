use crate::proto::user_events::{
	SubscribeUserEventsRequest, UserEvent, user_events_service_server::UserEventsService,
};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status};

// Stream type that maps BroadcastStream to Status error type
type UserEventStream =
	std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<UserEvent, Status>> + Send>>;

/// User event broadcaster
#[derive(Clone)]
pub struct UserEventsBroadcaster {
	tx: broadcast::Sender<UserEvent>,
}

impl UserEventsBroadcaster {
	/// Create a new broadcaster
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_grpc::services::UserEventsBroadcaster;
	///
	/// let broadcaster = UserEventsBroadcaster::new();
	/// ```
	pub fn new() -> Self {
		let (tx, _) = broadcast::channel(100);
		Self { tx }
	}

	/// Broadcast an event
	pub fn broadcast(&self, event: UserEvent) {
		let _ = self.tx.send(event);
	}

	/// Get a subscriber
	pub fn subscribe(&self) -> broadcast::Receiver<UserEvent> {
		self.tx.subscribe()
	}
}

impl Default for UserEventsBroadcaster {
	fn default() -> Self {
		Self::new()
	}
}

/// Implementation of UserEventsService
pub struct UserEventsServiceImpl {
	broadcaster: UserEventsBroadcaster,
}

impl UserEventsServiceImpl {
	/// Create a new service
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_grpc::services::{UserEventsServiceImpl, UserEventsBroadcaster};
	///
	/// let broadcaster = UserEventsBroadcaster::new();
	/// let service = UserEventsServiceImpl::new(broadcaster);
	/// ```
	pub fn new(broadcaster: UserEventsBroadcaster) -> Self {
		Self { broadcaster }
	}
}

#[tonic::async_trait]
impl UserEventsService for UserEventsServiceImpl {
	type SubscribeUserEventsStream = UserEventStream;

	async fn subscribe_user_events(
		&self,
		request: Request<SubscribeUserEventsRequest>,
	) -> Result<Response<Self::SubscribeUserEventsStream>, Status> {
		let req = request.into_inner();
		let rx = self.broadcaster.subscribe();

		// Extract filter parameters
		let event_types: Vec<i32> = req
			.event_types
			.iter()
			.filter_map(|s| s.parse::<i32>().ok())
			.collect();
		let user_id_filter = req.user_id.filter(|id| !id.is_empty());

		// Convert BroadcastStream to gRPC Status type and filter
		let stream = BroadcastStream::new(rx)
			.filter(move |result| {
				if let Ok(event) = result {
					// Filter by event_types if specified
					let type_match = event_types.is_empty() || event_types.contains(&event.r#type);

					// Filter by user_id if specified
					let user_match = user_id_filter
						.as_ref()
						.map_or(true, |filter_id| &event.user_id == filter_id);

					type_match && user_match
				} else {
					// Keep error results
					true
				}
			})
			.map(|result| result.map_err(|e| Status::internal(format!("Broadcast error: {}", e))));

		Ok(Response::new(Box::pin(stream)))
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;
	use super::*;
	use tokio_stream::StreamExt;

	#[rstest]
	#[tokio::test]
	async fn test_broadcaster_creation() {
		let broadcaster = UserEventsBroadcaster::new();
		let _subscriber = broadcaster.subscribe();
	}

	#[rstest]
	#[tokio::test]
	async fn test_broadcast_event() {
		let broadcaster = UserEventsBroadcaster::new();
		let mut subscriber = broadcaster.subscribe();

		let event = UserEvent {
			r#type: 0,
			user: None,
			user_id: "test-id".to_string(),
			timestamp: None,
		};

		broadcaster.broadcast(event.clone());

		let received = subscriber.recv().await.unwrap();
		assert_eq!(received.user_id, "test-id");
	}

	#[rstest]
	#[tokio::test]
	async fn test_multiple_subscribers() {
		let broadcaster = UserEventsBroadcaster::new();
		let mut sub1 = broadcaster.subscribe();
		let mut sub2 = broadcaster.subscribe();

		let event = UserEvent {
			r#type: 0,
			user: None,
			user_id: "multi-test".to_string(),
			timestamp: None,
		};

		broadcaster.broadcast(event);

		let received1 = sub1.recv().await.unwrap();
		let received2 = sub2.recv().await.unwrap();

		assert_eq!(received1.user_id, "multi-test");
		assert_eq!(received2.user_id, "multi-test");
	}

	#[rstest]
	#[tokio::test]
	async fn test_subscribe_with_event_type_filter() {
		let broadcaster = UserEventsBroadcaster::new();
		let service = UserEventsServiceImpl::new(broadcaster.clone());

		// Subscribe with event type filter (only CREATED events = type 0)
		let request = Request::new(SubscribeUserEventsRequest {
			event_types: vec!["0".to_string()],
			user_id: None,
		});

		let response = service.subscribe_user_events(request).await.unwrap();
		let mut stream = response.into_inner();

		// Broadcast different event types
		broadcaster.broadcast(UserEvent {
			r#type: 0,
			user: None,
			user_id: "user1".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 1,
			user: None,
			user_id: "user2".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 0,
			user: None,
			user_id: "user3".to_string(),
			timestamp: None,
		});

		// Should only receive type 0 events
		let event1 = stream.next().await.unwrap().unwrap();
		assert_eq!(event1.r#type, 0);
		assert_eq!(event1.user_id, "user1");

		let event2 = stream.next().await.unwrap().unwrap();
		assert_eq!(event2.r#type, 0);
		assert_eq!(event2.user_id, "user3");
	}

	#[rstest]
	#[tokio::test]
	async fn test_subscribe_with_user_id_filter() {
		let broadcaster = UserEventsBroadcaster::new();
		let service = UserEventsServiceImpl::new(broadcaster.clone());

		// Subscribe with user_id filter
		let request = Request::new(SubscribeUserEventsRequest {
			event_types: vec![],
			user_id: Some("target-user".to_string()),
		});

		let response = service.subscribe_user_events(request).await.unwrap();
		let mut stream = response.into_inner();

		// Broadcast events for different users
		broadcaster.broadcast(UserEvent {
			r#type: 0,
			user: None,
			user_id: "target-user".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 1,
			user: None,
			user_id: "other-user".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 2,
			user: None,
			user_id: "target-user".to_string(),
			timestamp: None,
		});

		// Should only receive events for target-user
		let event1 = stream.next().await.unwrap().unwrap();
		assert_eq!(event1.user_id, "target-user");
		assert_eq!(event1.r#type, 0);

		let event2 = stream.next().await.unwrap().unwrap();
		assert_eq!(event2.user_id, "target-user");
		assert_eq!(event2.r#type, 2);
	}

	#[rstest]
	#[tokio::test]
	async fn test_subscribe_with_combined_filters() {
		let broadcaster = UserEventsBroadcaster::new();
		let service = UserEventsServiceImpl::new(broadcaster.clone());

		// Subscribe with both event_type and user_id filters
		let request = Request::new(SubscribeUserEventsRequest {
			event_types: vec!["0".to_string(), "1".to_string()],
			user_id: Some("target-user".to_string()),
		});

		let response = service.subscribe_user_events(request).await.unwrap();
		let mut stream = response.into_inner();

		// Broadcast various events
		broadcaster.broadcast(UserEvent {
			r#type: 0,
			user: None,
			user_id: "target-user".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 2,
			user: None,
			user_id: "target-user".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 1,
			user: None,
			user_id: "other-user".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 1,
			user: None,
			user_id: "target-user".to_string(),
			timestamp: None,
		});

		// Should only receive type 0 or 1 events for target-user
		let event1 = stream.next().await.unwrap().unwrap();
		assert_eq!(event1.user_id, "target-user");
		assert_eq!(event1.r#type, 0);

		let event2 = stream.next().await.unwrap().unwrap();
		assert_eq!(event2.user_id, "target-user");
		assert_eq!(event2.r#type, 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_subscribe_without_filters() {
		let broadcaster = UserEventsBroadcaster::new();
		let service = UserEventsServiceImpl::new(broadcaster.clone());

		// Subscribe without any filters
		let request = Request::new(SubscribeUserEventsRequest {
			event_types: vec![],
			user_id: None,
		});

		let response = service.subscribe_user_events(request).await.unwrap();
		let mut stream = response.into_inner();

		// Broadcast different events
		broadcaster.broadcast(UserEvent {
			r#type: 0,
			user: None,
			user_id: "user1".to_string(),
			timestamp: None,
		});

		broadcaster.broadcast(UserEvent {
			r#type: 1,
			user: None,
			user_id: "user2".to_string(),
			timestamp: None,
		});

		// Should receive all events
		let event1 = stream.next().await.unwrap().unwrap();
		assert_eq!(event1.user_id, "user1");

		let event2 = stream.next().await.unwrap().unwrap();
		assert_eq!(event2.user_id, "user2");
	}
}
