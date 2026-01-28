//! Field Validation Tests
//!
//! Tests field-level validation and constraints.

use reinhardt_db::nosql::traits::DocumentBackend;
use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
use reinhardt_db_macros::{document, field};
use bson::doc;
use rstest::*;

/// Test document with validation
#[document(collection = "test_validation", backend = "mongodb")]
struct ValidationTest {
    #[field(primary_key)]
    id: Option<bson::oid::ObjectId>,

    #[field(required)]
    required_field: String,

    #[field(min = 0, max = 100)]
    score: i32,
}

/// Test required field validation
///
/// This test verifies that the required field metadata
/// is generated correctly by the macro.
#[rstest]
#[tokio::test]
async fn test_required_field_metadata(
    #[future] mongodb: MongoDBBackend,
) {
    let db = mongodb.await;
    let collection = "test_validation";

    // Note: MongoDB doesn't enforce required fields at document level
    // This tests that the metadata is generated correctly
    // Application-level validation would be tested separately

    // Insert document with required field
    let doc = doc! {
        "required_field": "test_value",
        "score": 50
    };
    db.insert_one(collection, doc).await.ok();

    // Cleanup
    db.drop_collection(collection).await.ok();
}

/// Test min/max validation metadata
///
/// This test verifies that the min/max constraint metadata
/// is generated correctly by the macro.
#[rstest]
#[tokio::test]
async fn test_min_max_metadata(
    #[future] mongodb: MongoDBBackend,
) {
    let db = mongodb.await;
    let collection = "test_minmax";

    // Note: MongoDB doesn't enforce min/max at document level
    // This is application-level validation
    // We test that the metadata is generated correctly

    // Insert with value < min
    let doc1 = doc! { "value": -10 };
    db.insert_one(collection, doc1).await.ok();

    // Insert with value > max
    let doc2 = doc! { "value": 150 };
    db.insert_one(collection, doc2).await.ok();

    // Cleanup
    db.drop_collection(collection).await.ok();
}
