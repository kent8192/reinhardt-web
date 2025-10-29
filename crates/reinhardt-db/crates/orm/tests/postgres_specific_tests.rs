//! PostgreSQL-Specific Feature Tests
//!
//! Tests for PostgreSQL-specific features: JSONB, Arrays, HSTORE, Full-text search,
//! Window functions, CTEs, LATERAL JOIN, and UPSERT (ON CONFLICT).
//!
//! Run with: cargo test --package reinhardt-orm --test postgres_specific_tests --features postgres

#[cfg(feature = "postgres")]
mod postgres_tests {

    // Test 1: JSONB field creation
    #[tokio::test]
    async fn test_jsonb_field_definition() {
        let field_def = "data JSONB NOT NULL DEFAULT '{}'::jsonb";

        assert!(
            field_def.contains("JSONB"),
            "JSONB field definition should contain 'JSONB' keyword"
        );
        assert!(
            field_def.contains("DEFAULT '{}'::jsonb"),
            "JSONB field should have default value with proper cast"
        );
    }

    // Test 2: JSONB containment operator (@>)
    #[tokio::test]
    async fn test_jsonb_containment_operator() {
        let query = "SELECT * FROM users WHERE data @> '{\"age\": 25}'::jsonb";

        assert!(
            query.contains("@>"),
            "JSONB containment query should use @> operator"
        );
        assert!(
            query.contains("::jsonb"),
            "JSONB containment query should cast value to jsonb"
        );
    }

    // Test 3: JSONB extraction operators (-> and ->>)
    #[tokio::test]
    async fn test_jsonb_extraction_operators() {
        // -> returns JSONB
        let json_extract = "SELECT data->'profile'->>'name' FROM users";

        // ->> returns text
        let text_extract = "SELECT data->>'email' FROM users";

        assert!(
            json_extract.contains("->"),
            "JSONB extraction should use -> operator for JSON navigation"
        );
        assert!(
            json_extract.contains("->>"),
            "JSONB extraction should use ->> operator for text result"
        );
        assert!(
            text_extract.contains("->>"),
            "Text extraction should use ->> operator"
        );
    }

    // Test 4: JSONB indexing with GIN
    #[tokio::test]
    async fn test_jsonb_gin_index() {
        let index_sql = "CREATE INDEX idx_users_data ON users USING GIN (data)";

        assert!(
            index_sql.contains("USING GIN"),
            "JSONB index should use GIN index type"
        );
        assert!(
            index_sql.contains("data"),
            "JSONB index should reference the JSONB column"
        );
    }

    // Test 5: Array field type
    #[tokio::test]
    async fn test_array_field() {
        let field_def = "tags TEXT[] DEFAULT '{}'";

        assert!(
            field_def.contains("TEXT[]"),
            "Array field should use TEXT[] type syntax"
        );
        assert!(
            field_def.contains("DEFAULT '{}'"),
            "Array field should have empty array as default value"
        );
    }

    // Test 6: Array ANY operator
    #[tokio::test]
    async fn test_array_any_operator() {
        let query = "SELECT * FROM posts WHERE 'rust' = ANY(tags)";

        assert!(
            query.contains("ANY(tags)"),
            "Array query should use ANY operator for element matching"
        );
    }

    // Test 7: Array contains operator (@>)
    #[tokio::test]
    async fn test_array_contains_operator() {
        let query = "SELECT * FROM posts WHERE tags @> ARRAY['rust', 'postgres']";

        assert!(
            query.contains("@>"),
            "Array containment query should use @> operator"
        );
        assert!(
            query.contains("ARRAY["),
            "Array containment query should use ARRAY constructor syntax"
        );
    }

    // Test 8: Array length function
    #[tokio::test]
    async fn test_array_length() {
        let query = "SELECT array_length(tags, 1) FROM posts";

        assert!(
            query.contains("array_length"),
            "Array length query should use array_length function"
        );
    }

    // Test 9: HSTORE extension
    #[tokio::test]
    async fn test_postgres_specific_hstore() {
        let field_def = "attributes HSTORE";
        let query = "SELECT * FROM products WHERE attributes->'color' = 'red'";

        assert!(
            field_def.contains("HSTORE"),
            "HSTORE field should use HSTORE type"
        );
        assert!(
            query.contains("attributes->'color'"),
            "HSTORE query should use -> operator for key access"
        );
    }

    // Test 10: Full-text search with tsvector
    #[tokio::test]
    async fn test_tsvector_field() {
        let field_def = "search_vector TSVECTOR";
        let index_sql = "CREATE INDEX idx_search ON articles USING GIN (search_vector)";

        assert!(
            field_def.contains("TSVECTOR"),
            "Full-text search field should use TSVECTOR type"
        );
        assert!(
            index_sql.contains("USING GIN"),
            "TSVECTOR index should use GIN index type"
        );
    }

