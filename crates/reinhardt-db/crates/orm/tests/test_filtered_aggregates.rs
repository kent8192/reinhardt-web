// Standalone test for filtered aggregates
use reinhardt_orm::aggregation::Aggregate;
use reinhardt_orm::expressions::Q;

#[test]
fn test_aggregate_with_filter_sql_generation() {
    // Test that filter generates proper SQL
    let filter = Q::new("name", "startswith", "test");
    let agg = Aggregate::sum("age").with_filter(filter);

    let sql = agg.to_sql();
    assert!(sql.contains("SUM(age)"));
    assert!(sql.contains("FILTER (WHERE"));
    assert!(sql.contains("name"));
}

#[test]
fn test_aggregate_with_complex_filter() {
    // Test complex filter (AND + NOT)
    let q1 = Q::new("name", "=", "test2");
    let q2 = Q::new("name", "=", "test").not();
    let filter = q1.and(q2);

    let agg = Aggregate::sum("age").with_filter(filter);

    let sql = agg.to_sql();
    assert!(sql.contains("SUM(age)"));
    assert!(sql.contains("FILTER (WHERE"));
}

#[test]
fn test_aggregate_with_negated_filter() {
    // Test negated filter
    let filter = Q::new("name", "=", "test2").not();
    let agg = Aggregate::sum("age").with_filter(filter);

    let sql = agg.to_sql();
    assert!(sql.contains("SUM(age)"));
    assert!(sql.contains("FILTER (WHERE"));
    assert!(sql.contains("NOT"));
}

#[test]
fn test_count_with_filter() {
    // Test Count with filter
    let filter = Q::new("status", "=", "active");
    let agg = Aggregate::count(Some("id")).with_filter(filter);

    let sql = agg.to_sql();
    assert!(sql.contains("COUNT(id)"));
    assert!(sql.contains("FILTER (WHERE"));
}

#[test]
fn test_aggregate_with_alias_and_filter() {
    // Test aggregate with both alias and filter
    let filter = Q::new("age", ">", "18");
    let agg = Aggregate::sum("price")
        .with_filter(filter)
        .with_alias("total_adult_price");

    let sql = agg.to_sql();
    assert!(sql.contains("SUM(price)"));
    assert!(sql.contains("FILTER (WHERE"));
    assert!(sql.contains("AS total_adult_price"));
}

#[test]
fn test_aggregate_with_distinct_and_filter() {
    // Test aggregate with distinct and filter
    let filter = Q::new("category", "=", "electronics");
    let agg = Aggregate::count(Some("product_id"))
        .distinct()
        .with_filter(filter);

    let sql = agg.to_sql();
    assert!(sql.contains("COUNT(DISTINCT product_id)"));
    assert!(sql.contains("FILTER (WHERE"));
}

#[test]
fn test_empty_filter() {
    // Test with empty filter
    let filter = Q::empty();
    let agg = Aggregate::count(Some("pk")).with_filter(filter);

    let sql = agg.to_sql();
    assert!(sql.contains("COUNT(pk)"));
    assert!(sql.contains("FILTER (WHERE"));
}
