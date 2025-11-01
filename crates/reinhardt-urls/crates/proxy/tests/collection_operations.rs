//! Collection operation tests for association proxies
//!
//! These tests verify that collection proxy operations work correctly,
//! based on SQLAlchemy's association_proxy tests.

use reinhardt_proxy::{
	CollectionProxy, ProxyError, ProxyResult, ScalarValue, reflection::Reflectable,
};
use std::any::Any;

/// Test model representing a parent with children
#[derive(Clone)]
struct Parent {
	id: i64,
	name: String,
	children_data: Vec<Child>,
}

/// Test model representing a child
#[derive(Clone, PartialEq)]
struct Child {
	id: i64,
	parent_id: i64,
	name: String,
}

impl Reflectable for Parent {
	fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
		match name {
			"children_data" => {
				let boxed_children: Vec<Box<dyn Reflectable>> = self
					.children_data
					.iter()
					.map(|c| Box::new(c.clone()) as Box<dyn Reflectable>)
					.collect();
				Some(Box::new(boxed_children))
			}
			_ => None,
		}
	}

	fn get_relationship_mut(&mut self, name: &str) -> Option<&mut dyn Any> {
		match name {
			"children_data" => Some(&mut self.children_data as &mut dyn Any),
			_ => None,
		}
	}

	fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
		match name {
			"id" => Some(ScalarValue::Integer(self.id)),
			"name" => Some(ScalarValue::String(self.name.clone())),
			_ => None,
		}
	}

	fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
		match name {
			"id" => {
				self.id = value.as_integer()?;
				Ok(())
			}
			"name" => {
				self.name = value.as_string()?;
				Ok(())
			}
			_ => Err(ProxyError::AttributeNotFound(name.to_string())),
		}
	}

	fn set_relationship_attribute(
		&mut self,
		relationship: &str,
		_attribute: &str,
		_value: ScalarValue,
	) -> ProxyResult<()> {
		Err(ProxyError::RelationshipNotFound(relationship.to_string()))
	}
}

impl Reflectable for Child {
	fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any>> {
		None
	}

	fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
		None
	}

	fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
		match name {
			"id" => Some(ScalarValue::Integer(self.id)),
			"parent_id" => Some(ScalarValue::Integer(self.parent_id)),
			"name" => Some(ScalarValue::String(self.name.clone())),
			_ => None,
		}
	}

	fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
		match name {
			"id" => {
				self.id = value.as_integer()?;
				Ok(())
			}
			"parent_id" => {
				self.parent_id = value.as_integer()?;
				Ok(())
			}
			"name" => {
				self.name = value.as_string()?;
				Ok(())
			}
			_ => Err(ProxyError::AttributeNotFound(name.to_string())),
		}
	}

	fn set_relationship_attribute(
		&mut self,
		relationship: &str,
		_attribute: &str,
		_value: ScalarValue,
	) -> ProxyResult<()> {
		Err(ProxyError::RelationshipNotFound(relationship.to_string()))
	}
}

#[tokio::test]
async fn test_list_append() {
	// Test: Verify that appending to list proxy works correctly
	// Based on: test_list_append from SQLAlchemy

	let proxy = CollectionProxy::new("children_data", "name");
	assert_eq!(proxy.relationship, "children_data");
	assert_eq!(proxy.attribute, "name");

	let mut parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![],
	};

	// Manually append a child (simulating ORM append behavior)
	parent.children_data.push(Child {
		id: 1,
		parent_id: 1,
		name: "child1".to_string(),
	});

	let values = proxy.get_values(&parent).await.unwrap();
	assert_eq!(values.len(), 1);
	assert_eq!(values[0].as_string().unwrap(), "child1");
}

#[tokio::test]
async fn test_list_extend() {
	// Test: Verify that extending list proxy works correctly
	// Based on: test_list_extend from SQLAlchemy

	let proxy = CollectionProxy::new("children_data", "name");

	let mut parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![],
	};

	// Extend with multiple children
	parent.children_data.extend(vec![
		Child {
			id: 1,
			parent_id: 1,
			name: "child1".to_string(),
		},
		Child {
			id: 2,
			parent_id: 1,
			name: "child2".to_string(),
		},
	]);

	let values = proxy.get_values(&parent).await.unwrap();
	assert_eq!(values.len(), 2);
	assert_eq!(values[0].as_string().unwrap(), "child1");
	assert_eq!(values[1].as_string().unwrap(), "child2");
}

