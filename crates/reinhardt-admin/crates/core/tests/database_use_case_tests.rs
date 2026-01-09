//! Use case tests for AdminDatabase
//!
//! This module contains tests that verify real-world use cases for AdminDatabase operations,
//! covering the "Use case testing" classification from the test plan.
//!
//! Tests in this file use high-level AdminDatabase API with rstest parameterization
//! and reinhardt-test admin_panel fixtures.

#![cfg(all(test, feature = "admin"))]

use reinhardt_admin_core::database::AdminDatabase;
use reinhardt_admin_core::{Filter, FilterOperator, FilterValue};
use reinhardt_db::Model;
use reinhardt_test::fixtures::admin_panel::admin_database;
use rstest::{fixture, rstest};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Use case: User management system
///
/// Simulates a user management system where we:
/// 1. Create users with different roles
/// 2. Filter users by role and status
/// 3. Update user statuses
/// 4. Bulk operations on user groups
///
/// **Test Category**: Use case testing
/// **Test Classification**: Real-world scenario
#[rstest]
#[tokio::test]
async fn use_case_user_management_system(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// 1. Create test users with different roles and statuses
	let users = vec![
		("Alice Admin", "admin", "active"),
		("Bob Moderator", "moderator", "active"),
		("Charlie User", "user", "pending"),
		("David User", "user", "active"),
		("Eve Admin", "admin", "suspended"),
	];

	let mut user_ids = HashMap::new();
	for (name, role, status) in users {
		let user_data = HashMap::from([
			("name".to_string(), json!(name)),
			("role".to_string(), json!(role)),
			("status".to_string(), json!(status)),
			(
				"email".to_string(),
				json!(format!(
					"{}@example.com",
					name.to_lowercase().replace(" ", ".")
				)),
			),
		]);

		let user_id = db
			.create::<reinhardt_admin_core::database::AdminRecord>(table_name, user_data)
			.await
			.expect("User creation should succeed");

		user_ids.insert(name, user_id);
	}

	// 2. Use case: List all active users
	let active_filter = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("active".to_string()),
	};

	let active_users = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![active_filter], 0, 50)
		.await
		.expect("List active users should succeed");

	assert_eq!(active_users.len(), 3, "Should have 3 active users");

	// 3. Use case: List admin users
	let admin_filter = Filter {
		field: "role".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("admin".to_string()),
	};

	let admin_users = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![admin_filter], 0, 50)
		.await
		.expect("List admin users should succeed");

	assert_eq!(admin_users.len(), 2, "Should have 2 admin users");

	// 4. Use case: Combined filter - active admins
	let active_admin_filters = vec![
		Filter {
			field: "role".to_string(),
			operator: FilterOperator::Eq,
			value: FilterValue::String("admin".to_string()),
		},
		Filter {
			field: "status".to_string(),
			operator: FilterOperator::Eq,
			value: FilterValue::String("active".to_string()),
		},
	];

	let active_admins = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			active_admin_filters,
			0,
			50,
		)
		.await
		.expect("List active admins should succeed");

	assert_eq!(active_admins.len(), 1, "Should have 1 active admin");

	// 5. Use case: Bulk update - suspend all users
	let all_users = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![], 0, 50)
		.await
		.expect("List all users should succeed");

	let mut suspended_count = 0;
	for user in &all_users {
		if let Some(id_value) = user.get("id") {
			if let Some(id_str) = id_value.as_str() {
				let update_data = HashMap::from([("status".to_string(), json!("suspended"))]);
				db.update::<reinhardt_admin_core::database::AdminRecord>(
					table_name,
					"id",
					id_str,
					update_data,
				)
				.await
				.expect("Update should succeed");
				suspended_count += 1;
			}
		}
	}

	// 6. Use case: Verify bulk update
	let suspended_filter = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("suspended".to_string()),
	};

	let suspended_users = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![suspended_filter])
		.await
		.expect("Count suspended users should succeed");

	assert_eq!(
		suspended_users, suspended_count,
		"All users should be suspended"
	);

	// 7. Use case: Search by email domain
	let email_filter = Filter {
		field: "email".to_string(),
		operator: FilterOperator::Contains,
		value: FilterValue::String("@example.com".to_string()),
	};

	let domain_users = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![email_filter])
		.await
		.expect("Count domain users should succeed");

	assert_eq!(domain_users, 5, "All users should have example.com emails");

	// 8. Clean up: Bulk delete all test users
	let all_user_ids: Vec<String> = user_ids.values().map(|id| id.to_string()).collect();
	let deleted_count = db
		.bulk_delete::<reinhardt_admin_core::database::AdminRecord>(table_name, "id", all_user_ids)
		.await
		.expect("Bulk delete should succeed");

	assert_eq!(deleted_count, 5, "Should delete all 5 test users");
}

