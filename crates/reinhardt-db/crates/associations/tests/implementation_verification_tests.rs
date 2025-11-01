//! Implementation verification tests
//!
//! These tests verify that the association proxy implementation actually works
//! and is not just a skeleton. They test edge cases, boundary conditions, and
//! proper behavior of the implementation.

use reinhardt_associations::prelude::*;

#[derive(Debug, Clone, PartialEq)]
struct Article {
	id: u32,
	title: String,
	comments: Vec<Comment>,
}

#[derive(Debug, Clone, PartialEq)]
struct Comment {
	id: u32,
	author: String,
	text: String,
	votes: i32,
}

#[derive(Debug, Clone, PartialEq)]
struct Company {
	name: String,
	address: CompanyAddress,
}

#[derive(Debug, Clone, PartialEq)]
struct CompanyAddress {
	street: String,
	city: String,
	postal_code: String,
}

// ============================================================================
// AssociationCollection Implementation Verification
// ============================================================================

#[test]
fn test_collection_getter_is_actually_called() {
	let article = Article {
		id: 1,
		title: "Test Article".to_string(),
		comments: vec![
			Comment {
				id: 1,
				author: "Alice".to_string(),
				text: "Great article!".to_string(),
				votes: 10,
			},
			Comment {
				id: 2,
				author: "Bob".to_string(),
				text: "Thanks for sharing".to_string(),
				votes: 5,
			},
		],
	};

	// Create proxy that accesses the comments collection
	let author_proxy =
		AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	// Verify the collection_getter actually retrieves the comments
	let authors = author_proxy.get_all(&article);
	assert_eq!(authors.len(), 2);
	assert_eq!(authors[0], "Alice");
	assert_eq!(authors[1], "Bob");
}

#[test]
fn test_attribute_getter_is_actually_called() {
	let article = Article {
		id: 1,
		title: "Test Article".to_string(),
		comments: vec![
			Comment {
				id: 1,
				author: "Charlie".to_string(),
				text: "First!".to_string(),
				votes: 100,
			},
			Comment {
				id: 2,
				author: "Dave".to_string(),
				text: "Second".to_string(),
				votes: 50,
			},
		],
	};

	// Verify that different attribute getters work correctly
	let text_proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.text);

	let votes_proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.votes);

	let texts = text_proxy.get_all(&article);
	let votes = votes_proxy.get_all(&article);

	assert_eq!(texts[0], "First!");
	assert_eq!(texts[1], "Second");
	assert_eq!(*votes[0], 100);
	assert_eq!(*votes[1], 50);
}

#[test]
fn test_count_reflects_actual_collection_size() {
	let empty_article = Article {
		id: 1,
		title: "No Comments".to_string(),
		comments: vec![],
	};

	let single_comment_article = Article {
		id: 2,
		title: "One Comment".to_string(),
		comments: vec![Comment {
			id: 1,
			author: "Eve".to_string(),
			text: "Solo".to_string(),
			votes: 0,
		}],
	};

	let many_comments_article = Article {
		id: 3,
		title: "Many Comments".to_string(),
		comments: (0..100)
			.map(|i| Comment {
				id: i,
				author: format!("User{}", i),
				text: format!("Comment {}", i),
				votes: i as i32,
			})
			.collect(),
	};

	let proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	assert_eq!(proxy.count(&empty_article), 0);
	assert_eq!(proxy.count(&single_comment_article), 1);
	assert_eq!(proxy.count(&many_comments_article), 100);
}

#[test]
fn test_is_empty_correctly_detects_empty_collections() {
	let empty_article = Article {
		id: 1,
		title: "Empty".to_string(),
		comments: vec![],
	};

	let non_empty_article = Article {
		id: 2,
		title: "Not Empty".to_string(),
		comments: vec![Comment {
			id: 1,
			author: "Frank".to_string(),
			text: "Content".to_string(),
			votes: 0,
		}],
	};

	let proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	assert!(proxy.is_empty(&empty_article));
	assert!(!proxy.is_empty(&non_empty_article));
}

#[test]
fn test_collection_order_is_preserved() {
	let article = Article {
		id: 1,
		title: "Ordered Comments".to_string(),
		comments: vec![
			Comment {
				id: 3,
				author: "Third".to_string(),
				text: "C".to_string(),
				votes: 0,
			},
			Comment {
				id: 1,
				author: "First".to_string(),
				text: "A".to_string(),
				votes: 0,
			},
			Comment {
				id: 2,
				author: "Second".to_string(),
				text: "B".to_string(),
				votes: 0,
			},
		],
	};

	let author_proxy =
		AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	let authors = author_proxy.get_all(&article);

	// Verify order is preserved (not sorted by id or name)
	assert_eq!(authors[0], "Third");
	assert_eq!(authors[1], "First");
	assert_eq!(authors[2], "Second");
}

#[test]
fn test_negative_values_handled_correctly() {
	let article = Article {
		id: 1,
		title: "Controversial".to_string(),
		comments: vec![
			Comment {
				id: 1,
				author: "Downvoted".to_string(),
				text: "Bad take".to_string(),
				votes: -50,
			},
			Comment {
				id: 2,
				author: "Upvoted".to_string(),
				text: "Good take".to_string(),
				votes: 100,
			},
		],
	};

	let votes_proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.votes);

	let votes = votes_proxy.get_all(&article);
	assert_eq!(*votes[0], -50);
	assert_eq!(*votes[1], 100);
}

