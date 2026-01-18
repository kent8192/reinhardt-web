//! Comprehensive tests for association proxies
//!
//! These tests are based on Django's many-to-many, many-to-one, one-to-one,
//! and generic relations tests, adapted for Rust's association proxy pattern.

use reinhardt_db::associations::prelude::*;

// ============================================================================
// Test Models
// ============================================================================

#[derive(Debug, Clone)]
struct User {
	#[allow(dead_code)]
	id: u32,
	#[allow(dead_code)]
	username: String,
	orders: Vec<Order>,
}

#[derive(Debug, Clone)]
struct Order {
	#[allow(dead_code)]
	id: u32,
	product_name: String,
	quantity: u32,
}

#[derive(Debug, Clone)]
struct Address {
	#[allow(dead_code)]
	id: u32,
	city: String,
	country: String,
}

#[derive(Debug, Clone)]
struct UserWithAddress {
	#[allow(dead_code)]
	id: u32,
	#[allow(dead_code)]
	username: String,
	address: Address,
}

// Generic relation test models
#[derive(Debug, Clone)]
struct Tag {
	#[allow(dead_code)]
	id: u32,
	name: String,
	content_type: String,
	#[allow(dead_code)]
	object_id: u32,
}

#[derive(Debug, Clone)]
struct BlogPost {
	#[allow(dead_code)]
	id: u32,
	#[allow(dead_code)]
	title: String,
	tags: Vec<Tag>,
}

// ============================================================================
// Many-to-Many Association Tests (Based on Django's many_to_many/tests.py)
// ============================================================================

#[cfg(test)]
mod many_to_many_tests {
	use super::*;

	/// Test: Basic many-to-many association collection access
	/// Django equivalent: ManyToManyTests::test_add
	#[test]
	fn test_m2m_collection_access() {
		let user = User {
			id: 1,
			username: "alice".to_string(),
			orders: vec![
				Order {
					id: 1,
					product_name: "Book".to_string(),
					quantity: 2,
				},
				Order {
					id: 2,
					product_name: "Pen".to_string(),
					quantity: 5,
				},
			],
		};

		let product_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.product_name);

		let products = product_proxy.get_all(&user);
		assert_eq!(products.len(), 2);
		assert_eq!(products[0], "Book");
		assert_eq!(products[1], "Pen");
	}

	/// Test: Count operation on many-to-many collection
	/// Django equivalent: ManyToManyTests::test_related_sets + count operations
	#[test]
	fn test_m2m_count() {
		let user = User {
			id: 1,
			username: "bob".to_string(),
			orders: vec![
				Order {
					id: 1,
					product_name: "Laptop".to_string(),
					quantity: 1,
				},
				Order {
					id: 2,
					product_name: "Mouse".to_string(),
					quantity: 2,
				},
				Order {
					id: 3,
					product_name: "Keyboard".to_string(),
					quantity: 1,
				},
			],
		};

		let order_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.product_name);

		assert_eq!(order_proxy.count(&user), 3);
		assert!(!order_proxy.is_empty(&user));
	}

	/// Test: Empty collection handling
	/// Django equivalent: ManyToManyTests::test_clear
	#[test]
	fn test_m2m_empty_collection() {
		let user = User {
			id: 1,
			username: "charlie".to_string(),
			orders: vec![],
		};

		let order_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.product_name);

		assert_eq!(order_proxy.count(&user), 0);
		assert!(order_proxy.is_empty(&user));
		assert_eq!(order_proxy.get_all(&user).len(), 0);
	}

	/// Test: Multiple association proxies on same collection
	/// Django equivalent: ManyToManyTests::test_selects (different attributes)
	#[test]
	fn test_m2m_multiple_proxies() {
		let user = User {
			id: 1,
			username: "dave".to_string(),
			orders: vec![
				Order {
					id: 1,
					product_name: "Tablet".to_string(),
					quantity: 1,
				},
				Order {
					id: 2,
					product_name: "Charger".to_string(),
					quantity: 3,
				},
			],
		};

		let product_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.product_name);

		let quantity_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.quantity);

		let products = product_proxy.get_all(&user);
		let quantities = quantity_proxy.get_all(&user);

		assert_eq!(products.len(), 2);
		assert_eq!(quantities.len(), 2);
		assert_eq!(products[0], "Tablet");
		assert_eq!(*quantities[0], 1);
		assert_eq!(products[1], "Charger");
		assert_eq!(*quantities[1], 3);
	}
}

