// Router Lifecycle and Event integration tests
// Inspired by Django signals and SQLAlchemy events

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_signals::{post_save, pre_save, Signal, SignalError};
use reinhardt_types::{Handler, Request, Response};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// Test model
#[derive(Debug, Clone)]
struct User {
    id: i64,
    username: String,
    email: String,
}

// Mock handler that triggers signals
#[derive(Clone)]
struct SignalTriggeringHandler {
    signal: Signal<User>,
}

impl SignalTriggeringHandler {
    fn new(signal: Signal<User>) -> Self {
        Self { signal }
    }
}

#[async_trait]
impl Handler for SignalTriggeringHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        // Create a user and send signal
        let user = User {
            id: 1,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
        };

        self.signal
            .send(user)
            .await
            .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

        Ok(Response::ok().with_body(Bytes::from("Signal sent")))
    }
}

// Test 1: Basic signal connection and emission
#[tokio::test]
async fn test_basic_signal_lifecycle() {
    let signal = Signal::new("test");
    let counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = Arc::clone(&counter);
    signal.connect(move |_user: Arc<User>| {
        let counter = Arc::clone(&counter_clone);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let user = User {
        id: 1,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    signal
        .send(user)
        .await
        .expect("Signal should send successfully");

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// Test 2: Pre-save and post-save signal integration
#[tokio::test]
async fn test_router_lifecycle_save_signals() {
    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));

    // Connect to pre_save
    let pre_clone = Arc::clone(&pre_counter);
    pre_save::<User>().connect(move |_user| {
        let counter = Arc::clone(&pre_clone);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    // Connect to post_save
    let post_clone = Arc::clone(&post_counter);
    post_save::<User>().connect(move |_user| {
        let counter = Arc::clone(&post_clone);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let user = User {
        id: 1,
        username: "bob".to_string(),
        email: "bob@example.com".to_string(),
    };

    // Simulate save operation
    pre_save::<User>().send(user.clone()).await.unwrap();
    // ... actual save would happen here ...
    post_save::<User>().send(user).await.unwrap();

    assert_eq!(pre_counter.load(Ordering::SeqCst), 1);
    assert_eq!(post_counter.load(Ordering::SeqCst), 1);
}

// Test 3: Signal handler in HTTP request lifecycle
#[tokio::test]
async fn test_signal_in_request_lifecycle() {
    let signal = Signal::new("user_created");
    let counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = Arc::clone(&counter);
    signal.connect(move |user: Arc<User>| {
        let counter = Arc::clone(&counter_clone);
        async move {
            assert_eq!(user.username, "testuser");
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let handler = Arc::new(SignalTriggeringHandler::new(signal));

    let request = Request::new(
        Method::POST,
        Uri::from_static("/users/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// Test 4: Multiple signal receivers
#[tokio::test]
async fn test_multiple_signal_receivers() {
    let signal = Signal::new("test");
    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));
    let counter3 = Arc::new(AtomicUsize::new(0));

    // Connect multiple receivers
    let c1 = Arc::clone(&counter1);
    signal.connect(move |_user: Arc<User>| {
        let counter = Arc::clone(&c1);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let c2 = Arc::clone(&counter2);
    signal.connect(move |_user: Arc<User>| {
        let counter = Arc::clone(&c2);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let c3 = Arc::clone(&counter3);
    signal.connect(move |_user: Arc<User>| {
        let counter = Arc::clone(&c3);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let user = User {
        id: 1,
        username: "test".to_string(),
        email: "test@example.com".to_string(),
    };

    signal.send(user).await.unwrap();

    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
    assert_eq!(counter3.load(Ordering::SeqCst), 1);
}

// Test 5: Signal disconnection
#[tokio::test]
async fn test_signal_disconnection() {
    let signal = Signal::new("test");
    let counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = Arc::clone(&counter);
    signal.connect_with_options(
        move |_user: Arc<User>| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        },
        None,
        Some("test_receiver".to_string()),
        0,
    );

    let user = User {
        id: 1,
        username: "test".to_string(),
        email: "test@example.com".to_string(),
    };

    // Send signal - should trigger receiver
    signal.send(user.clone()).await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Disconnect the receiver
    assert!(signal.disconnect("test_receiver"));

    // Send again - should NOT trigger receiver
    signal.send(user).await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// Test 6: Robust signal handling (error tolerance)
#[tokio::test]
async fn test_robust_signal_handling() {
    let signal = Signal::new("test");

    // Connect a failing receiver
    signal
        .connect(|_user: Arc<User>| async move { Err(SignalError::new("First receiver failed")) });

    // Connect a successful receiver
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    signal.connect(move |_user: Arc<User>| {
        let counter = Arc::clone(&counter_clone);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let user = User {
        id: 1,
        username: "test".to_string(),
        email: "test@example.com".to_string(),
    };

    let results = signal.send_robust(user, None).await;

    assert_eq!(results.len(), 2);
    assert!(results[0].is_err());
    assert!(results[1].is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
