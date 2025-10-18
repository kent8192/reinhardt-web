// Aggregation Tests - Inspired by Django aggregation tests
// Tests various aggregation functions: Count, Sum, Avg, Min, Max, etc.

#[cfg(test)]
mod aggregation_tests {
    use reinhardt_orm::aggregation::{Aggregate, Avg, Count, Max, Min, Sum};
    use reinhardt_orm::fields::{CharField, FloatField, IntegerField};
    use reinhardt_orm::query::QuerySet;
    use reinhardt_orm::Model;
    use std::sync::Arc;

    #[derive(Debug, Clone, Model)]
    struct Book {
        #[primary_key(auto = true)]
        id: Option<i32>,

        #[field(max_length = 200)]
        title: String,

        #[field]
        pages: i32,

        #[field]
        price: f64,

        #[field]
        rating: f64,
    }

    #[derive(Debug, Clone, Model)]
    struct Author {
        #[primary_key(auto = true)]
        id: Option<i32>,

        #[field(max_length = 100)]
        name: String,

        #[field]
        age: i32,
    }

    #[tokio::test]
    async fn test_aggregation_empty() {
        // Test aggregation on empty dataset
        let db = create_test_db().await;

        let count = Book::objects(&db).count().await.unwrap();
        assert_eq!(count, 0);

        let sum = Book::objects(&db).aggregate(Sum("pages")).await.unwrap();
        assert_eq!(sum, 0);

        let avg = Book::objects(&db).aggregate(Avg("pages")).await.unwrap();
        assert_eq!(avg, 0.0);

        let min_price = Book::objects(&db).aggregate(Min("price")).await.ok();
        assert_eq!(min_price, None);

        let max_price = Book::objects(&db).aggregate(Max("price")).await.ok();
        assert_eq!(max_price, None);
    }

    #[tokio::test]
    async fn test_aggregation_single() {
        // Test single aggregation function
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 300, 29.99, 4.5).await.unwrap();
        Book::create(&db, "Book 2", 400, 39.99, 4.0).await.unwrap();
        Book::create(&db, "Book 3", 500, 49.99, 4.8).await.unwrap();

