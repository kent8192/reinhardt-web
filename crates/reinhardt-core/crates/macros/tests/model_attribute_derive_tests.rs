//! Tests for automatic derive trait addition in #[model(...)] attribute macro

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[test]
fn test_auto_derive_all_traits() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test", table_name = "users")]
	pub struct User {
		#[field(primary_key = true)]
		pub id: i64,
		#[field(max_length = 255)]
		pub name: String,
	}

	let user = User {
		id: 1,
		name: "Alice".to_string(),
	};

	// Debug trait
	let debug_output = format!("{:?}", user);
	assert!(debug_output.contains("User"));

	// Clone trait
	let cloned = user.clone();
	assert_eq!(cloned.id, 1);
	assert_eq!(cloned.name, "Alice");

	// PartialEq trait
	assert_eq!(user, cloned);

	// Serialize/Deserialize traits
	let json = serde_json::to_string(&cloned).expect("Serialization failed");
	assert!(json.contains("Alice"));

	let deserialized: User = serde_json::from_str(&json).expect("Deserialization failed");
	assert_eq!(deserialized.id, 1);
	assert_eq!(deserialized.name, "Alice");
}

// Note: Attribute macro limitation
//
// When #[derive(...)] is placed ABOVE #[model(...)], the derive attribute
// is NOT visible to the model attribute macro (input.attrs is empty).
// This is a fundamental limitation of Rust's attribute macro system.
//
// Correct usage pattern:
//   #[model(app_label = "test", table_name = "users")]  // FIRST
//   #[derive(Serialize, Deserialize)]                   // SECOND (optional)
//   pub struct User { ... }
//
// The #[model] macro will automatically add Debug, Clone, PartialEq.
// Users only need to add Serialize, Deserialize (and optionally Hash, Eq).

#[test]
fn test_model_first_derive_second() {
	// Correct pattern: #[model] before #[derive]
	// #[model] adds Model, Debug, Clone, PartialEq automatically
	// User adds Serialize, Deserialize manually
	#[model(app_label = "test", table_name = "ordered_users")]
	#[derive(Serialize, Deserialize)]
	pub struct OrderedUser {
		#[field(primary_key = true)]
		pub id: i64,
	}

	let user = OrderedUser { id: 1 };

	// All traits should be available
	// Debug: auto-added by #[model]
	let debug_output = format!("{:?}", user);
	assert!(debug_output.contains("OrderedUser"));

	// Clone: auto-added by #[model]
	let cloned = user.clone();
	assert_eq!(cloned.id, 1);

	// PartialEq: auto-added by #[model]
	assert_eq!(user, cloned);

	// Serialize/Deserialize: from user's derive
	let json = serde_json::to_string(&cloned).expect("Serialization failed");
	let _: OrderedUser = serde_json::from_str(&json).expect("Deserialization failed");
}

#[test]
fn test_model_with_hash_eq() {
	// User can add Hash and Eq when needed
	#[model(app_label = "test", table_name = "hashable_users")]
	#[derive(Serialize, Deserialize, Eq, Hash)]
	pub struct HashableUser {
		#[field(primary_key = true)]
		pub id: i64,
	}

	let user = HashableUser { id: 1 };

	// Verify Hash works (via HashSet)
	use std::collections::HashSet;
	let mut set = HashSet::new();
	set.insert(user.clone());
	assert_eq!(set.len(), 1);

	// Insert same value again
	set.insert(user);
	assert_eq!(set.len(), 1); // Still 1 (duplicate)
}

#[test]
fn test_manual_hash_trait() {
	// Note: Hash and Eq are NOT auto-derived by #[model] because not all types
	// implement these traits (e.g., f64, f32). Users can manually add them when needed.
	#[derive(Serialize, Deserialize, Hash, Eq)]
	#[model(app_label = "test", table_name = "products")]
	pub struct Product {
		#[field(primary_key = true)]
		pub id: i64,
		#[field(max_length = 255)]
		pub name: String,
	}

	let p1 = Product {
		id: 1,
		name: "Apple".to_string(),
	};
	let p2 = Product {
		id: 2,
		name: "Banana".to_string(),
	};
	let p3 = Product {
		id: 1,
		name: "Apple".to_string(),
	};

	use std::collections::HashMap;
	let mut map = HashMap::new();
	map.insert(p1, 10);
	map.insert(p2, 20);

	// p3 is equal to p1, so it should update the existing entry
	assert_eq!(map.len(), 2);
	map.insert(p3, 15);
	assert_eq!(map.len(), 2); // Should still be 2 (updated p1's value)
}

#[test]
fn test_partialeq() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test", table_name = "items")]
	pub struct Item {
		#[field(primary_key = true)]
		pub id: i64,
		#[field(max_length = 255)]
		pub value: String,
	}

	let item1 = Item {
		id: 1,
		value: "test".to_string(),
	};
	let item2 = Item {
		id: 1,
		value: "test".to_string(),
	};
	let item3 = Item {
		id: 2,
		value: "test".to_string(),
	};

	// PartialEq (auto-derived by #[model])
	assert_eq!(item1, item2);
	assert_ne!(item1, item3);
}
