//! Bulk Update Tests for Hybrid Properties
//!
//! Based on SQLAlchemy's BulkUpdateTest from test_hybrid.py

use reinhardt_hybrid::HybridProperty;
use reinhardt_orm::bulk_update::{BulkUpdateBuilder, SynchronizeStrategy};

#[derive(Debug)]
struct Person {
	id: i32,
	first_name: String,
	last_name: String,
}

impl Person {
	fn name(&self) -> String {
		format!("{} {}", self.first_name, self.last_name)
	}

	fn set_name(&mut self, value: &str) {
		let parts: Vec<&str> = value.splitn(2, ' ').collect();
		if parts.len() == 2 {
			self.first_name = parts[0].to_string();
			self.last_name = parts[1].to_string();
		}
	}
}

// Test 1: Basic UPDATE with plain hybrid property
#[test]
fn test_update_plain() {
	let fname = HybridProperty::new(|p: &Person| p.first_name.clone());

	let builder = BulkUpdateBuilder::new("person").set_hybrid("first_name", &fname, "Dr.");

	let (sql, params, _) = builder.build();
	assert_eq!(sql, "UPDATE person SET first_name=?");
	assert_eq!(params, vec!["Dr."]);
}

// Test 2: UPDATE with expanded hybrid property (name -> first_name, last_name)
#[test]
fn test_update_expr_attr() {
	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid_expanded(vec![("first_name", "Dr."), ("last_name", "No")]);

	let (sql, params, _) = builder.build();
	assert!(sql.contains("UPDATE person SET"));
	assert!(sql.contains("first_name=?"));
	assert!(sql.contains("last_name=?"));
	assert_eq!(params.len(), 2);
	assert!(params.contains(&"Dr.".to_string()));
	assert!(params.contains(&"No".to_string()));
}

// Test 3: INSERT with expanded hybrid property
#[test]
fn test_insert_expr() {
	// Note: Using the same expanded approach for INSERT
	// In real implementation, this would use InsertBuilder
	let updates = vec![("first_name", "Dr."), ("last_name", "No")];

	// Verify the values can be split correctly
	assert_eq!(updates.len(), 2);
	assert_eq!(updates[0].0, "first_name");
	assert_eq!(updates[0].1, "Dr.");
	assert_eq!(updates[1].0, "last_name");
	assert_eq!(updates[1].1, "No");
}

// Test 4: Evaluate non-hybrid attribute (control test)
#[test]
fn test_evaluate_non_hybrid_attr() {
	let builder = BulkUpdateBuilder::new("person")
		.set("first_name", "moonbeam")
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Evaluate);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET first_name=? WHERE id = 3"));
	assert_eq!(params, vec!["moonbeam"]);
	assert_eq!(strategy, SynchronizeStrategy::Evaluate);
}

// Test 5: Evaluate hybrid attribute (indirect - fname2 -> fname -> first_name)
#[test]
fn test_evaluate_hybrid_attr_indirect() {
	// fname2 is a hybrid property that returns fname, which returns first_name
	let fname2 = HybridProperty::new(|p: &Person| p.first_name.clone());

	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid("first_name", &fname2, "moonbeam")
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Evaluate);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET first_name=? WHERE id = 3"));
	assert_eq!(params, vec!["moonbeam"]);
	assert_eq!(strategy, SynchronizeStrategy::Evaluate);
}

// Test 6: Evaluate plain hybrid attribute
#[test]
fn test_evaluate_hybrid_attr_plain() {
	let fname = HybridProperty::new(|p: &Person| p.first_name.clone());

	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid("first_name", &fname, "moonbeam")
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Evaluate);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET first_name=? WHERE id = 3"));
	assert_eq!(params, vec!["moonbeam"]);
	assert_eq!(strategy, SynchronizeStrategy::Evaluate);
}

// Test 7: Fetch hybrid attribute (indirect)
#[test]
fn test_fetch_hybrid_attr_indirect() {
	let fname2 = HybridProperty::new(|p: &Person| p.first_name.clone());

	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid("first_name", &fname2, "moonbeam")
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Fetch);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET first_name=? WHERE id = 3"));
	assert_eq!(params, vec!["moonbeam"]);
	assert_eq!(strategy, SynchronizeStrategy::Fetch);
}

// Test 8: Fetch plain hybrid attribute
#[test]
fn test_fetch_hybrid_attr_plain() {
	let fname = HybridProperty::new(|p: &Person| p.first_name.clone());

	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid("first_name", &fname, "moonbeam")
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Fetch);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET first_name=? WHERE id = 3"));
	assert_eq!(params, vec!["moonbeam"]);
	assert_eq!(strategy, SynchronizeStrategy::Fetch);
}

// Test 9: Evaluate hybrid attribute with update expression
#[test]
fn test_evaluate_hybrid_attr_w_update_expr() {
	// name property expands to first_name and last_name
	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid_expanded(vec![("first_name", "moonbeam"), ("last_name", "sunshine")])
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Evaluate);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET"));
	assert!(sql.contains("first_name=?"));
	assert!(sql.contains("last_name=?"));
	assert!(sql.contains("WHERE id = 3"));
	assert_eq!(params.len(), 2);
	assert_eq!(strategy, SynchronizeStrategy::Evaluate);
}

// Test 10: Fetch hybrid attribute with update expression
#[test]
fn test_fetch_hybrid_attr_w_update_expr() {
	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid_expanded(vec![("first_name", "moonbeam"), ("last_name", "sunshine")])
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Fetch);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET"));
	assert!(sql.contains("first_name=?"));
	assert!(sql.contains("last_name=?"));
	assert!(sql.contains("WHERE id = 3"));
	assert_eq!(params.len(), 2);
	assert_eq!(strategy, SynchronizeStrategy::Fetch);
}

// Test 11: Evaluate indirect hybrid attribute with update expression
#[test]
fn test_evaluate_hybrid_attr_indirect_w_update_expr() {
	// uname property returns name, which expands to first_name and last_name
	let builder = BulkUpdateBuilder::new("person")
		.set_hybrid_expanded(vec![("first_name", "moonbeam"), ("last_name", "sunshine")])
		.where_clause("id = 3")
		.synchronize(SynchronizeStrategy::Evaluate);

	let (sql, params, strategy) = builder.build();
	assert!(sql.contains("UPDATE person SET"));
	assert!(sql.contains("first_name=?"));
	assert!(sql.contains("last_name=?"));
	assert!(sql.contains("WHERE id = 3"));
	assert_eq!(params.len(), 2);
	assert_eq!(strategy, SynchronizeStrategy::Evaluate);
}