/// Use case: Product inventory management
///
/// Simulates an e-commerce product inventory system where we:
/// 1. Create products with categories and stock levels
/// 2. Filter by category and stock status
/// 3. Update stock levels
/// 4. Handle out-of-stock scenarios
///
/// **Test Category**: Use case testing
/// **Test Classification**: Real-world scenario
#[rstest]
#[tokio::test]
async fn use_case_product_inventory_management(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// 1. Create test products
	let products = vec![
		("Laptop", "electronics", 10, 999.99),
		("Mouse", "electronics", 50, 29.99),
		("Desk", "furniture", 5, 299.99),
		("Chair", "furniture", 0, 199.99), // out of stock
		("Monitor", "electronics", 3, 399.99),
		("Keyboard", "electronics", 25, 79.99),
	];

	let mut product_ids = Vec::new();
	for (name, category, stock, price) in products {
		let product_data = HashMap::from([
			("name".to_string(), json!(name)),
			("category".to_string(), json!(category)),
			("stock".to_string(), json!(stock)),
			("price".to_string(), json!(price)),
			("in_stock".to_string(), json!(stock > 0)),
		]);

		let product_id = db
			.create::<reinhardt_admin_core::database::AdminRecord>(table_name, product_data)
			.await
			.expect("Product creation should succeed");

		product_ids.push(product_id);
	}

	// 2. Use case: List all electronics products
	let electronics_filter = Filter {
		field: "category".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("electronics".to_string()),
	};

	let electronics = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![electronics_filter],
			0,
			50,
		)
		.await
		.expect("List electronics should succeed");

	assert_eq!(electronics.len(), 4, "Should have 4 electronics products");

	// 3. Use case: List in-stock products
	let in_stock_filter = Filter {
		field: "in_stock".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::Boolean(true),
	};

	let in_stock_count = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![in_stock_filter])
		.await
		.expect("Count in-stock should succeed");

	assert_eq!(in_stock_count, 5, "Should have 5 in-stock products");

	// 4. Use case: List out-of-stock products
	let out_of_stock_filter = Filter {
		field: "in_stock".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::Boolean(false),
	};

	let out_of_stock = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![out_of_stock_filter],
			0,
			50,
		)
		.await
		.expect("List out-of-stock should succeed");

	assert_eq!(out_of_stock.len(), 1, "Should have 1 out-of-stock product");

	// 5. Use case: Update stock after sales
	// Simulate selling 2 laptops
	let laptop_filter = Filter {
		field: "name".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("Laptop".to_string()),
	};

	let laptops = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![laptop_filter], 0, 1)
		.await
		.expect("Find laptop should succeed");

	assert!(!laptops.is_empty(), "Should find laptop product");

	if let Some(laptop) = laptops.first() {
		if let Some(id_value) = laptop.get("id") {
			if let Some(id_str) = id_value.as_str() {
				if let Some(current_stock) = laptop.get("stock").and_then(|v| v.as_i64()) {
					let new_stock = current_stock - 2;
					let update_data = HashMap::from([
						("stock".to_string(), json!(new_stock)),
						("in_stock".to_string(), json!(new_stock > 0)),
					]);

					db.update::<reinhardt_admin_core::database::AdminRecord>(
						table_name,
						"id",
						id_str,
						update_data,
					)
					.await
					.expect("Update stock should succeed");

					// Verify update
					let updated = db
						.get::<reinhardt_admin_core::database::AdminRecord>(
							table_name, "id", id_str,
						)
						.await
						.expect("Get updated laptop should succeed")
						.expect("Laptop should exist");

					assert_eq!(
						updated.get("stock"),
						Some(&json!(new_stock)),
						"Stock should be updated"
					);
				}
			}
		}
	}

	// 6. Use case: Filter by price range
	let price_filters = vec![
		Filter {
			field: "price".to_string(),
			operator: FilterOperator::Gte,
			value: FilterValue::Number(100.into()),
		},
		Filter {
			field: "price".to_string(),
			operator: FilterOperator::Lte,
			value: FilterValue::Number(500.into()),
		},
	];

	let mid_range_products = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, price_filters)
		.await
		.expect("Count price range should succeed");

	assert!(
		mid_range_products >= 3,
		"Should have products in $100-$500 range"
	);

	// 7. Use case: Restock operation
	let low_stock_filter = Filter {
		field: "stock".to_string(),
		operator: FilterOperator::Lt,
		value: FilterValue::Number(5.into()),
	};

	let low_stock_products = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![low_stock_filter],
			0,
			50,
		)
		.await
		.expect("List low stock should succeed");

	// Restock each low stock product
	for product in &low_stock_products {
		if let Some(id_value) = product.get("id") {
			if let Some(id_str) = id_value.as_str() {
				if let Some(current_stock) = product.get("stock").and_then(|v| v.as_i64()) {
					let new_stock = current_stock + 10; // Add 10 units
					let update_data = HashMap::from([
						("stock".to_string(), json!(new_stock)),
						("in_stock".to_string(), json!(true)),
					]);

					db.update::<reinhardt_admin_core::database::AdminRecord>(
						table_name,
						"id",
						id_str,
						update_data,
					)
					.await
					.expect("Restock should succeed");
				}
			}
		}
	}

	// 8. Verify no low stock products after restock
	let low_stock_after = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![low_stock_filter])
		.await
		.expect("Count low stock after should succeed");

	assert_eq!(
		low_stock_after, 0,
		"Should be no low stock products after restock"
	);

	// 9. Clean up
	let deleted_count = db
		.bulk_delete::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			"id",
			product_ids.iter().map(|id| id.to_string()).collect(),
		)
		.await
		.expect("Bulk delete should succeed");

	assert_eq!(deleted_count, 6, "Should delete all 6 products");
}

