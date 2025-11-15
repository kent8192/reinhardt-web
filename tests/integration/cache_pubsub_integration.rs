//! Integration tests for cache invalidation pub/sub system
//!
//! These tests verify that CacheInvalidationChannel correctly publishes
//! and subscribes to cache invalidation messages via Redis.

use reinhardt_cache::pubsub::{CacheInvalidationChannel, CacheInvalidationMessage};
use reinhardt_test::fixtures::*;
use rstest::*;
use testcontainers::{ContainerAsync, GenericImage};

#[rstest]
#[tokio::test]
async fn test_pubsub_invalidation(
	#[future] redis_container: (ContainerAsync<GenericImage>, String),
) {
	let (_container, redis_url) = redis_container.await;

	let channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	let mut subscriber = channel.subscribe().await.expect("Failed to subscribe");

	// Publish in background task
	let channel_clone = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	tokio::spawn(async move {
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		channel_clone.invalidate("test_key").await.ok();
	});

	// Receive message
	let msg = subscriber.next_message().await.expect("Failed to receive");

	match msg {
		Some(CacheInvalidationMessage::InvalidateKey { key }) => {
			assert_eq!(key, "test_key");
		}
		_ => panic!("Expected InvalidateKey message"),
	}
}
