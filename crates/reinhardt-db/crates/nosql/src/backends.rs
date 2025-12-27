//! NoSQL backend implementations
//!
//! This module contains concrete implementations of NoSQL backends
//! for various databases:
//! - MongoDB (document-oriented) - Phase 1 (Implemented)
//! - Redis (key-value) - Phase 2 (Planned)
//! - Cassandra (column-family) - Phase 3 (Planned)
//! - DynamoDB (key-value) - Phase 4 (Planned)
//! - Neo4j (graph) - Phase 4 (Planned)

#[cfg(feature = "mongodb")]
pub mod mongodb;

// Future implementation roadmap (Phase 2-4):
// - Phase 2: Redis integration
// - Phase 3: Cassandra integration
// - Phase 4: DynamoDB & Neo4j integration
// Uncommenting these modules requires implementing the corresponding backend traits.
// #[cfg(feature = "redis")]
// pub mod redis;
// #[cfg(feature = "cassandra")]
// pub mod cassandra;
// #[cfg(feature = "dynamodb")]
// pub mod dynamodb;
// #[cfg(feature = "neo4j")]
// pub mod neo4j;