// ============================================================================
// Many-to-One Association Tests (Based on Django's many_to_one/tests.py)
// ============================================================================

#[cfg(test)]
mod many_to_one_tests {
	use super::*;

	/// Test: Basic foreign key association access
	/// Django equivalent: ManyToManyTests::test_get (many_to_one context)
	#[test]
	fn test_fk_association_access() {
		let address = Address {
			id: 1,
			city: "Tokyo".to_string(),
			country: "Japan".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "eve".to_string(),
			address: address.clone(),
		};

		let city_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.city);

		assert_eq!(city_proxy.get(&user), "Tokyo");
	}

	/// Test: Nested attribute access through foreign key
	/// Django equivalent: ManyToManyTests::test_select_related
	#[test]
	fn test_fk_nested_attribute_access() {
		let address = Address {
			id: 1,
			city: "Paris".to_string(),
			country: "France".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "frank".to_string(),
			address,
		};

		let city_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.city);

		let country_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.country);

		assert_eq!(city_proxy.get(&user), "Paris");
		assert_eq!(country_proxy.get(&user), "France");
	}

	/// Test: Multiple proxies on same foreign key relationship
	/// Django equivalent: ManyToManyTests::test_explicit_fk
	#[test]
	fn test_fk_multiple_proxies() {
		let address = Address {
			id: 1,
			city: "Berlin".to_string(),
			country: "Germany".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "grace".to_string(),
			address,
		};

		let city_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.city);

		let country_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.country);

		// Both proxies should work independently
		assert_eq!(city_proxy.get(&user), "Berlin");
		assert_eq!(country_proxy.get(&user), "Germany");
	}
}

// ============================================================================
// One-to-One Association Tests (Based on Django's one_to_one/tests.py)
// ============================================================================

#[cfg(test)]
mod one_to_one_tests {
	use super::*;

	/// Test: Basic one-to-one association access
	/// Django equivalent: OneToOneTests::test_getter
	#[test]
	fn test_o2o_association_access() {
		let address = Address {
			id: 1,
			city: "London".to_string(),
			country: "UK".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "henry".to_string(),
			address,
		};

		let city_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.city);

		assert_eq!(city_proxy.get(&user), "London");
	}

	/// Test: One-to-one relationship attribute proxying
	/// Django equivalent: OneToOneTests::test_setter
	#[test]
	fn test_o2o_attribute_proxying() {
		let address = Address {
			id: 1,
			city: "Sydney".to_string(),
			country: "Australia".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "iris".to_string(),
			address,
		};

		let country_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.country);

		assert_eq!(country_proxy.get(&user), "Australia");
	}

	/// Test: Multiple one-to-one proxies
	/// Django equivalent: OneToOneTests::test_multiple_o2o
	#[test]
	fn test_o2o_multiple_proxies() {
		let address = Address {
			id: 1,
			city: "Rome".to_string(),
			country: "Italy".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "jack".to_string(),
			address,
		};

		let city_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.city);

		let country_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| &a.country);

		assert_eq!(city_proxy.get(&user), "Rome");
		assert_eq!(country_proxy.get(&user), "Italy");
	}
}

// ============================================================================
// Generic Relations Tests (Based on Django's generic_relations/tests.py)
// ============================================================================

#[cfg(test)]
mod generic_relations_tests {
	use super::*;

