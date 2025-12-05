//! Tests for automatic derive trait addition in #[model(...)] attribute macro

use reinhardt_macros::{Model, model};
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

// TODO: Fix duplicate detection logic for partial derives
// Currently the macro adds all traits even when some are already present
//
// #[test]
// fn test_partial_derive_present() {
// 	// User already has Debug and Clone
// 	#[derive(Debug, Clone)]
// 	#[model(app_label = "test", table_name = "users")]
// 	pub struct User {
// 		#[field(primary_key = true)]
// 		pub id: i64,
// 	}
//
// 	let user = User { id: 1 };
//
// 	// All traits should be available (auto-added by #[model])
// 	let _ = format!("{:?}", user);
// 	let cloned = user.clone();
// 	assert_eq!(user, cloned);
//
// 	// Serialize/Deserialize should also work
// 	let json = serde_json::to_string(&cloned).expect("Serialization failed");
// 	let _: User = serde_json::from_str(&json).expect("Deserialization failed");
// }

// TODO: Fix duplicate detection logic
// Currently the macro adds traits even when they're already present
// This causes compilation errors when testing explicit derives
//
// #[test]
// fn test_all_traits_already_present() {
// 	// User explicitly derives all traits
// 	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
// 	#[model(app_label = "test", table_name = "users")]
// 	pub struct User {
// 		#[field(primary_key = true)]
// 		pub id: i64,
// 	}
//
// 	let user = User { id: 1 };
//
// 	// Should not cause duplicate trait errors
// 	let cloned = user.clone();
// 	assert_eq!(user, cloned);
// }

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