/// Use case: Content management system
///
/// Simulates a CMS where we:
/// 1. Create content with different types and statuses
/// 2. Filter by type, status, and date ranges
/// 3. Update content status (draft → published → archived)
/// 4. Search content
///
/// **Test Category**: Use case testing
/// **Test Classification**: Real-world scenario
#[rstest]
#[tokio::test]
async fn use_case_content_management_system(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// 1. Create test content items
	let content_items = vec![
		(
			"Getting Started Guide",
			"article",
			"published",
			"2024-01-15",
		),
		("API Reference", "documentation", "published", "2024-02-01"),
		("News Announcement", "news", "draft", "2024-02-10"),
		("Tutorial Part 1", "tutorial", "published", "2024-01-20"),
		("Tutorial Part 2", "tutorial", "draft", "2024-02-05"),
		("Archived Policy", "policy", "archived", "2023-12-01"),
	];

	let mut content_ids = Vec::new();
	for (title, content_type, status, created_date) in content_items {
		let content_data = HashMap::from([
			("title".to_string(), json!(title)),
			("type".to_string(), json!(content_type)),
			("status".to_string(), json!(status)),
			("created_date".to_string(), json!(created_date)),
			("views".to_string(), json!(0)),
		]);

		let content_id = db
			.create::<reinhardt_admin_core::database::AdminRecord>(table_name, content_data)
			.await
			.expect("Content creation should succeed");

		content_ids.push(content_id);
	}

	// 2. Use case: List all published content
	let published_filter = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("published".to_string()),
	};

	let published_count = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![published_filter])
		.await
		.expect("Count published should succeed");

	assert_eq!(published_count, 3, "Should have 3 published items");

	// 3. Use case: List tutorials
	let tutorial_filter = Filter {
		field: "type".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("tutorial".to_string()),
	};

	let tutorials = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![tutorial_filter],
			0,
			50,
		)
		.await
		.expect("List tutorials should succeed");

	assert_eq!(tutorials.len(), 2, "Should have 2 tutorials");

	// 4. Use case: Publish a draft
	let draft_filter = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("draft".to_string()),
	};

	let drafts = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![draft_filter], 0, 50)
		.await
		.expect("List drafts should succeed");

	assert_eq!(drafts.len(), 2, "Should have 2 drafts");

	// Publish the first draft
	if let Some(draft) = drafts.first() {
		if let Some(id_value) = draft.get("id") {
			if let Some(id_str) = id_value.as_str() {
				let update_data = HashMap::from([
					("status".to_string(), json!("published")),
					("published_date".to_string(), json!("2024-02-15")),
				]);

				db.update::<reinhardt_admin_core::database::AdminRecord>(
					table_name,
					"id",
					id_str,
					update_data,
				)
				.await
				.expect("Publish should succeed");

				// Verify publish
				let published = db
					.get::<reinhardt_admin_core::database::AdminRecord>(table_name, "id", id_str)
					.await
					.expect("Get published should succeed")
					.expect("Content should exist");

				assert_eq!(
					published.get("status"),
					Some(&json!("published")),
					"Status should be published"
				);
			}
		}
	}

	// 5. Use case: Archive old content
	let old_content_filter = Filter {
		field: "created_date".to_string(),
		operator: FilterOperator::Lt,
		value: FilterValue::String("2024-01-01".to_string()),
	};

	let old_content = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![old_content_filter],
			0,
			50,
		)
		.await
		.expect("List old content should succeed");

	for content in &old_content {
		if let Some(id_value) = content.get("id") {
			if let Some(id_str) = id_value.as_str() {
				// Skip if already archived
				if content.get("status") != Some(&json!("archived")) {
					let update_data = HashMap::from([("status".to_string(), json!("archived"))]);
					db.update::<reinhardt_admin_core::database::AdminRecord>(
						table_name,
						"id",
						id_str,
						update_data,
					)
					.await
					.expect("Archive should succeed");
				}
			}
		}
	}

	// 6. Use case: Search content by keyword
	let search_filter = Filter {
		field: "title".to_string(),
		operator: FilterOperator::Contains,
		value: FilterValue::String("Tutorial".to_string()),
	};

	let search_results = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![search_filter])
		.await
		.expect("Search should succeed");

	assert_eq!(
		search_results, 2,
		"Should find 2 items with 'Tutorial' in title"
	);

	// 7. Use case: Track views (simulate view counting)
	let published_items = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![Filter {
				field: "status".to_string(),
				operator: FilterOperator::Eq,
				value: FilterValue::String("published".to_string()),
			}],
			0,
			50,
		)
		.await
		.expect("List published for views should succeed");

	for item in &published_items {
		if let Some(id_value) = item.get("id") {
			if let Some(id_str) = id_value.as_str() {
				if let Some(current_views) = item.get("views").and_then(|v| v.as_i64()) {
					let new_views = current_views + 1;
					let update_data = HashMap::from([("views".to_string(), json!(new_views))]);
					db.update::<reinhardt_admin_core::database::AdminRecord>(
						table_name,
						"id",
						id_str,
						update_data,
					)
					.await
					.expect("Update views should succeed");
				}
			}
		}
	}

	// 8. Use case: Get most viewed content
	let viewed_items = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![], 0, 50)
		.await
		.expect("List all for views should succeed");

	let total_views: i64 = viewed_items
		.iter()
		.filter_map(|item| item.get("views").and_then(|v| v.as_i64()))
		.sum();

	assert!(total_views >= 3, "Should have at least 3 total views");

	// 9. Clean up
	let deleted_count = db
		.bulk_delete::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			"id",
			content_ids.iter().map(|id| id.to_string()).collect(),
		)
		.await
		.expect("Bulk delete should succeed");

	assert_eq!(deleted_count, 6, "Should delete all 6 content items");
}

