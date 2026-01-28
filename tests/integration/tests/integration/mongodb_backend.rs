//! MongoDB Backend Tests
//!
//! Tests MongoDB-specific functionality.

use reinhardt_db::nosql::traits::DocumentBackend;
use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
use bson::doc;
use rstest::*;

/// Test MongoDB connection
///
/// This test verifies that:
/// 1. MongoDB backend can be created
/// 2. Connection is successful
#[rstest]
#[tokio::test]
async fn test_mongodb_connection(
    #[future] mongodb: MongoDBBackend,
) {
    // Arrange: Get MongoDB backend
    let db = mongodb.await;

    // Act: Ping database
    let result = db.execute_command("ping", doc! {}).await;

    // Assert: Connection successful
    assert!(result.is_ok());
}

/// Test aggregation pipeline
///
/// This test verifies that:
/// 1. Aggregation pipeline can be executed
/// 2. Results are returned correctly
#[rstest]
#[tokio::test]
async fn test_aggregation_pipeline(
    #[future] mongodb: MongoDBBackend,
) {
    let db = mongodb.await;
    let collection = "test_aggregation";

    // Arrange: Insert test documents
    for i in 1..=10 {
        let doc = doc! { "category": if i % 2 == 0 { "A" } else { "B" }, "value": i };
        db.insert_one(collection, doc).await.ok();
    }

    // Act: Run aggregation
    let pipeline = vec![
        doc! { "$match": { "category": "A" } },
        doc! { "$group": {
            "_id": "$category",
            "total": { "$sum": "$value" }
        }},
    ];

    let results = db.aggregate(collection, pipeline).await.unwrap();

    // Assert: Aggregation results
    assert!(!results.is_empty());

    // Cleanup
    db.drop_collection(collection).await.ok();
}