#[tokio::test]
async fn test_proxy_collection_creation() {
	// Test: Verify that collection proxy creation works correctly

	let proxy = CollectionProxy::new("posts", "title");
	assert_eq!(proxy.relationship, "posts");
	assert_eq!(proxy.attribute, "title");
	assert!(!proxy.unique);
}

#[tokio::test]
async fn test_proxy_collection_unique() {
	// Test: Verify that unique collection proxy works correctly

	let proxy = CollectionProxy::unique("posts", "title");
	assert_eq!(proxy.relationship, "posts");
	assert_eq!(proxy.attribute, "title");
	assert!(proxy.unique);
}

#[tokio::test]
async fn test_bulk_replace() {
	// Test: Verify that bulk replacement works correctly
	// Based on: test_bulk_replace from SQLAlchemy

	let proxy = CollectionProxy::new("children_data", "name");

	let mut parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![
			Child {
				id: 1,
				parent_id: 1,
				name: "old1".to_string(),
			},
			Child {
				id: 2,
				parent_id: 1,
				name: "old2".to_string(),
			},
		],
	};

	// Replace with new values
	parent.children_data = vec![
		Child {
			id: 3,
			parent_id: 1,
			name: "new1".to_string(),
		},
		Child {
			id: 4,
			parent_id: 1,
			name: "new2".to_string(),
		},
		Child {
			id: 5,
			parent_id: 1,
			name: "new3".to_string(),
		},
	];

	let values = proxy.get_values(&parent).await.unwrap();
	assert_eq!(values.len(), 3);
	assert_eq!(values[0].as_string().unwrap(), "new1");
	assert_eq!(values[1].as_string().unwrap(), "new2");
	assert_eq!(values[2].as_string().unwrap(), "new3");
}

#[tokio::test]
async fn test_contains() {
	// Test: Verify that contains operation works correctly

	let proxy = CollectionProxy::new("children_data", "name");

	let parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![Child {
			id: 1,
			parent_id: 1,
			name: "child1".to_string(),
		}],
	};

	let values = proxy.get_values(&parent).await.unwrap();
	let contains = values.iter().any(|v| v.as_string().unwrap() == "child1");
	assert!(contains);

	let not_contains = values.iter().any(|v| v.as_string().unwrap() == "child2");
	assert!(!not_contains);
}

#[tokio::test]
async fn test_remove() {
	// Test: Verify that remove operation works correctly

	let proxy = CollectionProxy::new("children_data", "name");

	let mut parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![
			Child {
				id: 1,
				parent_id: 1,
				name: "child1".to_string(),
			},
			Child {
				id: 2,
				parent_id: 1,
				name: "child2".to_string(),
			},
		],
	};

	// Remove child1
	parent.children_data.retain(|c| c.name != "child1");

	let values = proxy.get_values(&parent).await.unwrap();
	assert_eq!(values.len(), 1);
	assert_eq!(values[0].as_string().unwrap(), "child2");
}

#[tokio::test]
async fn test_proxy_collection_count() {
	// Test: Verify that count operation works correctly

	let proxy = CollectionProxy::new("children_data", "name");

	let parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![
			Child {
				id: 1,
				parent_id: 1,
				name: "child1".to_string(),
			},
			Child {
				id: 2,
				parent_id: 1,
				name: "child2".to_string(),
			},
		],
	};

	let values = proxy.get_values(&parent).await.unwrap();
	assert_eq!(values.len(), 2);
}

#[tokio::test]
async fn test_proxy_collection_empty() {
	// Test: Verify that empty collections are handled correctly
	// Based on: test_empty from SQLAlchemy

	let proxy = CollectionProxy::new("children_data", "name");

	let parent = Parent {
		id: 1,
		name: "Parent1".to_string(),
		children_data: vec![], // No children
	};

	let values = proxy.get_values(&parent).await.unwrap();
	assert_eq!(values.len(), 0);
}
