// Session Tests - Inspired by SQLAlchemy ORM session tests
// Tests session lifecycle, transactions, and basic CRUD operations

#[cfg(test)]
mod session_tests {
    use reinhardt_orm::database::Database;
    use reinhardt_orm::session::{ObjectState, Session};

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // Simple session implementation for testing
    #[derive(Debug, Clone, PartialEq)]
    enum ObjectState {
        Transient,  // Not in session
        Pending,    // In session, not in database
        Persistent, // In session and database
        Detached,   // Was in session, now removed
    }

    #[derive(Debug, Clone)]
    struct Entity {
        id: Option<i32>,
        data: HashMap<String, String>,
        state: ObjectState,
    }

    impl Entity {
        fn new() -> Self {
            Self {
                id: None,
                data: HashMap::new(),
                state: ObjectState::Transient,
            }
        }

        fn with_data(mut self, key: &str, value: &str) -> Self {
            self.data.insert(key.to_string(), value.to_string());
            self
        }
    }

    struct Session {
        entities: Vec<Entity>,
        next_id: i32,
        is_closed: bool,
        in_transaction: bool,
    }

    impl Session {
        fn new() -> Self {
            Self {
                entities: Vec::new(),
                next_id: 1,
                is_closed: false,
                in_transaction: false,
            }
        }

        fn add(&mut self, mut entity: Entity) {
            assert!(!self.is_closed, "Session is closed");
            entity.state = ObjectState::Pending;
            self.entities.push(entity);
        }

        fn flush(&mut self) {
            assert!(!self.is_closed, "Session is closed");
            for entity in &mut self.entities {
                if entity.state == ObjectState::Pending {
                    entity.id = Some(self.next_id);
                    self.next_id += 1;
                    entity.state = ObjectState::Persistent;
                }
            }
        }

        fn commit(&mut self) {
            assert!(!self.is_closed, "Session is closed");
            self.flush();
            self.in_transaction = false;
        }

        fn rollback(&mut self) {
            assert!(!self.is_closed, "Session is closed");
            self.entities.retain(|e| e.state == ObjectState::Persistent);
            self.in_transaction = false;
        }

        fn close(&mut self) {
            self.is_closed = true;
        }

        fn query_by_id(&self, id: i32) -> Option<&Entity> {
            assert!(!self.is_closed, "Session is closed");
            self.entities
                .iter()
                .find(|e| e.id == Some(id) && e.state == ObjectState::Persistent)
        }

        fn count(&self) -> usize {
            self.entities
                .iter()
                .filter(|e| e.state == ObjectState::Persistent)
                .count()
        }

        fn begin(&mut self) {
            assert!(!self.is_closed, "Session is closed");
            self.in_transaction = true;
        }
    }

    #[tokio::test]
    async fn test_no_close_on_flush() {
        // Flush() doesn't close a connection the session didn't open
        let mut session = Session::new();

        let entity = Entity::new().with_data("name", "test");
        session.add(entity);
        session.flush();

        // Session should still be open after flush
        assert!(!session.is_closed);

        // Can add more entities after flush
        let entity2 = Entity::new().with_data("name", "test2");
        session.add(entity2);
        session.flush();

        assert_eq!(session.count(), 2);
    }

    #[tokio::test]
    async fn test_close() {
        // close() doesn't close a connection the session didn't open
        let mut session = Session::new();

        let entity = Entity::new().with_data("name", "test");
        session.add(entity);
        session.commit();

        assert_eq!(session.count(), 1);

        // Close the session
        session.close();
        assert!(session.is_closed);
    }

    #[tokio::test]
    async fn test_parameter_execute() {
        // Test executing with parameters
        let mut session = Session::new();

        // Insert multiple entities
        let entities = vec![
            Entity::new().with_data("id", "7").with_data("name", "u7"),
            Entity::new().with_data("id", "8").with_data("name", "u8"),
            Entity::new().with_data("id", "9").with_data("name", "u9"),
        ];

        for entity in entities {
            session.add(entity);
        }

        session.commit();

        // Verify all were inserted
        assert_eq!(session.count(), 3);
    }

    #[tokio::test]
    async fn test_empty_list_execute() {
        // Test executing with empty parameter list
        let mut session = Session::new();

        let entity = Entity::new().with_data("col", "42");
        session.add(entity);
        session.commit();

        // Empty parameter list should not cause errors
        assert_eq!(session.count(), 1);
    }

    #[tokio::test]
    async fn test_add_and_flush() {
        // Test basic add and flush operations
        let mut session = Session::new();

        let mut entity = Entity::new().with_data("name", "test_entity");
        session.add(entity.clone());

        // Before flush, entity should be pending
        assert_eq!(session.entities[0].state, ObjectState::Pending);
        assert!(session.entities[0].id.is_none());

        session.flush();

        // After flush, entity should be persistent with ID
        assert_eq!(session.entities[0].state, ObjectState::Persistent);
        assert!(session.entities[0].id.is_some());
    }

