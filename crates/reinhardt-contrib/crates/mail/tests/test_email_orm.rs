//! ORM integration tests
//!
//! Tests for email integration with reinhardt-orm for bulk email sending.

use reinhardt_mail::{EmailBackend, EmailMessage, MemoryBackend};

/// Mock user model for testing
#[derive(Clone, Debug)]
struct User {
    id: i32,
    name: String,
    email: String,
}

/// Mock queryset-like collection
struct UserQuerySet {
    users: Vec<User>,
}

impl UserQuerySet {
    fn new(users: Vec<User>) -> Self {
        Self { users }
    }

    /// Extract email addresses from a field
    fn extract_emails(&self, _field: &str) -> Vec<String> {
        self.users.iter().map(|u| u.email.clone()).collect()
    }

    /// Send bulk emails using a builder function
    async fn send_bulk_emails<F>(
        &self,
        backend: &dyn EmailBackend,
        builder: F,
    ) -> Vec<Result<(), reinhardt_mail::EmailError>>
    where
        F: Fn(&User) -> EmailMessage,
    {
        let mut results = Vec::new();

        for user in &self.users {
            let message = builder(user);
            let result = backend.send(&message).await;
            results.push(result);
        }

        results
    }
}

#[tokio::test]
async fn test_queryset_extract_emails() {
    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
        },
    ];

    let queryset = UserQuerySet::new(users);
    let emails = queryset.extract_emails("email");

    assert_eq!(emails.len(), 3);
    assert!(emails.contains(&"alice@example.com".to_string()));
    assert!(emails.contains(&"bob@example.com".to_string()));
    assert!(emails.contains(&"charlie@example.com".to_string()));

    // Use extracted emails to send a mass email
    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Announcement")
        .body("Important announcement for all users")
        .from("admin@example.com")
        .to(emails)
        .build()
        .unwrap();

    backend.send(&message).await.unwrap();

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].to.len(), 3);
}

#[tokio::test]
async fn test_queryset_bulk_send() {
    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
        },
    ];

    let queryset = UserQuerySet::new(users);
    let backend = MemoryBackend::new();

    // Send personalized emails to each user
    let results = queryset
        .send_bulk_emails(&backend, |user| {
            EmailMessage::new()
                .subject(format!("Welcome, {}!", user.name))
                .body(format!(
                    "Hello {},\n\nWelcome to our service!\n\nYour user ID is: {}",
                    user.name, user.id
                ))
                .from("welcome@example.com")
                .to(vec![user.email.clone()])
                .build()
                .unwrap()
        })
        .await;

    // Verify all emails were sent successfully
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.is_ok()));

    // Verify emails in backend
    let messages = backend.get_messages();
    assert_eq!(messages.len(), 3);

    // Verify personalization
    assert!(messages[0].subject.contains("Alice"));
    assert!(messages[0].body.contains("Your user ID is: 1"));

    assert!(messages[1].subject.contains("Bob"));
    assert!(messages[1].body.contains("Your user ID is: 2"));

    assert!(messages[2].subject.contains("Charlie"));
    assert!(messages[2].body.contains("Your user ID is: 3"));
}

#[tokio::test]
async fn test_queryset_bulk_with_filtering() {
    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
        },
    ];

    // Filter users (e.g., only users with id > 1)
    let filtered_users: Vec<User> = users.into_iter().filter(|u| u.id > 1).collect();

    let queryset = UserQuerySet::new(filtered_users);
    let backend = MemoryBackend::new();

    let results = queryset
        .send_bulk_emails(&backend, |user| {
            EmailMessage::new()
                .subject("Filtered Notification")
                .body(format!("This email is for user: {}", user.name))
                .from("notifications@example.com")
                .to(vec![user.email.clone()])
                .build()
                .unwrap()
        })
        .await;

    // Only 2 emails should be sent (Bob and Charlie)
    assert_eq!(results.len(), 2);
    assert_eq!(backend.count(), 2);

    let messages = backend.get_messages();
    assert!(messages.iter().any(|m| m.body.contains("Bob")));
    assert!(messages.iter().any(|m| m.body.contains("Charlie")));
    assert!(!messages.iter().any(|m| m.body.contains("Alice")));
}
