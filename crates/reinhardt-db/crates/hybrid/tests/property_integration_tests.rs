//! Integration tests for hybrid properties
//! Based on InplaceCreationTest and SynonymOfPropertyTest from SQLAlchemy

use reinhardt_hybrid::prelude::*;

#[derive(Debug)]
struct Article {
    id: i32,
    title: String,
    body: String,
    view_count: i32,
}

impl Article {
    fn new(id: i32, title: String, body: String, view_count: i32) -> Self {
        Self {
            id,
            title,
            body,
            view_count,
        }
    }
}

#[test]
fn test_property_integration_basic() {
    // Test basic property integration
    let article = Article::new(1, "Hello".to_string(), "World".to_string(), 100);

    let title_prop = HybridProperty::new(|a: &Article| a.title.clone())
        .with_expression(|| "articles.title".to_string());

    assert_eq!(title_prop.get(&article), "Hello");
    assert_eq!(title_prop.expression(), Some("articles.title".to_string()));
}

#[test]
fn test_property_integration_with_transformation() {
    // Test property with transformation in integration context
    let article = Article::new(1, "hello".to_string(), "world".to_string(), 100);

    let prop = HybridProperty::new(|a: &Article| a.title.to_uppercase())
        .with_expression(|| "UPPER(articles.title)".to_string());

    assert_eq!(prop.get(&article), "HELLO");
    assert_eq!(prop.expression(), Some("UPPER(articles.title)".to_string()));
}

#[test]
fn test_multiple_properties_integration() {
    // Test multiple properties working together
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 150);

    let title_prop = HybridProperty::new(|a: &Article| a.title.clone());
    let body_prop = HybridProperty::new(|a: &Article| a.body.clone());
    let views_prop = HybridProperty::new(|a: &Article| a.view_count);

    assert_eq!(title_prop.get(&article), "Test");
    assert_eq!(body_prop.get(&article), "Content");
    assert_eq!(views_prop.get(&article), 150);
}

#[test]
fn test_property_with_computed_value_integration() {
    // Test property that computes value from multiple fields
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 150);

    let summary_prop =
        HybridProperty::new(|a: &Article| format!("{}: {} views", a.title, a.view_count))
            .with_expression(|| "CONCAT(title, ': ', view_count, ' views')".to_string());

    assert_eq!(summary_prop.get(&article), "Test: 150 views");
    assert!(summary_prop.expression().unwrap().contains("CONCAT"));
}

#[test]
fn test_property_chaining_integration() {
    // Test that properties can be used in sequence
    let article = Article::new(1, "test".to_string(), "body".to_string(), 100);

    let prop1 = HybridProperty::new(|a: &Article| a.title.to_uppercase());
    let result1 = prop1.get(&article);

    // Use move to take ownership of result1
    let prop2 = HybridProperty::new(move |_a: &Article| result1.len());

    assert_eq!(prop2.get(&article), 4);
}

#[test]
fn test_property_with_conditional_integration() {
    // Test property with conditional logic in integration context
    let popular = Article::new(1, "Popular".to_string(), "Content".to_string(), 1000);
    let unpopular = Article::new(2, "Unpopular".to_string(), "Content".to_string(), 10);

    let popularity_prop = HybridProperty::new(|a: &Article| {
        if a.view_count > 500 {
            "popular"
        } else {
            "unpopular"
        }
    })
    .with_expression(|| {
        "CASE WHEN view_count > 500 THEN 'popular' ELSE 'unpopular' END".to_string()
    });

    assert_eq!(popularity_prop.get(&popular), "popular");
    assert_eq!(popularity_prop.get(&unpopular), "unpopular");
}

#[test]
fn test_method_integration_basic() {
    // Test basic method integration
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 100);

    let increment_views = HybridMethod::new(|a: &Article, amount: i32| a.view_count + amount)
        .with_expression(|amount: i32| format!("view_count + {}", amount));

    assert_eq!(increment_views.call(&article, 50), 150);
    assert_eq!(
        increment_views.expression(50),
        Some("view_count + 50".to_string())
    );
}

#[test]
fn test_method_with_string_manipulation() {
    // Test method that manipulates strings
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 100);

    let prepend_method =
        HybridMethod::new(|a: &Article, prefix: String| format!("{}{}", prefix, a.title))
            .with_expression(|prefix: String| format!("CONCAT('{}', title)", prefix));

    assert_eq!(
        prepend_method.call(&article, "[New] ".to_string()),
        "[New] Test"
    );
}

#[test]
fn test_property_expression_without_instance() {
    // Test that expression can exist without being called on instance
    let prop: HybridProperty<Article, String> = HybridProperty::new(|_a: &Article| String::new())
        .with_expression(|| "SELECT title FROM articles".to_string());

    assert_eq!(
        prop.expression(),
        Some("SELECT title FROM articles".to_string())
    );
}

#[test]
fn test_property_unit_with_references() {
    // Test property that uses borrowed data
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 100);

    let prop = HybridProperty::new(|a: &Article| a.title.as_str().len());

    assert_eq!(prop.get(&article), 4);
}

#[test]
fn test_method_with_multiple_calls() {
    // Test that method can be called multiple times
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 100);

    let multiply_views =
        HybridMethod::new(|a: &Article, factor: f64| (a.view_count as f64 * factor) as i32);

    assert_eq!(multiply_views.call(&article, 2.0), 200);
    assert_eq!(multiply_views.call(&article, 3.0), 300);
    assert_eq!(multiply_views.call(&article, 0.5), 50);
}

#[test]
fn test_synonym_behavior() {
    // Test synonym-like behavior (property accessing another property)
    let article = Article::new(1, "Test".to_string(), "Content".to_string(), 100);

    let original_prop = HybridProperty::new(|a: &Article| a.view_count);
    // Use move to take ownership of original_prop
    let synonym_prop = HybridProperty::new(move |a: &Article| original_prop.get(a));

    // Only test synonym_prop since original_prop was moved
    assert_eq!(synonym_prop.get(&article), 100);
}

#[test]
fn test_expression_property() {
    // Test property focused on SQL expression generation
    let prop: HybridProperty<Article, String> =
        HybridProperty::new(|a: &Article| a.title.clone()).with_expression(|| "title".to_string());

    assert_eq!(prop.expression(), Some("title".to_string()));
}

#[test]
fn test_hasattr_simulation() {
    // Test that we can check if expression exists
    let prop_with_expr =
        HybridProperty::new(|a: &Article| a.title.clone()).with_expression(|| "title".to_string());

    let prop_without_expr = HybridProperty::new(|a: &Article| a.title.clone());

    assert!(prop_with_expr.expression().is_some());
    assert!(prop_without_expr.expression().is_none());
}
