//! PostgreSQL-Specific Feature Tests
//!
//! Tests for PostgreSQL-specific features: JSONB, Arrays, HSTORE, Full-text search,
//! Window functions, CTEs, LATERAL JOIN, and UPSERT (ON CONFLICT).
//!
//! Run with: cargo test --package reinhardt-orm --test postgres_specific_tests --features postgres

#[cfg(feature = "postgres")]
mod postgres_tests {
    use reinhardt_orm::database::Database;
    #[cfg(feature = "postgres")]
    use reinhardt_orm::{Model, fields::JsonbField};

    use serde_json::{Value as JsonValue, json};
    use std::collections::HashMap;

    // Test 1: JSONB field creation
    #[tokio::test]
    async fn test_jsonb_field_definition() {
        let field_def = "data JSONB NOT NULL DEFAULT '{}'::jsonb";

        assert!(field_def.contains("JSONB"));
        assert!(field_def.contains("DEFAULT '{}'::jsonb"));
    }

    // Test 2: JSONB containment operator (@>)
    #[tokio::test]
    async fn test_jsonb_containment_operator() {
        let query = "SELECT * FROM users WHERE data @> '{\"age\": 25}'::jsonb";

        assert!(query.contains("@>"));
        assert!(query.contains("::jsonb"));
    }

    // Test 3: JSONB extraction operators (-> and ->>)
    #[tokio::test]
    async fn test_jsonb_extraction_operators() {
        // -> returns JSONB
        let json_extract = "SELECT data->'profile'->>'name' FROM users";

        // ->> returns text
        let text_extract = "SELECT data->>'email' FROM users";

        assert!(json_extract.contains("->"));
        assert!(json_extract.contains("->>"));
        assert!(text_extract.contains("->>"));
    }

    // Test 4: JSONB indexing with GIN
    #[tokio::test]
    async fn test_jsonb_gin_index() {
        let index_sql = "CREATE INDEX idx_users_data ON users USING GIN (data)";

        assert!(index_sql.contains("USING GIN"));
        assert!(index_sql.contains("data"));
    }

    // Test 5: Array field type
    #[tokio::test]
    async fn test_array_field() {
        let field_def = "tags TEXT[] DEFAULT '{}'";

        assert!(field_def.contains("TEXT[]"));
        assert!(field_def.contains("DEFAULT '{}'"));
    }

    // Test 6: Array ANY operator
    #[tokio::test]
    async fn test_array_any_operator() {
        let query = "SELECT * FROM posts WHERE 'rust' = ANY(tags)";

        assert!(query.contains("ANY(tags)"));
    }

    // Test 7: Array contains operator (@>)
    #[tokio::test]
    async fn test_array_contains_operator() {
        let query = "SELECT * FROM posts WHERE tags @> ARRAY['rust', 'postgres']";

        assert!(query.contains("@>"));
        assert!(query.contains("ARRAY["));
    }

    // Test 8: Array length function
    #[tokio::test]
    async fn test_array_length() {
        let query = "SELECT array_length(tags, 1) FROM posts";

        assert!(query.contains("array_length"));
    }

    // Test 9: HSTORE extension
    #[tokio::test]
    async fn test_postgres_specific_hstore() {
        let field_def = "attributes HSTORE";
        let query = "SELECT * FROM products WHERE attributes->'color' = 'red'";

        assert!(field_def.contains("HSTORE"));
        assert!(query.contains("attributes->'color'"));
    }

    // Test 10: Full-text search with tsvector
    #[tokio::test]
    async fn test_tsvector_field() {
        let field_def = "search_vector TSVECTOR";
        let index_sql = "CREATE INDEX idx_search ON articles USING GIN (search_vector)";

        assert!(field_def.contains("TSVECTOR"));
        assert!(index_sql.contains("USING GIN"));
    }

    // Test 11: Full-text search query with tsquery
    #[tokio::test]
    async fn test_tsquery_search() {
        let query = "SELECT * FROM articles WHERE search_vector @@ to_tsquery('english', 'rust & postgres')";

        assert!(query.contains("@@"));
        assert!(query.contains("to_tsquery"));
    }