/// Use case: Audit log system
///
/// Simulates an audit log system where we:
/// 1. Create audit entries with different action types and severities
/// 2. Filter by date ranges, action types, and severities
/// 3. Search audit logs
/// 4. Clean up old audit entries
///
/// **Test Category**: Use case testing
/// **Test Classification**: Real-world scenario
#[rstest]
#[tokio::test]
async fn use_case_audit_log_system(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// 1. Create test audit entries
	let audit_entries = vec![
		(
			"user_login",
			"info",
			"2024-02-15T10:30:00Z",
			"User 'alice' logged in",
		),
		(
			"user_logout",
			"info",
			"2024-02-15T11:00:00Z",
			"User 'alice' logged out",
		),
		(
			"data_update",
			"warning",
			"2024-02-14T15:45:00Z",
			"User 'admin' updated settings",
		),
		(
			"security_event",
			"critical",
			"2024-02-13T09:15:00Z",
			"Failed login attempt from unknown IP",
		),
		(
			"system_backup",
			"info",
			"2024-02-12T02:00:00Z",
			"Nightly backup completed",
		),
		(
			"data_export",
			"info",
			"2024-02-11T14:20:00Z",
			"User 'bob' exported report",
		),
	];

	let mut audit_ids = Vec::new();
	for (action, severity, timestamp, description) in audit_entries {
		let audit_data = HashMap::from([
			("action".to_string(), json!(action)),
			("severity".to_string(), json!(severity)),
			("timestamp".to_string(), json!(timestamp)),
			("description".to_string(), json!(description)),
			(
				"user".to_string(),
				json!(match action {
					"user_login" | "user_logout" => "alice",
					"data_update" | "security_event" => "admin",
					"system_backup" => "system",
					"data_export" => "bob",
					_ => "unknown",
				}),
			),
		]);

		let audit_id = db
			.create::<reinhardt_admin_core::database::AdminRecord>(table_name, audit_data)
			.await
			.expect("Audit entry creation should succeed");

		audit_ids.push(audit_id);
	}

	// 2. Use case: Get recent audit entries (last 3 days)
	// Simulated by filtering entries newer than 2024-02-13
	let recent_filter = Filter {
		field: "timestamp".to_string(),
		operator: FilterOperator::Gte,
		value: FilterValue::String("2024-02-13T00:00:00Z".to_string()),
	};

	let recent_count = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![recent_filter])
		.await
		.expect("Count recent should succeed");

	assert_eq!(recent_count, 4, "Should have 4 recent audit entries");

	// 3. Use case: Get critical security events
	let critical_filter = Filter {
		field: "severity".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("critical".to_string()),
	};

	let critical_events = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![critical_filter],
			0,
			50,
		)
		.await
		.expect("List critical should succeed");

	assert_eq!(critical_events.len(), 1, "Should have 1 critical event");

	// 4. Use case: Get audit entries for specific user
	let user_filter = Filter {
		field: "user".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("alice".to_string()),
	};

	let user_entries = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![user_filter])
		.await
		.expect("Count user entries should succeed");

	assert_eq!(user_entries, 2, "Should have 2 entries for user 'alice'");

	// 5. Use case: Search audit descriptions
	let search_filter = Filter {
		field: "description".to_string(),
		operator: FilterOperator::Contains,
		value: FilterValue::String("login".to_string()),
	};

	let login_entries = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![search_filter], 0, 50)
		.await
		.expect("Search login should succeed");

	assert_eq!(login_entries.len(), 2, "Should find 2 entries with 'login'");

	// 6. Use case: Clean up old audit entries (older than 7 days)
	let old_filter = Filter {
		field: "timestamp".to_string(),
		operator: FilterOperator::Lt,
		value: FilterValue::String("2024-02-08T00:00:00Z".to_string()),
	};

	let old_entries = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![old_filter], 0, 50)
		.await
		.expect("List old entries should succeed");

	// Delete old entries (simulate cleanup)
	let mut deleted_count = 0;
	for entry in &old_entries {
		if let Some(id_value) = entry.get("id") {
			if let Some(id_str) = id_value.as_str() {
				db.delete::<reinhardt_admin_core::database::AdminRecord>(table_name, "id", id_str)
					.await
					.expect("Delete old entry should succeed");
				deleted_count += 1;
			}
		}
	}

	// In this test dataset, no entries are older than 7 days
	assert_eq!(deleted_count, 0, "Should delete 0 old entries");

	// 7. Use case: Get audit statistics
	let all_entries = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![], 0, 50)
		.await
		.expect("List all for stats should succeed");

	// Count by severity
	let mut severity_counts = std::collections::HashMap::new();
	for entry in &all_entries {
		if let Some(severity) = entry.get("severity").and_then(|v| v.as_str()) {
			*severity_counts.entry(severity).or_insert(0) += 1;
		}
	}

	assert_eq!(
		severity_counts.get("info").copied().unwrap_or(0),
		4,
		"Should have 4 info entries"
	);
	assert_eq!(
		severity_counts.get("warning").copied().unwrap_or(0),
		1,
		"Should have 1 warning entry"
	);
	assert_eq!(
		severity_counts.get("critical").copied().unwrap_or(0),
		1,
		"Should have 1 critical entry"
	);

	// 8. Clean up
	let deleted_count = db
		.bulk_delete::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			"id",
			audit_ids.iter().map(|id| id.to_string()).collect(),
		)
		.await
		.expect("Bulk delete should succeed");

	assert_eq!(deleted_count, 6, "Should delete all 6 audit entries");
}