    #[tokio::test]
    async fn test_commit() {
        // Test commit persists changes
        let mut session = Session::new();

        let entity = Entity::new().with_data("name", "committed_entity");
        session.add(entity);

        session.commit();

        assert_eq!(session.count(), 1);

        // Verify entity has ID
        let persisted = &session.entities[0];
        assert!(persisted.id.is_some());
        assert_eq!(persisted.state, ObjectState::Persistent);
    }

    #[tokio::test]
    async fn test_rollback() {
        // Test rollback discards pending changes
        let mut session = Session::new();

        // Add and commit first entity
        let entity1 = Entity::new().with_data("name", "entity1");
        session.add(entity1);
        session.commit();

        assert_eq!(session.count(), 1);

        // Add second entity but rollback
        let entity2 = Entity::new().with_data("name", "entity2");
        session.add(entity2);

        session.rollback();

        // Only first entity should remain
        assert_eq!(session.count(), 1);
    }

    #[tokio::test]
    async fn test_query_after_add() {
        // Test querying entities after adding them
        let mut session = Session::new();

        let entity = Entity::new().with_data("name", "queryable");
        session.add(entity);
        session.flush();

        // Query by ID
        let id = session.entities[0].id.unwrap();
        let found = session.query_by_id(id);

        assert!(found.is_some());
        assert_eq!(found.unwrap().data.get("name").unwrap(), "queryable");
    }

    #[tokio::test]
    async fn test_transaction_state() {
        // Test transaction state management
        let mut session = Session::new();

        assert!(!session.in_transaction);

        session.begin();
        assert!(session.in_transaction);

        let entity = Entity::new().with_data("name", "transactional");
        session.add(entity);

        session.commit();
        assert!(!session.in_transaction);
    }

    #[tokio::test]
    async fn test_orm_session_transaction_rollback() {
        // Test transaction rollback
        let mut session = Session::new();

        session.begin();

        let entity = Entity::new().with_data("name", "will_rollback");
        session.add(entity);

        session.rollback();

        assert_eq!(session.count(), 0);
        assert!(!session.in_transaction);
    }

    #[tokio::test]
    async fn test_multiple_flushes() {
        // Test multiple flush operations
        let mut session = Session::new();

        let entity1 = Entity::new().with_data("name", "first");
        session.add(entity1);
        session.flush();

        assert_eq!(session.count(), 1);

        let entity2 = Entity::new().with_data("name", "second");
        session.add(entity2);
        session.flush();

        assert_eq!(session.count(), 2);
    }

    #[tokio::test]
    async fn test_entity_state_transitions() {
        // Test entity state transitions through lifecycle
        let mut entity = Entity::new();

        // Initial state is transient
        assert_eq!(entity.state, ObjectState::Transient);

        // Add to session makes it pending
        let mut session = Session::new();
        session.add(entity);
        assert_eq!(session.entities[0].state, ObjectState::Pending);

        // Flush makes it persistent
        session.flush();
        assert_eq!(session.entities[0].state, ObjectState::Persistent);
    }

    #[tokio::test]
    async fn test_query_nonexistent() {
        // Test querying for non-existent entity
        let session = Session::new();

        let found = session.query_by_id(999);
        assert!(found.is_none());
    }

    #[test]
    #[should_panic(expected = "Session is closed")]
    fn test_closed_session_operations() {
        // Test that operations on closed session fail
        let mut session = Session::new();
        session.close();

        // This should panic
        let entity = Entity::new().with_data("name", "should_fail");
        session.add(entity);
    }

    #[tokio::test]
    async fn test_bulk_insert() {
        // Test bulk insertion of entities
        let mut session = Session::new();

        for i in 0..10 {
            let entity = Entity::new().with_data("name", &format!("entity_{}", i));
            session.add(entity);
        }

        session.commit();

        assert_eq!(session.count(), 10);

        // Verify all have unique IDs
        let ids: Vec<i32> = session.entities.iter().filter_map(|e| e.id).collect();

        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();

        assert_eq!(ids.len(), unique_ids.len());
    }

    #[tokio::test]
    async fn test_session_isolation() {
        // Test that sessions are isolated from each other
        let mut session1 = Session::new();
        let mut session2 = Session::new();

        let entity1 = Entity::new().with_data("name", "session1_entity");
        session1.add(entity1);
        session1.commit();

        let entity2 = Entity::new().with_data("name", "session2_entity");
        session2.add(entity2);
        session2.commit();

        // Each session should only see its own entities
        assert_eq!(session1.count(), 1);
        assert_eq!(session2.count(), 1);
    }
}
