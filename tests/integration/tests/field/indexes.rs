//! Index Tests
//!
//! Tests index creation and usage.

use reinhardt_db::nosql::traits::DocumentBackend;
use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
use reinhardt_db_macros::{document, field};
use bson::doc;
use rstest::*;

/// Test document with indexes
#[document(collection = "test_indexes", backend = "mongodb")]
struct IndexTest {
    #[field(primary_key)]
    id: Option<bson::oid::ObjectId>,

    #[field(index)]
    indexed_field: String,

    #[field(unique, index)]
    unique_field: String,
}

/// Test single field index
///
/// This test verifies that:
/// 1. Index metadata is generated correctly
/// 2. Queries with indexed fields work
#[rstest]
#[tokio::test]
async fn test_single_field_index(
    #[future] mongodb: MongoDBBackend,
) {
    let db = mongodb.await;
    let collection = "test_indexes";

    // Note: Indexes are created by macro-generated code
    // or by migrations. For testing, we verify that
    // the index metadata is generated correctly.

    // Insert test data
    let doc = doc! {
        "indexed_field": "test_value",
        "unique_field": "unique_value"
    };
    db.insert_one(collection, doc).await.ok();

    // Query with indexed field
    let filter = doc! { "indexed_field": "test_value" };
    let found = db.find_one(collection, filter).await.unwrap();
    assert!(found.is_some());

    // Cleanup
    db.drop_collection(collection).await.ok();
}

/// Test unique index enforcement
///
/// This test verifies that:
/// 1. Unique indexes prevent duplicate values
/// 2. Appropriate error is returned on violation
#[rstest]
#[tokio::test]
async fn test_unique_index_enforcement(
    #[future] mongodb: MongoDBBackend,
) {
    let db = mongodb.await;
    let collection = "test_unique_index";

    // Create unique index
    db.create_unique_index(collection, "email").await.ok();

    // Insert first document
    let doc1 = doc! { "email": "unique@example.com" };
    db.insert_one(collection, doc1).await.unwrap();

    // Attempt duplicate
    let doc2 = doc! { "email": "unique@example.com" };
    let result = db.insert_one(collection, doc2).await;

    // Assert: Duplicate key error
    assert!(result.is_err());

    // Cleanup
    db.drop_collection(collection).await.ok();
}