// ============================================================================
// AssociationProxy Implementation Verification
// ============================================================================

#[test]
fn test_proxy_association_getter_is_called() {
	let company = Company {
		name: "Tech Corp".to_string(),
		address: CompanyAddress {
			street: "123 Main St".to_string(),
			city: "San Francisco".to_string(),
			postal_code: "94102".to_string(),
		},
	};

	let city_proxy = AssociationProxy::new(|c: &Company| &c.address, |a: &CompanyAddress| &a.city);

	// Verify the association_getter actually accesses the address
	let city = city_proxy.get(&company);
	assert_eq!(city, "San Francisco");
}

#[test]
fn test_proxy_attribute_getter_is_called() {
	let company = Company {
		name: "Startup Inc".to_string(),
		address: CompanyAddress {
			street: "456 Oak Ave".to_string(),
			city: "New York".to_string(),
			postal_code: "10001".to_string(),
		},
	};

	// Test different attribute getters
	let street_proxy =
		AssociationProxy::new(|c: &Company| &c.address, |a: &CompanyAddress| &a.street);

	let postal_proxy = AssociationProxy::new(
		|c: &Company| &c.address,
		|a: &CompanyAddress| &a.postal_code,
	);

	assert_eq!(street_proxy.get(&company), "456 Oak Ave");
	assert_eq!(postal_proxy.get(&company), "10001");
}

#[test]
fn test_proxy_returns_reference_to_actual_data() {
	let company = Company {
		name: "MegaCorp".to_string(),
		address: CompanyAddress {
			street: "789 Pine Rd".to_string(),
			city: "Seattle".to_string(),
			postal_code: "98101".to_string(),
		},
	};

	let city_proxy = AssociationProxy::new(|c: &Company| &c.address, |a: &CompanyAddress| &a.city);

	let city_ref = city_proxy.get(&company);

	// Verify we get a reference to the actual data, not a copy
	assert_eq!(city_ref, "Seattle");
	assert_eq!(city_ref.len(), 7);
	assert!(city_ref.starts_with("Sea"));
}

#[test]
fn test_proxy_with_different_source_instances() {
	let company1 = Company {
		name: "Company A".to_string(),
		address: CompanyAddress {
			street: "100 First St".to_string(),
			city: "Boston".to_string(),
			postal_code: "02101".to_string(),
		},
	};

	let company2 = Company {
		name: "Company B".to_string(),
		address: CompanyAddress {
			street: "200 Second St".to_string(),
			city: "Austin".to_string(),
			postal_code: "78701".to_string(),
		},
	};

	let city_proxy = AssociationProxy::new(|c: &Company| &c.address, |a: &CompanyAddress| &a.city);

	// Verify the proxy works correctly with different source instances
	assert_eq!(city_proxy.get(&company1), "Boston");
	assert_eq!(city_proxy.get(&company2), "Austin");
}

// ============================================================================
// Complex Scenario Tests
// ============================================================================

#[test]
fn test_collection_with_large_dataset() {
	let large_article = Article {
		id: 1,
		title: "Popular Article".to_string(),
		comments: (0..10_000)
			.map(|i| Comment {
				id: i,
				author: format!("User{:05}", i),
				text: format!("Comment number {}", i),
				votes: (i % 100) as i32,
			})
			.collect(),
	};

	let proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	let authors = proxy.get_all(&large_article);
	assert_eq!(authors.len(), 10_000);
	assert_eq!(authors[0], "User00000");
	assert_eq!(authors[9_999], "User09999");
	assert_eq!(proxy.count(&large_article), 10_000);
}

#[test]
fn test_unicode_strings_handled_correctly() {
	let article = Article {
		id: 1,
		title: "国際記事".to_string(),
		comments: vec![
			Comment {
				id: 1,
				author: "山田太郎".to_string(),
				text: "素晴らしい記事です！".to_string(),
				votes: 10,
			},
			Comment {
				id: 2,
				author: "Müller".to_string(),
				text: "Sehr gut! 🎉".to_string(),
				votes: 5,
			},
		],
	};

	let author_proxy =
		AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	let text_proxy = AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.text);

	let authors = author_proxy.get_all(&article);
	let texts = text_proxy.get_all(&article);

	assert_eq!(authors[0], "山田太郎");
	assert_eq!(authors[1], "Müller");
	assert_eq!(texts[0], "素晴らしい記事です！");
	assert_eq!(texts[1], "Sehr gut! 🎉");
}

#[test]
fn test_empty_string_values() {
	let article = Article {
		id: 1,
		title: "Test".to_string(),
		comments: vec![
			Comment {
				id: 1,
				author: "".to_string(),
				text: "".to_string(),
				votes: 0,
			},
			Comment {
				id: 2,
				author: "Anonymous".to_string(),
				text: "".to_string(),
				votes: 0,
			},
		],
	};

	let author_proxy =
		AssociationCollection::new(|a: &Article| &a.comments, |c: &Comment| &c.author);

	let authors = author_proxy.get_all(&article);
	assert_eq!(authors[0], "");
	assert_eq!(authors[1], "Anonymous");
	assert_eq!(author_proxy.count(&article), 2);
}
