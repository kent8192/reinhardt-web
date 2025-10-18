use async_graphql::{Context, Subscription, ID};
use futures_util::Stream;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

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
        let broadcaster = ctx.data::<EventBroadcaster>().unwrap();
        let mut rx = broadcaster.subscribe().await;

        async_stream::stream! {
            while let Ok(event) = rx.recv().await {
                if let UserEvent::Created(user) = event {
                    yield user;
                }
            }
        }
    }

    async fn user_updated<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = crate::schema::User> + 'ctx {
        let broadcaster = ctx.data::<EventBroadcaster>().unwrap();
        let mut rx = broadcaster.subscribe().await;

        async_stream::stream! {
            while let Ok(event) = rx.recv().await {
                if let UserEvent::Updated(user) = event {
                    yield user;
                }
            }
        }
    }

    async fn user_deleted<'ctx>(&self, ctx: &Context<'ctx>) -> impl Stream<Item = ID> + 'ctx {
        let broadcaster = ctx.data::<EventBroadcaster>().unwrap();
        let mut rx = broadcaster.subscribe().await;

        async_stream::stream! {
            while let Ok(event) = rx.recv().await {
                if let UserEvent::Deleted(id) = event {
                    yield id;
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
                assert_eq!(u.active, true);
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
                assert_eq!(u.active, false);
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
}
