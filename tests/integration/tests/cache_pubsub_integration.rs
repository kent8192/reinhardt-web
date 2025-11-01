//! Integration tests for Redis PubSub cache invalidation
//!
//! These tests verify the Redis Pub/Sub implementation for cache invalidation
//! using TestContainers and Redis, testing message publishing, subscription,
//! and multi-subscriber scenarios.

use futures::StreamExt;
use reinhardt_cache::{CacheInvalidationChannel, CacheInvalidationMessage};
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::redis::Redis;
use tokio::time::{Duration, sleep};

/// Set up test Redis container and return the connection URL
async fn setup_test_redis() -> (ContainerAsync<Redis>, String) {
	// Start Redis container
	let redis_container = Redis::default()
		.start()
		.await
		.expect("Failed to start Redis container");

	let port = redis_container
		.get_host_port_ipv4(6379)
		.await
		.expect("Failed to get Redis port");

	let redis_url = format!("redis://127.0.0.1:{}", port);

	(redis_container, redis_url)
}

#[tokio::test]
async fn test_redis_pubsub_basic() {
	let (_container, redis_url) = setup_test_redis().await;

	// Create channel for publishing and subscribing
	let channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	let mut subscriber = channel
		.subscribe()
		.await
		.expect("Failed to create subscriber");

	// Publish message in background task
	let publish_channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create publish channel");

	tokio::spawn(async move {
		sleep(Duration::from_millis(100)).await;
		publish_channel
			.invalidate("test_key")
			.await
			.expect("Failed to publish message");
	});

	// Receive message
	let msg = subscriber
		.next_message()
		.await
		.expect("Failed to receive message");

	match msg {
		Some(CacheInvalidationMessage::InvalidateKey { key }) => {
			assert_eq!(key, "test_key");
		}
		_ => panic!("Expected InvalidateKey message"),
	}
}

#[tokio::test]
async fn test_redis_pubsub_pattern_invalidation() {
	let (_container, redis_url) = setup_test_redis().await;

	// Create channel for publishing and subscribing
	let channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	let mut subscriber = channel
		.subscribe()
		.await
		.expect("Failed to create subscriber");

	// Publish pattern invalidation message
	let publish_channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create publish channel");

	tokio::spawn(async move {
		sleep(Duration::from_millis(100)).await;
		publish_channel
			.invalidate_pattern("user:*")
			.await
			.expect("Failed to publish pattern message");
	});

	// Receive message
	let msg = subscriber
		.next_message()
		.await
		.expect("Failed to receive message");

	match msg {
		Some(CacheInvalidationMessage::InvalidatePattern { pattern }) => {
			assert_eq!(pattern, "user:*");
		}
		_ => panic!("Expected InvalidatePattern message"),
	}
}

#[tokio::test]
async fn test_redis_pubsub_clear_all() {
	let (_container, redis_url) = setup_test_redis().await;

	// Create channel for publishing and subscribing
	let channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	let mut subscriber = channel
		.subscribe()
		.await
		.expect("Failed to create subscriber");

	// Publish clear all message
	let publish_channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create publish channel");

	tokio::spawn(async move {
		sleep(Duration::from_millis(100)).await;
		publish_channel
			.clear_all()
			.await
			.expect("Failed to publish clear all message");
	});

	// Receive message
	let msg = subscriber
		.next_message()
		.await
		.expect("Failed to receive message");

	match msg {
		Some(CacheInvalidationMessage::ClearAll) => {
			// Success
		}
		_ => panic!("Expected ClearAll message"),
	}
}

