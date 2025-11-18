//! MongoDB integration tests using TestContainers
//!
//! These tests require Docker to be running.

#[cfg(feature = "mongodb-backend")]
mod mongodb_tests {
	use bson::doc;
	use reinhardt_db::backends::mongodb::{
		MongoDBBackend, MongoDBBackendBuilder, MongoDBQueryBuilder, MongoDBSchemaEditor,
	};
	use rstest::*;
	use serial_test::serial;
	use testcontainers::{ContainerAsync, GenericImage, core::WaitFor, runners::AsyncRunner};

	#[fixture]
	async fn mongodb_container() -> (ContainerAsync<GenericImage>, String, u16) {
		let mongo = GenericImage::new("mongo", "7.0")
			.with_wait_for(WaitFor::message_on_stdout("Waiting for connections"))
			.start()
			.await
			.expect("Failed to start MongoDB container");

		let port = mongo
			.get_host_port_ipv4(27017)
			.await
			.expect("Failed to get MongoDB port");

		let connection_string = format!("mongodb://127.0.0.1:{}", port);

		(mongo, connection_string, port)
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_connection(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect");

		let backend = backend.with_database("test_db");
		let db = backend.database();

		// Verify connection by listing collections
		let collections = db
			.list_collection_names()
			.await
			.expect("Failed to list collections");

		assert!(collections.is_empty() || !collections.is_empty()); // Just verify it works
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_builder_connection(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackendBuilder::new()
			.url(&connection_string)
			.database("test_db")
			.max_pool_size(50)
			.min_pool_size(5)
			.build()
			.await
			.expect("Failed to build backend");

		let db = backend.database();
		let collections = db
			.list_collection_names()
			.await
			.expect("Failed to list collections");

		assert!(collections.is_empty() || !collections.is_empty());
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_insert_and_find(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_insert");

		// Clean up collection first
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;

		// Insert a document
		let doc = doc! {
			"name": "Alice",
			"age": 30,
			"email": "alice@example.com"
		};

		let id = backend
			.insert_one("users", doc.clone())
			.await
			.expect("Failed to insert");

		assert!(!id.as_object_id().unwrap().to_hex().is_empty());

		// Find the document
		let filter = doc! { "name": "Alice" };
		let found = backend
			.find_one("users", filter)
			.await
			.expect("Failed to find")
			.expect("Document not found");

		assert_eq!(found.get_str("name").unwrap(), "Alice");
		assert_eq!(found.get_i32("age").unwrap(), 30);

		// Clean up
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_insert_many(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_insert_many");

		// Clean up collection first
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;

		// Insert multiple documents
		let docs = vec![
			doc! { "name": "Alice", "age": 30 },
			doc! { "name": "Bob", "age": 25 },
			doc! { "name": "Charlie", "age": 35 },
		];

		let ids = backend
			.insert_many("users", docs)
			.await
			.expect("Failed to insert many");

		assert_eq!(ids.len(), 3);

		// Verify all documents were inserted
		let all_docs = backend
			.find("users", doc! {})
			.await
			.expect("Failed to find all");

		assert_eq!(all_docs.len(), 3);

		// Clean up
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_update_operations(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_update");

		// Clean up collection first
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;

		// Insert test documents
		let docs = vec![
			doc! { "name": "Alice", "age": 30, "status": "active" },
			doc! { "name": "Bob", "age": 25, "status": "active" },
		];
		backend
			.insert_many("users", docs)
			.await
			.expect("Failed to insert");

		// Update one document
		let filter = doc! { "name": "Alice" };
		let update = doc! { "$set": { "age": 31 } };
		let count = backend
			.update_one("users", filter, update)
			.await
			.expect("Failed to update one");

		assert_eq!(count, 1);

		// Verify update
		let updated = backend
			.find_one("users", doc! { "name": "Alice" })
			.await
			.expect("Failed to find")
			.expect("Document not found");

		assert_eq!(updated.get_i32("age").unwrap(), 31);

		// Update many documents
		let filter = doc! { "status": "active" };
		let update = doc! { "$set": { "status": "inactive" } };
		let count = backend
			.update_many("users", filter, update)
			.await
			.expect("Failed to update many");

		assert_eq!(count, 2);

		// Clean up
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_delete_operations(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_delete");

		// Clean up collection first
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;

		// Insert test documents
		let docs = vec![
			doc! { "name": "Alice", "age": 30 },
			doc! { "name": "Bob", "age": 25 },
			doc! { "name": "Charlie", "age": 35 },
		];
		backend
			.insert_many("users", docs)
			.await
			.expect("Failed to insert");

		// Delete one document
		let filter = doc! { "name": "Alice" };
		let count = backend
			.delete_one("users", filter)
			.await
			.expect("Failed to delete one");

		assert_eq!(count, 1);

		// Verify deletion
		let remaining = backend
			.find("users", doc! {})
			.await
			.expect("Failed to find all");

		assert_eq!(remaining.len(), 2);

		// Delete many documents
		let filter = doc! { "age": { "$gte": 25 } };
		let count = backend
			.delete_many("users", filter)
			.await
			.expect("Failed to delete many");

		assert_eq!(count, 2);

		// Verify all deleted
		let remaining = backend
			.find("users", doc! {})
			.await
			.expect("Failed to find all");

		assert_eq!(remaining.len(), 0);

		// Clean up
		let _ = backend
			.database()
			.collection::<bson::Document>("users")
			.drop()
			.await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_aggregation(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_aggregation");

		// Clean up collection first
		let _ = backend
			.database()
			.collection::<bson::Document>("orders")
			.drop()
			.await;

		// Insert test documents
		let docs = vec![
			doc! { "user": "Alice", "amount": 100, "status": "completed" },
			doc! { "user": "Alice", "amount": 200, "status": "completed" },
			doc! { "user": "Bob", "amount": 150, "status": "completed" },
			doc! { "user": "Bob", "amount": 50, "status": "pending" },
		];
		backend
			.insert_many("orders", docs)
			.await
			.expect("Failed to insert");

		// Aggregate: sum completed orders by user
		let pipeline = vec![
			doc! { "$match": { "status": "completed" } },
			doc! { "$group": {
				"_id": "$user",
				"total": { "$sum": "$amount" }
			}},
			doc! { "$sort": { "total": -1 } },
		];

		let results = backend
			.aggregate("orders", pipeline)
			.await
			.expect("Failed to aggregate");

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].get_str("_id").unwrap(), "Alice");
		assert_eq!(results[0].get_i32("total").unwrap(), 300);
		assert_eq!(results[1].get_str("_id").unwrap(), "Bob");
		assert_eq!(results[1].get_i32("total").unwrap(), 150);

		// Clean up
		let _ = backend
			.database()
			.collection::<bson::Document>("orders")
			.drop()
			.await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_query_builder(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_query_builder");

		// Clean up collection first
		let _ = backend
			.database()
			.collection::<bson::Document>("products")
			.drop()
			.await;

		// Insert test documents
		let docs = vec![
			doc! { "name": "Laptop", "price": 1000, "category": "electronics" },
			doc! { "name": "Mouse", "price": 25, "category": "electronics" },
			doc! { "name": "Desk", "price": 300, "category": "furniture" },
			doc! { "name": "Chair", "price": 150, "category": "furniture" },
		];
		backend
			.insert_many("products", docs)
			.await
			.expect("Failed to insert");

		// Use query builder
		let query = MongoDBQueryBuilder::new("products")
			.filter(doc! { "category": "electronics" })
			.sort(doc! { "price": -1 })
			.limit(10);

		let filter = query.build_filter();
		assert_eq!(filter.get_str("category").unwrap(), "electronics");

		// Execute query
		let db = backend.database();
		let coll = db.collection::<bson::Document>("products");

		let cursor = coll
			.find(filter)
			.sort(query.build_sort().unwrap())
			.limit(query.get_limit().unwrap())
			.await
			.expect("Failed to execute query");

		use futures::stream::TryStreamExt;
		let results: Vec<bson::Document> = cursor
			.try_collect()
			.await
			.expect("Failed to collect results");

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].get_str("name").unwrap(), "Laptop"); // Higher price first

		// Clean up
		let _ = backend
			.database()
			.collection::<bson::Document>("products")
			.drop()
			.await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_schema_editor(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let editor = MongoDBSchemaEditor::new(&connection_string, "test_schema")
			.await
			.expect("Failed to create editor");

		// Clean up collection first
		let _ = editor.drop_collection("test_collection").await;

		// Create collection with validation
		let validator = doc! {
			"$jsonSchema": {
				"required": ["name", "email"],
				"properties": {
					"name": { "bsonType": "string" },
					"email": { "bsonType": "string" }
				}
			}
		};

		editor
			.create_collection("test_collection", Some(validator))
			.await
			.expect("Failed to create collection");

		// Create index
		editor
			.create_index("test_collection", "idx_email", &["email"], true)
			.await
			.expect("Failed to create index");

		// List indexes
		let indexes = editor
			.list_indexes("test_collection")
			.await
			.expect("Failed to list indexes");

		assert!(!indexes.is_empty());
		let idx_names: Vec<String> = indexes
			.iter()
			.filter_map(|doc| doc.get_str("name").ok().map(|s| s.to_string()))
			.collect();
		assert!(idx_names.contains(&"idx_email".to_string()));

		// Drop index
		editor
			.drop_index("test_collection", "idx_email")
			.await
			.expect("Failed to drop index");

		// Drop collection
		editor
			.drop_collection("test_collection")
			.await
			.expect("Failed to drop collection");
	}

	#[rstest]
	#[tokio::test]
	#[serial(mongodb)]
	async fn test_validation_enforcement(
		#[future] mongodb_container: (ContainerAsync<GenericImage>, String, u16),
	) {
		let (_container, connection_string, _port) = mongodb_container.await;

		let editor = MongoDBSchemaEditor::new(&connection_string, "test_validation")
			.await
			.expect("Failed to create editor");

		// Clean up collection first
		let _ = editor.drop_collection("validated_users").await;

		// Create collection with strict validation
		let validator = doc! {
			"$jsonSchema": {
				"required": ["name", "email", "age"],
				"properties": {
					"name": { "bsonType": "string", "minLength": 1 },
					"email": { "bsonType": "string", "pattern": "^.+@.+$" },
					"age": { "bsonType": "int", "minimum": 0, "maximum": 150 }
				}
			}
		};

		editor
			.create_collection("validated_users", Some(validator))
			.await
			.expect("Failed to create collection");

		let backend = MongoDBBackend::connect(&connection_string)
			.await
			.expect("Failed to connect")
			.with_database("test_validation");

		// Valid document should succeed
		let valid_doc = doc! {
			"name": "Alice",
			"email": "alice@example.com",
			"age": 30
		};

		let result = backend.insert_one("validated_users", valid_doc).await;
		assert!(result.is_ok());

		// Invalid document (missing required field) should fail
		let invalid_doc = doc! {
			"name": "Bob",
			"age": 25
			// missing email
		};

		let result = backend.insert_one("validated_users", invalid_doc).await;
		assert!(result.is_err());

		// Clean up
		let _ = editor.drop_collection("validated_users").await;
	}
}
