//! Integration tests for GraphQL Subscriptions over gRPC

#[cfg(feature = "graphql-grpc")]
mod subscription_tests {
    use async_graphql::{ID, Schema};
    use reinhardt_graphql::{
        grpc_service::GraphQLGrpcService,
        schema::{Mutation, Query, User},
        subscription::{EventBroadcaster, SubscriptionRoot, UserEvent},
    };
    use reinhardt_grpc::proto::graphql::{GraphQlRequest, graph_ql_service_server::GraphQlService};
    use tokio_stream::StreamExt;
    use tonic::Request;

    #[tokio::test]
    async fn test_subscription_user_created() {
        // Create schema with subscription support
        let broadcaster = EventBroadcaster::new();
        let schema = Schema::build(Query, Mutation, SubscriptionRoot)
            .data(broadcaster.clone())
            .finish();

        let service = GraphQLGrpcService::new(schema);

        // Create subscription request
        let request = Request::new(GraphQlRequest {
            query: r#"subscription { userCreated { id name email active } }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute subscription
        let response = service.execute_subscription(request).await.unwrap();
        let mut stream = response.into_inner();

        // Broadcast a user created event in a separate task
        let broadcaster_clone = broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let user = User {
                id: ID::from("sub-test-1"),
                name: "Subscription Test".to_string(),
                email: "sub@test.com".to_string(),
                active: true,
            };

            broadcaster_clone.broadcast(UserEvent::Created(user)).await;
        });

        // Wait for the event
        let event = tokio::time::timeout(tokio::time::Duration::from_secs(2), stream.next())
            .await
            .expect("Timeout waiting for subscription event")
            .expect("Stream should yield an event")
            .expect("Event should be Ok");

        // Verify event data
        assert!(event.payload.is_some());
        let payload = event.payload.unwrap();
        assert!(payload.data.is_some());

        let data = payload.data.unwrap();
        assert!(data.contains("sub-test-1"));
        assert!(data.contains("Subscription Test"));
        assert!(data.contains("sub@test.com"));
        assert!(payload.errors.is_empty());
    }

    #[tokio::test]
    async fn test_subscription_user_updated() {
        // Create schema with subscription support
        let broadcaster = EventBroadcaster::new();
        let schema = Schema::build(Query, Mutation, SubscriptionRoot)
            .data(broadcaster.clone())
            .finish();

        let service = GraphQLGrpcService::new(schema);

        // Create subscription request
        let request = Request::new(GraphQlRequest {
            query: r#"subscription { userUpdated { id name email active } }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute subscription
        let response = service.execute_subscription(request).await.unwrap();
        let mut stream = response.into_inner();

        // Broadcast a user updated event
        let broadcaster_clone = broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let user = User {
                id: ID::from("update-test-1"),
                name: "Updated User".to_string(),
                email: "updated@test.com".to_string(),
                active: false,
            };

            broadcaster_clone.broadcast(UserEvent::Updated(user)).await;
        });

        // Wait for the event
        let event = tokio::time::timeout(tokio::time::Duration::from_secs(2), stream.next())
            .await
            .expect("Timeout waiting for subscription event")
            .expect("Stream should yield an event")
            .expect("Event should be Ok");

        // Verify event data
        assert!(event.payload.is_some());
        let payload = event.payload.unwrap();
        assert!(payload.data.is_some());

        let data = payload.data.unwrap();
        assert!(data.contains("update-test-1"));
        assert!(data.contains("Updated User"));
        assert!(data.contains("false")); // active: false
    }

    #[tokio::test]
    async fn test_subscription_user_deleted() {
        // Create schema with subscription support
        let broadcaster = EventBroadcaster::new();
        let schema = Schema::build(Query, Mutation, SubscriptionRoot)
            .data(broadcaster.clone())
            .finish();

        let service = GraphQLGrpcService::new(schema);

        // Create subscription request
        let request = Request::new(GraphQlRequest {
            query: r#"subscription { userDeleted }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute subscription
        let response = service.execute_subscription(request).await.unwrap();
        let mut stream = response.into_inner();

        // Broadcast a user deleted event
        let broadcaster_clone = broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            broadcaster_clone
                .broadcast(UserEvent::Deleted(ID::from("deleted-123")))
                .await;
        });

        // Wait for the event
        let event = tokio::time::timeout(tokio::time::Duration::from_secs(2), stream.next())
            .await
            .expect("Timeout waiting for subscription event")
            .expect("Stream should yield an event")
            .expect("Event should be Ok");

        // Verify event data
        assert!(event.payload.is_some());
        let payload = event.payload.unwrap();
        assert!(payload.data.is_some());

        let data = payload.data.unwrap();
        assert!(data.contains("deleted-123"));
    }

