//! Tests for hybrid property value operations
//! Based on PropertyValueTest from SQLAlchemy

use reinhardt_db::hybrid::prelude::*;

#[derive(Debug)]
struct Product {
	id: i32,
	name: String,
	price: f64,
}

impl Product {
	fn new(id: i32, name: String, price: f64) -> Self {
		Self { id, name, price }
	}
}

#[test]
fn test_hybrid_property_value_set_get() {
	// Test basic property getter
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let name_property = HybridProperty::new(|p: &Product| p.name.clone());
	let price_property = HybridProperty::new(|p: &Product| p.price);

	assert_eq!(name_property.get(&product), "Widget");
	assert_eq!(price_property.get(&product), 19.99);
}

#[test]
fn test_value_transformation() {
	// Test property that transforms the value
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.price * 1.1); // Add 10% tax

	let result = property.get(&product);
	assert!((result - 21.989).abs() < 0.01);
}

#[test]
fn test_string_value_transformation() {
	// Test property that transforms string values
	let product = Product::new(1, "widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.name.to_uppercase());

	assert_eq!(property.get(&product), "WIDGET");
}

#[test]
fn test_conditional_value() {
	// Test property with conditional logic
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| {
		if p.price > 20.0 {
			"expensive"
		} else {
			"affordable"
		}
	});

	assert_eq!(property.get(&product), "affordable");
}

#[test]
fn test_computed_value() {
	// Test property that computes a value
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| format!("{} - ${:.2}", p.name, p.price));

	assert_eq!(property.get(&product), "Widget - $19.99");
}

#[test]
fn test_nullable_value() {
	// Test property that might return None
	#[derive(Debug)]
	struct Item {
		value: Option<i32>,
	}

	let item = Item { value: Some(42) };
	let empty_item = Item { value: None };

	let property = HybridProperty::new(|i: &Item| i.value);

	assert_eq!(property.get(&item), Some(42));
	assert_eq!(property.get(&empty_item), None);
}

#[test]
fn test_boolean_value() {
	// Test property that returns boolean
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.price < 50.0);

	assert!(property.get(&product));
}

#[test]
fn test_numeric_comparison_value() {
	// Test property with numeric comparison
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.price.round() as i32);

	assert_eq!(property.get(&product), 20);
}

#[test]
fn test_value_with_multiple_fields() {
	// Test property that uses multiple fields
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| format!("#{}: {}", p.id, p.name));

	assert_eq!(property.get(&product), "#1: Widget");
}

#[test]
fn test_value_caching() {
	// Test that property can be called multiple times
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.price * 2.0);

	assert_eq!(property.get(&product), 39.98);
	assert_eq!(property.get(&product), 39.98); // Should return same value
}

#[test]
fn test_hybrid_value_with_expression() {
	// Test property with both instance value and SQL expression
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.price * 1.1)
		.with_expression(|| "price * 1.1".to_string());

	assert!((property.get(&product) - 21.989).abs() < 0.01);
	assert_eq!(property.expression(), Some("price * 1.1".to_string()));
}

#[test]
fn test_zero_value() {
	// Test property with zero value
	let product = Product::new(1, "Free Item".to_string(), 0.0);

	let property = HybridProperty::new(|p: &Product| p.price);

	assert_eq!(property.get(&product), 0.0);
}

#[test]
fn test_negative_value() {
	// Test property with negative value (e.g., discount)
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| p.price - 25.0);

	assert!((property.get(&product) - (-5.01)).abs() < 0.01);
}

#[test]
fn test_value_precision() {
	// Test property value precision
	let product = Product::new(1, "Widget".to_string(), 19.99);

	let property = HybridProperty::new(|p: &Product| format!("{:.2}", p.price));

	assert_eq!(property.get(&product), "19.99");
}

#[test]
fn test_collection_value() {
	// Test property that returns a collection-like value
	#[derive(Debug)]
	struct Store {
		products: Vec<String>,
	}

	let store = Store {
		products: vec!["A".to_string(), "B".to_string(), "C".to_string()],
	};

	let property = HybridProperty::new(|s: &Store| s.products.len());

	assert_eq!(property.get(&store), 3);
}
