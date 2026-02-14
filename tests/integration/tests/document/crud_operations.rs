// TODO: [PR#31] Rewrite to use Repository<T> API instead of low-level DocumentBackend
//! CRUD Operations Tests
//!
//! Tests Create, Read, Update, Delete operations.

use crate::mongodb_fixtures::mongodb;
use bson::{doc, oid::ObjectId};
use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
use reinhardt_db::nosql::traits::DocumentBackend;
use reinhardt_db_macros::document;
use rstest::*;
use serde::{Deserialize, Serialize};
use testcontainers::{ContainerAsync, GenericImage};

/// Test document structure
#[document(collection = "test_users", backend = "mongodb")]
#[derive(Serialize, Deserialize)]
struct TestUser {
	#[field(primary_key)]
	id: Option<ObjectId>,
	#[field(required, unique)]
	email: String,
	name: String,
}

impl TestUser {
	fn new(email: &str, name: &str) -> Self {
		Self {
			id: None,
			email: email.to_string(),
			name: name.to_string(),
		}
	}
}

/// Test document insertion
///
/// This test verifies that:
/// 1. Documents can be inserted into MongoDB
/// 2. The returned ID is valid
#[rstest]
#[tokio::test]
async fn test_insert_one(#[future] mongodb: (ContainerAsync<GenericImage>, MongoDBBackend)) {
	// Arrange: Get MongoDB backend
	let (_container, db) = mongodb.await;
	let user = TestUser::new("test@example.com", "Test User");
	let collection = "test_users";
	let user_doc = bson::serialize_to_document(&user).unwrap();

	// Act: Insert document
	let id = db.insert_one(collection, user_doc).await.unwrap();
	let oid = ObjectId::parse_str(&id).unwrap();

	// Assert: ID is valid
	assert!(!id.is_empty());

	// Cleanup: Remove test document
	db.delete_one(collection, doc! { "_id": oid }).await.ok();
}

/// Test document finding
///
/// This test verifies that:
/// 1. Documents can be retrieved by ID
/// 2. Retrieved data matches inserted data
#[rstest]
#[tokio::test]
async fn test_find_one(#[future] mongodb: (ContainerAsync<GenericImage>, MongoDBBackend)) {
	// Arrange: Insert test document
	let (_container, db) = mongodb.await;
	let user = TestUser::new("find@example.com", "Find User");
	let collection = "test_users";
	let user_doc = bson::serialize_to_document(&user).unwrap();
	let id = db.insert_one(collection, user_doc.clone()).await.unwrap();
	let oid = ObjectId::parse_str(&id).unwrap();

	// Act: Find document by ID
	let filter = doc! { "_id": &oid };
	let found = db.find_one(collection, filter).await.unwrap();

	// Assert: Document found with correct data
	assert!(found.is_some());
	let found_doc = found.unwrap();
	assert_eq!(found_doc.get_str("email").unwrap(), "find@example.com");

	// Cleanup
	db.delete_one(collection, doc! { "_id": oid }).await.ok();
}

/// Test document update
///
/// This test verifies that:
/// 1. Documents can be updated
/// 2. Update operation returns correct counts
/// 3. Updated data is persisted
#[rstest]
#[tokio::test]
async fn test_update_one(#[future] mongodb: (ContainerAsync<GenericImage>, MongoDBBackend)) {
	// Arrange: Insert test document
	let (_container, db) = mongodb.await;
	let user = TestUser::new("update@example.com", "Original Name");
	let collection = "test_users";
	let user_doc = bson::serialize_to_document(&user).unwrap();
	let id = db.insert_one(collection, user_doc).await.unwrap();
	let oid = ObjectId::parse_str(&id).unwrap();

	// Act: Update document
	let filter = doc! { "_id": &oid };
	let update = doc! { "$set": { "name": "Updated Name" } };
	let result = db.update_one(collection, filter, update).await.unwrap();

	// Assert: Update successful
	assert_eq!(result.matched_count, 1);
	assert_eq!(result.modified_count, 1);

	// Verify: Name was updated
	let filter = doc! { "_id": &oid };
	let found = db.find_one(collection, filter).await.unwrap().unwrap();
	assert_eq!(found.get_str("name").unwrap(), "Updated Name");

	// Cleanup
	db.delete_one(collection, doc! { "_id": oid }).await.ok();
}

/// Test document deletion
///
/// This test verifies that:
/// 1. Documents can be deleted
/// 2. Deleted documents cannot be found
#[rstest]
#[tokio::test]
async fn test_delete_one(#[future] mongodb: (ContainerAsync<GenericImage>, MongoDBBackend)) {
	// Arrange: Insert test document
	let (_container, db) = mongodb.await;
	let user = TestUser::new("delete@example.com", "Delete User");
	let collection = "test_users";
	let user_doc = bson::serialize_to_document(&user).unwrap();
	let id = db.insert_one(collection, user_doc).await.unwrap();
	let oid = ObjectId::parse_str(&id).unwrap();

	// Act: Delete document
	let filter = doc! { "_id": &oid };
	let count = db.delete_one(collection, filter).await.unwrap();

	// Assert: One document deleted
	assert_eq!(count, 1);

	// Verify: Document no longer exists
	let filter = doc! { "_id": &oid };
	let found = db.find_one(collection, filter).await.unwrap();
	assert!(found.is_none());
}

/// Test find_many with options
///
/// This test verifies that:
/// 1. Multiple documents can be retrieved
/// 2. Limit option works correctly
#[rstest]
#[tokio::test]
async fn test_find_many(#[future] mongodb: (ContainerAsync<GenericImage>, MongoDBBackend)) {
	// Arrange: Insert multiple documents
	let (_container, db) = mongodb.await;
	let collection = "test_users";
	let mut inserted_ids = Vec::new();

	for i in 0..5 {
		let user = TestUser::new(&format!("many{}@example.com", i), &format!("User {}", i));
		let user_doc = bson::serialize_to_document(&user).unwrap();
		let id = db.insert_one(collection, user_doc).await.unwrap();
		inserted_ids.push(id);
	}

	// Act: Find all documents with limit
	let filter = doc! {};
	let options = reinhardt_db::nosql::types::FindOptions::new().limit(3);
	let results = db.find_many(collection, filter, options).await.unwrap();

	// Assert: Limited to 3 results
	assert_eq!(results.len(), 3);

	// Cleanup
	for id in inserted_ids {
		let oid = ObjectId::parse_str(&id).unwrap();
		db.delete_one(collection, doc! { "_id": oid }).await.ok();
	}
}