        let total_books = Book::objects(&db).count().await.unwrap();
        assert_eq!(total_books, 3);
    }

    #[tokio::test]
    async fn test_aggregation_multiple() {
        // Test multiple aggregation functions simultaneously
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 300, 29.99, 4.5).await.unwrap();
        Book::create(&db, "Book 2", 400, 39.99, 4.0).await.unwrap();
        Book::create(&db, "Book 3", 200, 19.99, 3.5).await.unwrap();

        // Count
        let count = Book::objects(&db).count().await.unwrap();
        assert_eq!(count, 3);

        // Sum
        let total_pages = Book::objects(&db).aggregate(Sum("pages")).await.unwrap();
        assert_eq!(total_pages, 900);

        // Average
        let avg_pages = Book::objects(&db).aggregate(Avg("pages")).await.unwrap();
        assert_eq!(avg_pages, 300.0);

        // Min
        let min_price = Book::objects(&db).aggregate(Min("price")).await.unwrap();
        assert_eq!(min_price, 19.99);

        // Max
        let max_price = Book::objects(&db).aggregate(Max("price")).await.unwrap();
        assert_eq!(max_price, 39.99);
    }

    #[tokio::test]
    async fn test_aggregation_with_filter() {
        // Test aggregation with filtering
        let db = create_test_db().await;

        Book::create(&db, "Short Book", 100, 9.99, 3.0)
            .await
            .unwrap();
        Book::create(&db, "Medium Book", 300, 29.99, 4.0)
            .await
            .unwrap();
        Book::create(&db, "Long Book", 500, 49.99, 5.0)
            .await
            .unwrap();
        Book::create(&db, "Very Long Book", 800, 59.99, 4.5)
            .await
            .unwrap();

        // Filter books with > 200 pages
        let long_books_count = Book::objects(&db)
            .filter(pages__gt = 200)
            .count()
            .await
            .unwrap();
        assert_eq!(long_books_count, 3);

        let avg_pages_long = Book::objects(&db)
            .filter(pages__gt = 200)
            .aggregate(Avg("pages"))
            .await
            .unwrap();

        // Use approximate comparison for floating point
        assert!((avg_pages_long - 533.333_333_333_333_3).abs() < 0.000_001);
    }

    #[tokio::test]
    async fn test_aggregate_in_order_by() {
        // Test using aggregates in ordering
        let db = create_test_db().await;

        Book::create(&db, "Book C", 300, 29.99, 4.0).await.unwrap();
        Book::create(&db, "Book A", 100, 9.99, 5.0).await.unwrap();
        Book::create(&db, "Book B", 200, 19.99, 3.5).await.unwrap();

        // Order by pages
        let books_by_pages = Book::objects(&db).order_by("pages").all().await.unwrap();

        assert_eq!(books_by_pages[0].title, "Book A");
        assert_eq!(books_by_pages[1].title, "Book B");
        assert_eq!(books_by_pages[2].title, "Book C");

        // Order by rating (descending)
        let books_by_rating = Book::objects(&db).order_by("-rating").all().await.unwrap();

        assert_eq!(books_by_rating[0].title, "Book A"); // 5.0
        assert_eq!(books_by_rating[1].title, "Book C"); // 4.0
        assert_eq!(books_by_rating[2].title, "Book B"); // 3.5
    }

    #[tokio::test]
    async fn test_aggregation_avg() {
        // Test average calculation
        let db = create_test_db().await;

        Author::create(&db, "Author 1", 30).await.unwrap();
        Author::create(&db, "Author 2", 40).await.unwrap();
        Author::create(&db, "Author 3", 50).await.unwrap();

        let avg_age = Author::objects(&db).aggregate(Avg("age")).await.unwrap();
        assert_eq!(avg_age, 40.0);
    }

    #[tokio::test]
    async fn test_aggregation_sum() {
        // Test sum calculation
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 100, 10.0, 4.0).await.unwrap();
        Book::create(&db, "Book 2", 200, 20.0, 4.0).await.unwrap();
        Book::create(&db, "Book 3", 300, 30.0, 4.0).await.unwrap();

        let total_pages = Book::objects(&db).aggregate(Sum("pages")).await.unwrap();
        assert_eq!(total_pages, 600);

        let total_price = Book::objects(&db).aggregate(Sum("price")).await.unwrap();
        assert_eq!(total_price, 60.0);
    }

    #[tokio::test]
    async fn test_aggregation_min_max() {
        // Test min and max calculations
        let db = create_test_db().await;

        Book::create(&db, "Cheap Book", 100, 5.99, 3.0)
            .await
            .unwrap();
        Book::create(&db, "Medium Book", 200, 25.99, 4.0)
            .await
            .unwrap();
        Book::create(&db, "Expensive Book", 300, 99.99, 5.0)
            .await
            .unwrap();

        let min_price = Book::objects(&db).aggregate(Min("price")).await.unwrap();
        assert_eq!(min_price, 5.99);

        let max_price = Book::objects(&db).aggregate(Max("price")).await.unwrap();
        assert_eq!(max_price, 99.99);

        // Min/max pages
        let min_pages = Book::objects(&db).aggregate(Min("pages")).await.unwrap();
        let max_pages = Book::objects(&db).aggregate(Max("pages")).await.unwrap();

        assert_eq!(min_pages, 100);
        assert_eq!(max_pages, 300);
    }

    #[tokio::test]
    async fn test_aggregation_count_distinct() {
        // Test counting distinct values
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 100, 10.0, 4.0).await.unwrap();
        Book::create(&db, "Book 2", 100, 15.0, 4.0).await.unwrap();
        Book::create(&db, "Book 3", 200, 20.0, 5.0).await.unwrap();
        Book::create(&db, "Book 4", 200, 25.0, 5.0).await.unwrap();

        // Count distinct page counts
        let distinct_pages = Book::objects(&db)
            .distinct()
            .values("pages")
            .count()
            .await
            .unwrap();
        assert_eq!(distinct_pages, 2);

        // Count distinct ratings
        let distinct_ratings = Book::objects(&db)
            .distinct()
            .values("rating")
            .count()
            .await
            .unwrap();
        assert_eq!(distinct_ratings, 2);
    }

    #[tokio::test]
    async fn test_group_by_aggregate() {
        // Test grouping with aggregation
        let db = create_test_db().await;

        Book::create(&db, "Short 1", 100, 10.0, 4.0).await.unwrap();
        Book::create(&db, "Short 2", 150, 12.0, 4.5).await.unwrap();
        Book::create(&db, "Long 1", 400, 30.0, 3.5).await.unwrap();
        Book::create(&db, "Long 2", 500, 40.0, 4.0).await.unwrap();

        // Group by page range
        let short_count = Book::objects(&db)
            .filter(pages__lt = 200)
            .count()
            .await
            .unwrap();
        assert_eq!(short_count, 2);

        let long_count = Book::objects(&db)
            .filter(pages__gte = 200)
            .count()
            .await
            .unwrap();
        assert_eq!(long_count, 2);

        // Average price per group
        let avg_price_short = Book::objects(&db)
            .filter(pages__lt = 200)
            .aggregate(Avg("price"))
            .await
            .unwrap();
        assert_eq!(avg_price_short, 11.0);

        let avg_price_long = Book::objects(&db)
            .filter(pages__gte = 200)
            .aggregate(Avg("price"))
            .await
            .unwrap();
        assert_eq!(avg_price_long, 35.0);
    }

    #[tokio::test]
    async fn test_conditional_aggregation() {
        // Test conditional aggregation (like CASE WHEN)
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 100, 10.0, 4.5).await.unwrap();
        Book::create(&db, "Book 2", 200, 20.0, 3.0).await.unwrap();
        Book::create(&db, "Book 3", 300, 30.0, 4.8).await.unwrap();
        Book::create(&db, "Book 4", 400, 40.0, 2.5).await.unwrap();

        // Count highly rated books (rating >= 4.0)
        let high_rated_count = Book::objects(&db)
            .filter(rating__gte = 4.0)
            .count()
            .await
            .unwrap();
        assert_eq!(high_rated_count, 2);

        // Average price of highly rated books
        let avg_price_high_rated = Book::objects(&db)
            .filter(rating__gte = 4.0)
            .aggregate(Avg("price"))
            .await
            .unwrap();
        assert_eq!(avg_price_high_rated, 20.0);
    }

    #[tokio::test]
    async fn test_aggregate_with_null_values() {
        // Test aggregation handling of missing/null values
        #[derive(Debug, Clone, Model)]
        struct Product {
            #[primary_key(auto = true)]
            id: Option<i32>,

            #[field(max_length = 100)]
            name: String,

            #[field(nullable = true)]
            price: Option<f64>,
        }

        let db = create_test_db().await;

        Product::create(&db, "Product 1", Some(10.0)).await.unwrap();
        Product::create(&db, "Product 2", None).await.unwrap();
        Product::create(&db, "Product 3", Some(30.0)).await.unwrap();

        // Count all products
        let all_count = Product::objects(&db).count().await.unwrap();
        assert_eq!(all_count, 3);

        // Count products with price (non-null)
        let with_price_count = Product::objects(&db)
            .filter(price__isnull = false)
            .count()
            .await
            .unwrap();
        assert_eq!(with_price_count, 2);

        // Average of non-null prices
        let avg_price = Product::objects(&db)
            .filter(price__isnull = false)
            .aggregate(Avg("price"))
            .await
            .unwrap();
        assert_eq!(avg_price, 20.0);
    }

    #[tokio::test]
    async fn test_aggregation_with_annotation() {
        // Test annotating query results with aggregates
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 100, 10.0, 4.0).await.unwrap();
        Book::create(&db, "Book 2", 200, 20.0, 4.5).await.unwrap();
        Book::create(&db, "Book 3", 300, 30.0, 5.0).await.unwrap();

        // Annotate with price-per-page ratio
        let annotated = Book::objects(&db)
            .annotate(price_per_page = F("price") / F("pages"))
            .all()
            .await
            .unwrap();

        assert_eq!(annotated.len(), 3);
        assert_eq!(annotated[0].price_per_page, 0.1); // 10.0 / 100
        assert_eq!(annotated[1].price_per_page, 0.1); // 20.0 / 200
        assert_eq!(annotated[2].price_per_page, 0.1); // 30.0 / 300
    }

    #[tokio::test]
    async fn test_aggregate_having() {
        // Test HAVING-like filtering on aggregated results
        let db = create_test_db().await;

        Book::create(&db, "Cheap 1", 100, 5.0, 3.0).await.unwrap();
        Book::create(&db, "Cheap 2", 150, 8.0, 3.5).await.unwrap();
        Book::create(&db, "Expensive 1", 400, 50.0, 4.5)
            .await
            .unwrap();
        Book::create(&db, "Expensive 2", 500, 60.0, 5.0)
            .await
            .unwrap();

        // Group by price category
        let cheap_count = Book::objects(&db)
            .filter(price__lt = 20.0)
            .count()
            .await
            .unwrap();
        assert!(cheap_count > 1);

        let expensive_count = Book::objects(&db)
            .filter(price__gte = 20.0)
            .count()
            .await
            .unwrap();
        assert!(expensive_count > 1);

        // Average pages for each group
        let avg_pages_cheap = Book::objects(&db)
            .filter(price__lt = 20.0)
            .aggregate(Avg("pages"))
            .await
            .unwrap();
        assert_eq!(avg_pages_cheap, 125.0);

        let avg_pages_expensive = Book::objects(&db)
            .filter(price__gte = 20.0)
            .aggregate(Avg("pages"))
            .await
            .unwrap();
        assert_eq!(avg_pages_expensive, 450.0);
    }

    #[tokio::test]
    async fn test_window_aggregate() {
        // Test window function-like behavior
        let db = create_test_db().await;

        Book::create(&db, "Book 1", 100, 10.0, 4.0).await.unwrap();
        Book::create(&db, "Book 2", 200, 20.0, 4.5).await.unwrap();
        Book::create(&db, "Book 3", 300, 30.0, 5.0).await.unwrap();

        // Calculate running total of pages using window function
        let with_running_total = Book::objects(&db)
            .annotate(
                running_total = Window(
                    Sum("pages"),
                    order_by = "id",
                    frame = "ROWS UNBOUNDED PRECEDING",
                ),
            )
            .order_by("id")
            .all()
            .await
            .unwrap();

        assert_eq!(with_running_total[0].running_total, 100);
        assert_eq!(with_running_total[1].running_total, 300);
        assert_eq!(with_running_total[2].running_total, 600);
    }

    // Helper function to create a test database
    async fn create_test_db() -> Arc<Database> {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        // Create tables
        Book::create_table(&db).await.unwrap();
        Author::create_table(&db).await.unwrap();

        Arc::new(db)
    }

    // Helper implementations for model creation
    impl Book {
        async fn create(
            db: &Database,
            title: &str,
            pages: i32,
            price: f64,
            rating: f64,
        ) -> Result<Self, Error> {
            let book = Self {
                id: None,
                title: title.to_string(),
                pages,
                price,
                rating,
            };
            book.save(db).await
        }
    }

    impl Author {
        async fn create(db: &Database, name: &str, age: i32) -> Result<Self, Error> {
            let author = Self {
                id: None,
                name: name.to_string(),
                age,
            };
            author.save(db).await
        }
    }
}
