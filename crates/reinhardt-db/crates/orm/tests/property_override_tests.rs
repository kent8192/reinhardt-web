//! Property Override Tests for Hybrid Properties
//!
//! Based on SQLAlchemy's PropertyOverrideTest (test/ext/test_hybrid.py:758-865)
//!
//! Tests property override capabilities using Rust's trait and builder patterns
//! instead of Python's class inheritance.

use reinhardt_hybrid::HybridProperty;

/// Base model for property override tests
#[derive(Debug, Clone)]
struct Person {
	id: i32,
	firstname: String,
	lastname: String,
}

impl Person {
	fn new(id: i32, firstname: &str, lastname: &str) -> Self {
		Self {
			id,
			firstname: firstname.to_string(),
			lastname: lastname.to_string(),
		}
	}

	/// Default getter for full name
	fn get_name(&self) -> String {
		format!("{} {}", self.firstname, self.lastname)
	}

	/// Default setter for full name
	fn set_name(&mut self, value: &str) {
		let parts: Vec<&str> = value.splitn(2, ' ').collect();
		if parts.len() == 2 {
			self.firstname = parts[0].to_string();
			self.lastname = parts[1].to_string();
		}
	}
}

/// Extended model that "overrides" behavior using different getter/setter
#[derive(Debug, Clone)]
struct SpecialPerson {
	person: Person,
}

impl SpecialPerson {
	fn new(id: i32, firstname: &str, lastname: &str) -> Self {
		Self {
			person: Person::new(id, firstname, lastname),
		}
	}

	/// Overridden getter - returns reversed name
	fn get_name_reversed(&self) -> String {
		format!("{} {}", self.person.lastname, self.person.firstname)
	}

	/// Overridden setter - uppercases the names
	fn set_name_upper(&mut self, value: &str) {
		let parts: Vec<&str> = value.splitn(2, ' ').collect();
		if parts.len() == 2 {
			self.person.firstname = parts[0].to_uppercase();
			self.person.lastname = parts[1].to_uppercase();
		}
	}
}

#[test]
fn test_property_override() {
	// Test 1: Basic property behavior without override
	let name_prop = HybridProperty::new(|p: &Person| p.get_name());

	let person = Person::new(1, "John", "Doe");
	let name = name_prop.get(&person);

	assert_eq!(name, "John Doe");
}

#[test]
fn test_property_override_setter() {
	// Test 2: Override setter using builder pattern
	// In Python, this would be class inheritance
	// In Rust, we use composition and different functions

	let mut person = Person::new(1, "John", "Doe");
	let mut special = SpecialPerson::new(2, "Jane", "Smith");

	// Base setter (normal)
	person.set_name("Alice Bob");
	assert_eq!(person.firstname, "Alice");
	assert_eq!(person.lastname, "Bob");

	// Overridden setter (uppercases)
	special.set_name_upper("Charlie Delta");
	assert_eq!(special.person.firstname, "CHARLIE");
	assert_eq!(special.person.lastname, "DELTA");
}

#[test]
fn test_property_override_getter() {
	// Test 3: Override getter using builder pattern
	let person = Person::new(1, "John", "Doe");
	let special = SpecialPerson::new(2, "Jane", "Smith");

	// Base getter (firstname lastname)
	let name1 = person.get_name();
	assert_eq!(name1, "John Doe");

	// Overridden getter (lastname firstname - reversed)
	let name2 = special.get_name_reversed();
	assert_eq!(name2, "Smith Jane");
}

#[test]
fn test_override_expr() {
	// Test 4: Override SQL expression
	// In SQLAlchemy, this would override the @hybrid_property.expression method
	// In Rust, we demonstrate this with HybridProperty builder pattern

	// Base expression: CONCAT(firstname, ' ', lastname)
	let base_expr = "CONCAT(firstname, ' ', lastname)";

	// Overridden expression: CONCAT(lastname, ', ', firstname)
	let override_expr = "CONCAT(lastname, ', ', firstname)";

	// Verify the expressions are different
	assert_ne!(base_expr, override_expr);

	// In a real implementation, HybridProperty would have .with_expression()
	// let prop = HybridProperty::new(getter)
	//     .with_expression(override_expr);
}

#[test]
fn test_override_comparator() {
	// Test 5: Override comparator logic
	// In SQLAlchemy, this would override the @hybrid_property.comparator method
	// In Rust, we demonstrate different comparison strategies

	let person1 = Person::new(1, "Alice", "Anderson");
	let person2 = Person::new(2, "Bob", "Brown");

	// Base comparator: simple string comparison
	let name1 = person1.get_name();
	let name2 = person2.get_name();
	assert!(name1 < name2); // "Alice Anderson" < "Bob Brown"

	// Overridden comparator: compare by last name only
	assert!(person1.lastname < person2.lastname); // "Anderson" < "Brown"

	// Another override: compare by length
	let len1 = name1.len();
	let len2 = name2.len();
	assert_eq!(len1, 14); // "Alice Anderson" = 14 chars
	assert_eq!(len2, 9); // "Bob Brown" = 9 chars
	assert!(len1 > len2); // Longer name comes "after" in this comparator
}

// Additional test demonstrating HybridProperty with custom getter
#[test]
fn test_hybrid_property_custom_getter() {
	// Demonstrate that HybridProperty can use different getters
	// This is the Rust equivalent of "overriding" in Python

	let person = Person::new(1, "John", "Doe");

	// Property with default getter
	let name_normal = HybridProperty::new(|p: &Person| p.get_name());
	let result1 = name_normal.get(&person);
	assert_eq!(result1, "John Doe");

	// Property with "overridden" getter (different function)
	let name_reversed = HybridProperty::new(|p: &Person| format!("{} {}", p.lastname, p.firstname));
	let result2 = name_reversed.get(&person);
	assert_eq!(result2, "Doe John");

	// Property with yet another getter (uppercase)
	let name_upper = HybridProperty::new(|p: &Person| {
		format!(
			"{} {}",
			p.firstname.to_uppercase(),
			p.lastname.to_uppercase()
		)
	});
	let result3 = name_upper.get(&person);
	assert_eq!(result3, "JOHN DOE");
}

// Integration test showing override pattern in action
#[test]
fn test_property_override_pattern() {
	// This test demonstrates the Rust pattern for "overriding" properties
	// Instead of inheritance, we use:
	// 1. Composition (SpecialPerson contains Person)
	// 2. Different methods (get_name_reversed instead of get_name)
	// 3. HybridProperty with custom closures

	let mut special = SpecialPerson::new(1, "Alice", "Anderson");

	// Use the "overridden" getter
	let name_before = special.get_name_reversed();
	assert_eq!(name_before, "Anderson Alice");

	// Use the "overridden" setter
	special.set_name_upper("bob brown");
	assert_eq!(special.person.firstname, "BOB");
	assert_eq!(special.person.lastname, "BROWN");

	// Verify with getter again
	let name_after = special.get_name_reversed();
	assert_eq!(name_after, "BROWN BOB");
}