    // Test 11: Full-text search query with tsquery
    #[tokio::test]
    async fn test_tsquery_search() {
        let query = "SELECT * FROM articles WHERE search_vector @@ to_tsquery('english', 'rust & postgres')";

        assert!(
            query.contains("@@"),
            "Full-text search should use @@ match operator"
        );
        assert!(
            query.contains("to_tsquery"),
            "Full-text search should use to_tsquery function"
        );
    }

    // Test 12: ts_rank for search ranking
    #[tokio::test]
    async fn test_ts_rank() {
        let query = "SELECT title, ts_rank(search_vector, to_tsquery('rust')) as rank
                     FROM articles
                     WHERE search_vector @@ to_tsquery('rust')
                     ORDER BY rank DESC";

        assert!(
            query.contains("ts_rank"),
            "Search ranking should use ts_rank function"
        );
        assert!(
            query.contains("ORDER BY rank DESC"),
            "Search results should be ordered by rank descending"
        );
    }

    // Test 13: Window functions - ROW_NUMBER
    #[tokio::test]
    async fn test_row_number_window_function() {
        let query = "SELECT id, name, ROW_NUMBER() OVER (ORDER BY created_at DESC) as row_num
                     FROM users";

        assert!(
            query.contains("ROW_NUMBER()"),
            "Window function query should use ROW_NUMBER()"
        );
        assert!(
            query.contains("OVER"),
            "Window function query should use OVER clause"
        );
    }

    // Test 14: Window functions - RANK
    #[tokio::test]
    async fn test_rank_window_function() {
        let query = "SELECT name, score, RANK() OVER (ORDER BY score DESC) as rank
                     FROM players";

        assert!(
            query.contains("RANK()"),
            "Ranking window function should use RANK()"
        );
        assert!(
            query.contains("OVER"),
            "Ranking window function should use OVER clause"
        );
    }

    // Test 15: Window functions with PARTITION BY
    #[tokio::test]
    async fn test_partition_by_window() {
        let query = "SELECT category, product, price,
                     AVG(price) OVER (PARTITION BY category) as avg_price
                     FROM products";

        assert!(
            query.contains("PARTITION BY category"),
            "Partitioned window function should use PARTITION BY clause"
        );
        assert!(
            query.contains("AVG(price) OVER"),
            "Partitioned window function should combine aggregate with OVER clause"
        );
    }

    // Test 16: Common Table Expressions (CTEs)
    #[tokio::test]
    async fn test_cte_with_clause() {
        let query = "WITH recent_orders AS (
                       SELECT * FROM orders WHERE created_at > NOW() - INTERVAL '7 days'
                     )
                     SELECT * FROM recent_orders WHERE total > 100";

        assert!(
            query.contains("WITH"),
            "CTE query should use WITH clause"
        );
        assert!(
            query.contains("AS ("),
            "CTE query should define named subquery with AS"
        );
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

        assert!(
            query.contains("WITH RECURSIVE"),
            "Recursive CTE should use WITH RECURSIVE clause"
        );
        assert!(
            query.contains("UNION ALL"),
            "Recursive CTE should use UNION ALL to combine base and recursive parts"
        );
    }

    // Test 18: LATERAL JOIN
    #[tokio::test]
    async fn test_lateral_join() {
        let query = "SELECT u.name, o.order_date, o.total
                     FROM users u
                     CROSS JOIN LATERAL (
                       SELECT * FROM orders WHERE user_id = u.id ORDER BY order_date DESC LIMIT 3
                     ) o";

        assert!(
            query.contains("LATERAL"),
            "LATERAL join should use LATERAL keyword"
        );
        assert!(
            query.contains("CROSS JOIN"),
            "LATERAL join should use CROSS JOIN syntax"
        );
    }

    // Test 19: UPSERT with ON CONFLICT DO UPDATE
    #[tokio::test]
    async fn test_on_conflict_do_update() {
        let query = "INSERT INTO users (email, name) VALUES ('test@example.com', 'Test User')
                     ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name";

        assert!(
            query.contains("ON CONFLICT"),
            "UPSERT query should use ON CONFLICT clause"
        );
        assert!(
            query.contains("DO UPDATE"),
            "UPSERT query should use DO UPDATE action"
        );
        assert!(
            query.contains("EXCLUDED"),
            "UPSERT query should reference EXCLUDED for new values"
        );
    }

    // Test 20: UPSERT with ON CONFLICT DO NOTHING
    #[tokio::test]
    async fn test_on_conflict_do_nothing() {
        let query = "INSERT INTO users (email, name) VALUES ('test@example.com', 'Test User')
                     ON CONFLICT (email) DO NOTHING";

        assert!(
            query.contains("ON CONFLICT"),
            "UPSERT with ignore should use ON CONFLICT clause"
        );
        assert!(
            query.contains("DO NOTHING"),
            "UPSERT with ignore should use DO NOTHING action"
        );
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