#[tokio::test]
async fn test_redis_pubsub_multiple_subscribers() {
	let (_container, redis_url) = setup_test_redis().await;

	// Create channel
	let channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	// Create two subscribers
	let mut subscriber1 = channel
		.subscribe()
		.await
		.expect("Failed to create subscriber1");

	let channel2 = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel2");

	let mut subscriber2 = channel2
		.subscribe()
		.await
		.expect("Failed to create subscriber2");

	// Publish message
	let publish_channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create publish channel");

	tokio::spawn(async move {
		sleep(Duration::from_millis(100)).await;
		publish_channel
			.invalidate("multi_key")
			.await
			.expect("Failed to publish message");
	});

	// Both subscribers should receive the message
	let msg1 = subscriber1
		.next_message()
		.await
		.expect("Failed to receive message from subscriber1");

	let msg2 = subscriber2
		.next_message()
		.await
		.expect("Failed to receive message from subscriber2");

	// Verify both received the same message
	match (msg1, msg2) {
		(
			Some(CacheInvalidationMessage::InvalidateKey { key: key1 }),
			Some(CacheInvalidationMessage::InvalidateKey { key: key2 }),
		) => {
			assert_eq!(key1, "multi_key");
			assert_eq!(key2, "multi_key");
		}
		_ => panic!("Expected InvalidateKey messages from both subscribers"),
	}
}

#[tokio::test]
async fn test_redis_pubsub_multiple_messages() {
	let (_container, redis_url) = setup_test_redis().await;

	// Create channel
	let channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create channel");

	let mut subscriber = channel
		.subscribe()
		.await
		.expect("Failed to create subscriber");

	// Publish multiple messages
	let publish_channel = CacheInvalidationChannel::new(&redis_url)
		.await
		.expect("Failed to create publish channel");

	tokio::spawn(async move {
		sleep(Duration::from_millis(100)).await;
		publish_channel
			.invalidate("key1")
			.await
			.expect("Failed to publish message 1");
		publish_channel
			.invalidate("key2")
			.await
			.expect("Failed to publish message 2");
		publish_channel
			.invalidate_pattern("user:*")
			.await
			.expect("Failed to publish message 3");
	});

	// Receive all three messages
	let msg1 = subscriber
		.next_message()
		.await
		.expect("Failed to receive message 1");
	let msg2 = subscriber
		.next_message()
		.await
		.expect("Failed to receive message 2");
	let msg3 = subscriber
		.next_message()
		.await
		.expect("Failed to receive message 3");

	// Verify messages
	match msg1 {
		Some(CacheInvalidationMessage::InvalidateKey { key }) => {
			assert_eq!(key, "key1");
		}
		_ => panic!("Expected InvalidateKey message for key1"),
	}

	match msg2 {
		Some(CacheInvalidationMessage::InvalidateKey { key }) => {
			assert_eq!(key, "key2");
		}
		_ => panic!("Expected InvalidateKey message for key2"),
	}

	match msg3 {
		Some(CacheInvalidationMessage::InvalidatePattern { pattern }) => {
			assert_eq!(pattern, "user:*");
		}
		_ => panic!("Expected InvalidatePattern message"),
	}
}

#[tokio::test]
async fn test_redis_pubsub_custom_channel_name() {
	let (_container, redis_url) = setup_test_redis().await;

	// Create channel with custom name
	let custom_channel_name = "custom:cache:invalidation".to_string();
	let channel =
		CacheInvalidationChannel::with_channel_name(&redis_url, custom_channel_name.clone())
			.await
			.expect("Failed to create channel");

	let mut subscriber = channel
		.subscribe()
		.await
		.expect("Failed to create subscriber");

	// Publish message
	let publish_channel =
		CacheInvalidationChannel::with_channel_name(&redis_url, custom_channel_name)
			.await
			.expect("Failed to create publish channel");

	tokio::spawn(async move {
		sleep(Duration::from_millis(100)).await;
		publish_channel
			.invalidate("custom_key")
			.await
			.expect("Failed to publish message");
	});

	// Receive message
	let msg = subscriber
		.next_message()
		.await
		.expect("Failed to receive message");

	match msg {
		Some(CacheInvalidationMessage::InvalidateKey { key }) => {
			assert_eq!(key, "custom_key");
		}
		_ => panic!("Expected InvalidateKey message"),
	}
}