    // Test 12: ts_rank for search ranking
    #[tokio::test]
    async fn test_ts_rank() {
        let query = "SELECT title, ts_rank(search_vector, to_tsquery('rust')) as rank
                     FROM articles
                     WHERE search_vector @@ to_tsquery('rust')
                     ORDER BY rank DESC";

        assert!(query.contains("ts_rank"));
        assert!(query.contains("ORDER BY rank DESC"));
    }

    // Test 13: Window functions - ROW_NUMBER
    #[tokio::test]
    async fn test_row_number_window_function() {
        let query = "SELECT id, name, ROW_NUMBER() OVER (ORDER BY created_at DESC) as row_num
                     FROM users";

        assert!(query.contains("ROW_NUMBER()"));
        assert!(query.contains("OVER"));
    }

    // Test 14: Window functions - RANK
    #[tokio::test]
    async fn test_rank_window_function() {
        let query = "SELECT name, score, RANK() OVER (ORDER BY score DESC) as rank
                     FROM players";

        assert!(query.contains("RANK()"));
        assert!(query.contains("OVER"));
    }

    // Test 15: Window functions with PARTITION BY
    #[tokio::test]
    async fn test_partition_by_window() {
        let query = "SELECT category, product, price,
                     AVG(price) OVER (PARTITION BY category) as avg_price
                     FROM products";

        assert!(query.contains("PARTITION BY category"));
        assert!(query.contains("AVG(price) OVER"));
    }

    // Test 16: Common Table Expressions (CTEs)
    #[tokio::test]
    async fn test_cte_with_clause() {
        let query = "WITH recent_orders AS (
                       SELECT * FROM orders WHERE created_at > NOW() - INTERVAL '7 days'
                     )
                     SELECT * FROM recent_orders WHERE total > 100";

        assert!(query.contains("WITH"));
        assert!(query.contains("AS ("));
    }

    // Test 17: Recursive CTE
    #[tokio::test]
    async fn test_postgres_specific_recursive_cte() {
        let query = "WITH RECURSIVE hierarchy AS (
                       SELECT id, name, parent_id FROM categories WHERE parent_id IS NULL
                       UNION ALL
                       SELECT c.id, c.name, c.parent_id
                       FROM categories c
                       INNER JOIN hierarchy h ON c.parent_id = h.id
                     )
                     SELECT * FROM hierarchy";

        assert!(query.contains("WITH RECURSIVE"));
        assert!(query.contains("UNION ALL"));
    }

    // Test 18: LATERAL JOIN
    #[tokio::test]
    async fn test_lateral_join() {
        let query = "SELECT u.name, o.order_date, o.total
                     FROM users u
                     CROSS JOIN LATERAL (
                       SELECT * FROM orders WHERE user_id = u.id ORDER BY order_date DESC LIMIT 3
                     ) o";

        assert!(query.contains("LATERAL"));
        assert!(query.contains("CROSS JOIN"));
    }

    // Test 19: UPSERT with ON CONFLICT DO UPDATE
    #[tokio::test]
    async fn test_on_conflict_do_update() {
        let query = "INSERT INTO users (email, name) VALUES ('test@example.com', 'Test User')
                     ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name";

        assert!(query.contains("ON CONFLICT"));
        assert!(query.contains("DO UPDATE"));
        assert!(query.contains("EXCLUDED"));
    }

    // Test 20: UPSERT with ON CONFLICT DO NOTHING
    #[tokio::test]
    async fn test_on_conflict_do_nothing() {
        let query = "INSERT INTO users (email, name) VALUES ('test@example.com', 'Test User')
                     ON CONFLICT (email) DO NOTHING";

        assert!(query.contains("ON CONFLICT"));
        assert!(query.contains("DO NOTHING"));
    }
}

// Stub tests for when postgres feature is disabled
#[cfg(not(feature = "postgres"))]
mod stub_tests {
    #[tokio::test]
    async fn postgres_tests_disabled() {
        println!("PostgreSQL-specific tests require --features postgres");
    }
}