    #[tokio::test]
    async fn test_subscription_multiple_events() {
        // Create schema with subscription support
        let broadcaster = EventBroadcaster::new();
        let schema = Schema::build(Query, Mutation, SubscriptionRoot)
            .data(broadcaster.clone())
            .finish();

        let service = GraphQLGrpcService::new(schema);

        // Create subscription request
        let request = Request::new(GraphQlRequest {
            query: r#"subscription { userCreated { id name } }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute subscription
        let response = service.execute_subscription(request).await.unwrap();
        let mut stream = response.into_inner();

        // Broadcast multiple events
        let broadcaster_clone = broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            for i in 1..=3 {
                let user = User {
                    id: ID::from(format!("multi-{}", i)),
                    name: format!("User {}", i),
                    email: format!("user{}@test.com", i),
                    active: true,
                };

                broadcaster_clone.broadcast(UserEvent::Created(user)).await;

                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        });

        // Receive multiple events
        let mut received_count = 0;
        let timeout_duration = tokio::time::Duration::from_secs(3);

        while received_count < 3 {
            match tokio::time::timeout(timeout_duration, stream.next()).await {
                Ok(Some(Ok(event))) => {
                    received_count += 1;
                    assert!(event.payload.is_some());
                    let payload = event.payload.unwrap();
                    assert!(payload.data.is_some());

                    let data = payload.data.unwrap();
                    assert!(data.contains(&format!("multi-{}", received_count)));
                    assert!(data.contains(&format!("User {}", received_count)));
                }
                Ok(Some(Err(e))) => {
                    panic!("Stream error: {:?}", e);
                }
                Ok(None) => {
                    panic!("Stream ended unexpectedly after {} events", received_count);
                }
                Err(_) => {
                    panic!("Timeout after receiving {} events", received_count);
                }
            }
        }

        assert_eq!(received_count, 3, "Should receive all 3 events");
    }

    #[tokio::test]
    async fn test_subscription_filtering() {
        // Create schema with subscription support
        let broadcaster = EventBroadcaster::new();
        let schema = Schema::build(Query, Mutation, SubscriptionRoot)
            .data(broadcaster.clone())
            .finish();

        let service = GraphQLGrpcService::new(schema);

        // Subscribe only to userCreated events
        let request = Request::new(GraphQlRequest {
            query: r#"subscription { userCreated { id name } }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        let response = service.execute_subscription(request).await.unwrap();
        let mut stream = response.into_inner();

        // Broadcast mixed events
        let broadcaster_clone = broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Send Updated event (should be filtered out)
            let user1 = User {
                id: ID::from("filter-1"),
                name: "Updated".to_string(),
                email: "updated@test.com".to_string(),
                active: true,
            };
            broadcaster_clone.broadcast(UserEvent::Updated(user1)).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Send Created event (should be received)
            let user2 = User {
                id: ID::from("filter-2"),
                name: "Created".to_string(),
                email: "created@test.com".to_string(),
                active: true,
            };
            broadcaster_clone.broadcast(UserEvent::Created(user2)).await;
        });

        // Wait for event - should only receive the Created event
        let event = tokio::time::timeout(tokio::time::Duration::from_secs(2), stream.next())
            .await
            .expect("Timeout waiting for subscription event")
            .expect("Stream should yield an event")
            .expect("Event should be Ok");

        // Verify we got the Created event, not the Updated one
        assert!(event.payload.is_some());
        let payload = event.payload.unwrap();
        assert!(payload.data.is_some());

        let data = payload.data.unwrap();
        assert!(data.contains("filter-2"));
        assert!(data.contains("Created"));
        assert!(!data.contains("filter-1")); // Updated event should be filtered
    }

    #[tokio::test]
    async fn test_subscription_event_metadata() {
        // Create schema with subscription support
        let broadcaster = EventBroadcaster::new();
        let schema = Schema::build(Query, Mutation, SubscriptionRoot)
            .data(broadcaster.clone())
            .finish();

        let service = GraphQLGrpcService::new(schema);

        // Create subscription request
        let request = Request::new(GraphQlRequest {
            query: r#"subscription { userCreated { id } }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        let response = service.execute_subscription(request).await.unwrap();
        let mut stream = response.into_inner();

        // Broadcast event
        let broadcaster_clone = broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let user = User {
                id: ID::from("metadata-test"),
                name: "Metadata".to_string(),
                email: "metadata@test.com".to_string(),
                active: true,
            };

            broadcaster_clone.broadcast(UserEvent::Created(user)).await;
        });

        // Wait for event
        let event = tokio::time::timeout(tokio::time::Duration::from_secs(2), stream.next())
            .await
            .expect("Timeout")
            .expect("Stream event")
            .expect("Event Ok");

        // Verify event metadata
        assert!(!event.id.is_empty(), "Event should have an ID");
        assert_eq!(event.event_type, "data", "Event type should be 'data'");
        assert!(event.timestamp.is_some(), "Event should have timestamp");

        let timestamp = event.timestamp.unwrap();
        assert!(
            timestamp.seconds > 0,
            "Timestamp seconds should be positive"
        );
    }
}