	/// Test: Generic relation association collection access
	/// Django equivalent: GenericRelationsTests::test_generic_relations_m2m_mimic
	#[test]
	fn test_generic_relation_collection() {
		let blog_post = BlogPost {
			id: 1,
			title: "Rust Programming".to_string(),
			tags: vec![
				Tag {
					id: 1,
					name: "rust".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
				Tag {
					id: 2,
					name: "programming".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
			],
		};

		let tag_proxy = AssociationCollection::new(|bp: &BlogPost| &bp.tags, |t: &Tag| &t.name);

		let tag_names = tag_proxy.get_all(&blog_post);
		assert_eq!(tag_names.len(), 2);
		assert_eq!(tag_names[0], "rust");
		assert_eq!(tag_names[1], "programming");
	}

	/// Test: Generic relation count operations
	/// Django equivalent: GenericRelationsTests::test_add_bulk + count
	#[test]
	fn test_generic_relation_count() {
		let blog_post = BlogPost {
			id: 1,
			title: "Web Development".to_string(),
			tags: vec![
				Tag {
					id: 1,
					name: "web".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
				Tag {
					id: 2,
					name: "frontend".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
				Tag {
					id: 3,
					name: "backend".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
			],
		};

		let tag_proxy = AssociationCollection::new(|bp: &BlogPost| &bp.tags, |t: &Tag| &t.name);

		assert_eq!(tag_proxy.count(&blog_post), 3);
		assert!(!tag_proxy.is_empty(&blog_post));
	}

	/// Test: Empty generic relation collection
	/// Django equivalent: GenericRelationsTests::test_clear
	#[test]
	fn test_generic_relation_empty() {
		let blog_post = BlogPost {
			id: 1,
			title: "Empty Post".to_string(),
			tags: vec![],
		};

		let tag_proxy = AssociationCollection::new(|bp: &BlogPost| &bp.tags, |t: &Tag| &t.name);

		assert_eq!(tag_proxy.count(&blog_post), 0);
		assert!(tag_proxy.is_empty(&blog_post));
	}

	/// Test: Multiple attribute access through generic relations
	/// Django equivalent: GenericRelationsTests::test_access_content_object
	#[test]
	fn test_generic_relation_multiple_attributes() {
		let blog_post = BlogPost {
			id: 1,
			title: "Database Design".to_string(),
			tags: vec![
				Tag {
					id: 1,
					name: "database".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
				Tag {
					id: 2,
					name: "sql".to_string(),
					content_type: "blogpost".to_string(),
					object_id: 1,
				},
			],
		};

		let name_proxy = AssociationCollection::new(|bp: &BlogPost| &bp.tags, |t: &Tag| &t.name);

		let content_type_proxy =
			AssociationCollection::new(|bp: &BlogPost| &bp.tags, |t: &Tag| &t.content_type);

		let names = name_proxy.get_all(&blog_post);
		let content_types = content_type_proxy.get_all(&blog_post);

		assert_eq!(names.len(), 2);
		assert_eq!(content_types.len(), 2);
		assert_eq!(names[0], "database");
		assert_eq!(content_types[0], "blogpost");
	}
}

// ============================================================================
// Complex Association Tests
// ============================================================================

#[cfg(test)]
mod complex_association_tests {
	use super::*;

	/// Test: Chained association proxies
	/// Simulates accessing deeply nested attributes
	#[test]
	fn test_chained_proxies() {
		let address = Address {
			id: 1,
			city: "Amsterdam".to_string(),
			country: "Netherlands".to_string(),
		};

		let user = UserWithAddress {
			id: 1,
			username: "kate".to_string(),
			address,
		};

		// First level proxy
		let address_proxy =
			AssociationProxy::new(|u: &UserWithAddress| &u.address, |a: &Address| a);

		// Access through proxy
		let addr = address_proxy.get(&user);
		assert_eq!(addr.city, "Amsterdam");
		assert_eq!(addr.country, "Netherlands");
	}

	/// Test: Collection proxy with filtering logic
	/// Django equivalent: ManyToManyQueryTests::test_exists_join_optimization
	#[test]
	fn test_collection_proxy_filtering() {
		let user = User {
			id: 1,
			username: "laura".to_string(),
			orders: vec![
				Order {
					id: 1,
					product_name: "Phone".to_string(),
					quantity: 1,
				},
				Order {
					id: 2,
					product_name: "Case".to_string(),
					quantity: 2,
				},
				Order {
					id: 3,
					product_name: "Charger".to_string(),
					quantity: 1,
				},
			],
		};

		let product_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.product_name);

		let products = product_proxy.get_all(&user);

		// Simulate filtering (would be done externally in real usage)
		let filtered: Vec<&&String> = products.iter().filter(|p| p.len() > 4).collect();

		assert_eq!(filtered.len(), 2); // "Phone" and "Charger"
	}

	/// Test: Association proxy with numeric types
	#[test]
	fn test_numeric_association_proxy() {
		let user = User {
			id: 1,
			username: "mike".to_string(),
			orders: vec![
				Order {
					id: 1,
					product_name: "Item1".to_string(),
					quantity: 5,
				},
				Order {
					id: 2,
					product_name: "Item2".to_string(),
					quantity: 10,
				},
				Order {
					id: 3,
					product_name: "Item3".to_string(),
					quantity: 3,
				},
			],
		};

		let quantity_proxy =
			AssociationCollection::new(|u: &User| &u.orders, |o: &Order| &o.quantity);

		let quantities = quantity_proxy.get_all(&user);
		let total: u32 = quantities.iter().map(|q| **q).sum();

		assert_eq!(quantities.len(), 3);
		assert_eq!(total, 18); // 5 + 10 + 3
	}
}
