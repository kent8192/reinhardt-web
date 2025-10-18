//! Tests for hybrid property expressions
//! Based on PropertyExpressionTest from SQLAlchemy

use reinhardt_hybrid::prelude::*;

#[derive(Debug)]
struct User {
    id: i32,
    first_name: String,
    last_name: String,
    value: i32,
}

impl User {
    fn new(id: i32, first_name: String, last_name: String, value: i32) -> Self {
        Self {
            id,
            first_name,
            last_name,
            value,
        }
    }
}

#[test]
fn test_property_with_instance_expression() {
    // Test hybrid property with both instance and class-level expressions
    let user = User::new(1, "John".to_string(), "Doe".to_string(), 15);

    let property = HybridProperty::new(|u: &User| u.value - 5)
        .with_expression(|| "users.value - 5".to_string());

    assert_eq!(property.get(&user), 10);
    assert_eq!(property.expression(), Some("users.value - 5".to_string()));
}

#[test]
fn test_expression_with_function() {
    // Test expression that uses SQL functions
    let user = User::new(1, "John".to_string(), "Doe".to_string(), 0);

    let property = HybridProperty::new(|u: &User| format!("{} {}", u.first_name, u.last_name))
        .with_expression(|| "CONCAT(first_name, ' ', last_name)".to_string());

    assert_eq!(property.get(&user), "John Doe");
    assert_eq!(
        property.expression(),
        Some("CONCAT(first_name, ' ', last_name)".to_string())
    );
}

#[test]
fn test_expression_with_multiple_columns() {
    // Test expression that references multiple columns
    let user = User::new(1, "John".to_string(), "Doe".to_string(), 100);

    let property = HybridProperty::new(|u: &User| u.value * 2)
        .with_expression(|| "users.value * 2".to_string());

    assert_eq!(property.get(&user), 200);
    assert_eq!(property.expression(), Some("users.value * 2".to_string()));
}

#[test]
fn test_unnamed_expression() {
    // Test expression without explicit name (uses lambda)
    let user = User::new(1, "Jane".to_string(), "Smith".to_string(), 0);

    let property = HybridProperty::new(|u: &User| u.first_name.clone());

    assert_eq!(property.get(&user), "Jane");
}

#[test]
fn test_expression_with_conditional() {
    // Test expression with conditional logic
    let user = User::new(1, "John".to_string(), "Doe".to_string(), 5);

    let property = HybridProperty::new(|u: &User| {
        if u.value > 10 {
            "high".to_string()
        } else {
            "low".to_string()
        }
    })
    .with_expression(|| "CASE WHEN value > 10 THEN 'high' ELSE 'low' END".to_string());

    assert_eq!(property.get(&user), "low");
    assert!(property.expression().unwrap().contains("CASE WHEN"));
}

#[test]
fn test_sql_expression_concat() {
    // Test SqlExpression concat functionality
    let expr = SqlExpression::concat(&["first_name", "' '", "last_name"]);
    assert_eq!(expr.sql, "CONCAT(first_name, ' ', last_name)");
}

#[test]
fn test_sql_expression_upper() {
    // Test SqlExpression upper functionality
    let expr = SqlExpression::upper("email");
    assert_eq!(expr.sql, "UPPER(email)");
}

#[test]
fn test_hybrid_property_expression_lower() {
    // Test SqlExpression lower functionality
    let expr = SqlExpression::lower("name");
    assert_eq!(expr.sql, "LOWER(name)");
}

#[test]
fn test_sql_expression_coalesce() {
    // Test SqlExpression coalesce functionality
    let expr = SqlExpression::coalesce("optional_field", "'default'");
    assert_eq!(expr.sql, "COALESCE(optional_field, 'default')");
}

#[test]
fn test_expression_trait_implementation() {
    // Test Expression trait implementation for SqlExpression
    let expr = SqlExpression::new("SELECT * FROM users");
    assert_eq!(expr.to_sql(), "SELECT * FROM users");
}

#[test]
fn test_expression_trait_for_string() {
    // Test Expression trait implementation for String
    let sql = "SELECT id FROM users".to_string();
    assert_eq!(sql.to_sql(), "SELECT id FROM users");
}

#[test]
fn test_expression_trait_for_str() {
    // Test Expression trait implementation for &str
    let sql = "SELECT name FROM users";
    assert_eq!(sql.to_sql(), "SELECT name FROM users");
}

#[test]
fn test_property_with_complex_expression() {
    // Test property with complex SQL expression
    let user = User::new(1, "John".to_string(), "Doe".to_string(), 100);

    let property = HybridProperty::new(|u: &User| u.value as f64 / 100.0)
        .with_expression(|| "CAST(value AS FLOAT) / 100.0".to_string());

    assert_eq!(property.get(&user), 1.0);
    assert!(property.expression().unwrap().contains("CAST"));
}

#[test]
fn test_expression_with_subquery() {
    // Test expression that could represent a subquery
    let property: HybridProperty<User, String> = HybridProperty::new(|_u: &User| {
        "result".to_string()
    })
    .with_expression(|| "(SELECT COUNT(*) FROM orders WHERE user_id = users.id)".to_string());

    assert!(property.expression().unwrap().contains("SELECT"));
    assert!(property.expression().unwrap().contains("subquery") == false);
}

#[test]
fn test_multiple_expressions() {
    // Test multiple properties with different expressions
    let user = User::new(1, "John".to_string(), "Doe".to_string(), 50);

    let prop1 = HybridProperty::new(|u: &User| u.first_name.clone())
        .with_expression(|| "users.first_name".to_string());

    let prop2 = HybridProperty::new(|u: &User| u.last_name.clone())
        .with_expression(|| "users.last_name".to_string());

    assert_eq!(prop1.get(&user), "John");
    assert_eq!(prop2.get(&user), "Doe");
    assert_eq!(prop1.expression(), Some("users.first_name".to_string()));
    assert_eq!(prop2.expression(), Some("users.last_name".to_string()));
}
