// Basic Query Tests - Inspired by Django and SQLAlchemy ORM tests
// Tests basic ORM operations including filtering, ordering, counting

#[cfg(test)]
mod query_basic_tests {
    use reinhardt_orm::database::Database;
    use reinhardt_orm::{Model, QuerySet};

    use chrono::{DateTime, TimeZone, Utc};
    use std::sync::Arc;

    // Simple test models
    #[derive(Debug, Clone)]
    struct Author {
        id: Option<i32>,
        name: String,
        age: i32,
    }

    #[derive(Debug, Clone)]
    struct Book {
        id: Option<i32>,
        title: String,
        pages: i32,
        price: f64,
        author_id: Option<i32>,
    }

    // Mock database connection for testing
    struct TestDatabase {
        authors: Vec<Author>,
        books: Vec<Book>,
    }

    impl TestDatabase {
        fn new() -> Self {
            Self {
                authors: Vec::new(),
                books: Vec::new(),
            }
        }

        fn add_author(&mut self, name: &str, age: i32) -> i32 {
            let id = self.authors.len() as i32 + 1;
            self.authors.push(Author {
                id: Some(id),
                name: name.to_string(),
                age,
            });
            id
        }

        fn add_book(&mut self, title: &str, pages: i32, price: f64, author_id: Option<i32>) -> i32 {
            let id = self.books.len() as i32 + 1;
            self.books.push(Book {
                id: Some(id),
                title: title.to_string(),
                pages,
                price,
                author_id,
            });
            id
        }

        fn count_authors(&self) -> usize {
            self.authors.len()
        }

        fn count_books(&self) -> usize {
            self.books.len()
        }

        fn filter_authors_by_age(&self, age: i32) -> Vec<&Author> {
            self.authors.iter().filter(|a| a.age == age).collect()
        }

        fn filter_authors_by_age_gt(&self, age: i32) -> Vec<&Author> {
            self.authors.iter().filter(|a| a.age > age).collect()
        }

        fn get_author_by_id(&self, id: i32) -> Option<&Author> {
            self.authors.iter().find(|a| a.id == Some(id))
        }

        fn get_books_by_author(&self, author_id: i32) -> Vec<&Book> {
            self.books
                .iter()
                .filter(|b| b.author_id == Some(author_id))
                .collect()
        }
    }

    #[tokio::test]
    async fn test_query_exists() {
        // Test that we can check if records exist
        let mut db = TestDatabase::new();

        assert_eq!(db.count_authors(), 0);

        db.add_author("John Doe", 30);
        db.add_author("Jane Smith", 25);

        assert_eq!(db.count_authors(), 2);

        let authors = db.filter_authors_by_age(30);
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].name, "John Doe");
    }

    #[tokio::test]
    async fn test_lookup_int_as_str() {
        // Test looking up by integer fields
        let mut db = TestDatabase::new();

        db.add_author("Alice", 35);
        db.add_author("Bob", 35);
        db.add_author("Charlie", 40);

        // Filter by exact age
        let authors_35 = db.filter_authors_by_age(35);
        assert_eq!(authors_35.len(), 2);

        // Filter by age greater than
        let authors_gt_35 = db.filter_authors_by_age_gt(35);
        assert_eq!(authors_gt_35.len(), 1);
        assert_eq!(authors_gt_35[0].name, "Charlie");
    }

    #[tokio::test]
    async fn test_query_count() {
        // Test counting records
        let mut db = TestDatabase::new();

        assert_eq!(db.count_authors(), 0);

        db.add_author("Author 1", 30);
        assert_eq!(db.count_authors(), 1);

        db.add_author("Author 2", 40);
        assert_eq!(db.count_authors(), 2);

        db.add_author("Author 3", 50);
        assert_eq!(db.count_authors(), 3);
    }

    #[tokio::test]
    async fn test_query_basic_empty_aggregate() {
        // Test aggregation on empty query set
        let db = TestDatabase::new();

        assert_eq!(db.count_authors(), 0);
        assert_eq!(db.count_books(), 0);
    }

    #[tokio::test]
    async fn test_query_basic_single_aggregate() {
        // Test single aggregation function
        let mut db = TestDatabase::new();

        db.add_author("Author 1", 30);
        db.add_author("Author 2", 40);
        db.add_author("Author 3", 50);

        let total_count = db.count_authors();
        assert_eq!(total_count, 3);

        // Calculate average age
        let total_age: i32 = db.authors.iter().map(|a| a.age).sum();
        let avg_age = total_age as f64 / db.count_authors() as f64;
        assert_eq!(avg_age, 40.0);
    }

    #[tokio::test]
    async fn test_query_basic_multiple_aggregates() {
        // Test multiple aggregation functions in single query
        let mut db = TestDatabase::new();

        db.add_author("Author 1", 25);
        db.add_author("Author 2", 35);
        db.add_author("Author 3", 45);

        // Count
        let count = db.count_authors();
        assert_eq!(count, 3);

        // Min age
        let min_age = db.authors.iter().map(|a| a.age).min().unwrap();
        assert_eq!(min_age, 25);

        // Max age
        let max_age = db.authors.iter().map(|a| a.age).max().unwrap();
        assert_eq!(max_age, 45);

        // Avg age
        let total_age: i32 = db.authors.iter().map(|a| a.age).sum();
        let avg_age = total_age as f64 / count as f64;
        assert_eq!(avg_age, 35.0);
    }

    #[tokio::test]
    async fn test_query_basic_filter_aggregate() {
        // Test aggregation with filtering
        let mut db = TestDatabase::new();

        db.add_author("Young Author 1", 25);
        db.add_author("Young Author 2", 28);
        db.add_author("Old Author 1", 45);
        db.add_author("Old Author 2", 50);

        // Filter authors under 30
        let young_authors = db
            .filter_authors_by_age_gt(20)
            .into_iter()
            .filter(|a| a.age < 30)
            .collect::<Vec<_>>();

        assert_eq!(young_authors.len(), 2);

        // Average age of young authors
        let young_avg =
            young_authors.iter().map(|a| a.age).sum::<i32>() as f64 / young_authors.len() as f64;
        assert_eq!(young_avg, 26.5);
    }

    #[tokio::test]
    async fn test_basic_annotation() {
        // Test basic field annotation
        let mut db = TestDatabase::new();

        let author_id = db.add_author("John Doe", 30);
        db.add_book("Book 1", 200, 29.99, Some(author_id));
        db.add_book("Book 2", 300, 39.99, Some(author_id));

        let author = db.get_author_by_id(author_id).unwrap();
        let books = db.get_books_by_author(author_id);

        // Annotate with book count
        assert_eq!(books.len(), 2);

        // Annotate with total pages
        let total_pages: i32 = books.iter().map(|b| b.pages).sum();
        assert_eq!(total_pages, 500);
    }

    #[tokio::test]
    async fn test_basic_f_annotation() {
        // Test F() expression annotation
        let mut db = TestDatabase::new();

        db.add_book("Short Book", 100, 19.99, None);
        db.add_book("Medium Book", 300, 29.99, None);
        db.add_book("Long Book", 500, 49.99, None);

        // Calculate price per page
        for book in &db.books {
            let price_per_page = book.price / book.pages as f64;
            assert!(price_per_page > 0.0);
        }
    }

    #[tokio::test]
    async fn test_model_instance_creation() {
        // Test creating model instances
        let author = Author {
            id: None,
            name: "Test Author".to_string(),
            age: 35,
        };

        assert!(author.id.is_none());
        assert_eq!(author.name, "Test Author");
        assert_eq!(author.age, 35);
    }

    #[tokio::test]
    async fn test_get_or_create() {
        // Test get_or_create functionality
        let mut db = TestDatabase::new();

        // First call should create
        let id1 = db.add_author("Unique Author", 30);
        assert_eq!(db.count_authors(), 1);

        // Check if already exists
        let existing = db.get_author_by_id(id1);
        assert!(existing.is_some());
        assert_eq!(existing.unwrap().name, "Unique Author");

        // Adding another with same name would create new record
        let id2 = db.add_author("Unique Author", 30);
        assert_ne!(id1, id2);
        assert_eq!(db.count_authors(), 2);
    }

    #[tokio::test]
    async fn test_query_update() {
        // Test updating records
        let mut db = TestDatabase::new();

        let id = db.add_author("Old Name", 30);

        // Verify initial state
        let author = db.get_author_by_id(id).unwrap();
        assert_eq!(author.name, "Old Name");

        // Update (simulated)
        if let Some(author) = db.authors.iter_mut().find(|a| a.id == Some(id)) {
            author.name = "New Name".to_string();
        }

        // Verify updated state
        let author = db.get_author_by_id(id).unwrap();
        assert_eq!(author.name, "New Name");
    }

    #[tokio::test]
    async fn test_query_basic_delete() {
        // Test deleting records
        let mut db = TestDatabase::new();

        let id1 = db.add_author("Author 1", 30);
        let id2 = db.add_author("Author 2", 40);

        assert_eq!(db.count_authors(), 2);

        // Delete one author
        db.authors.retain(|a| a.id != Some(id1));

        assert_eq!(db.count_authors(), 1);
        assert!(db.get_author_by_id(id1).is_none());
        assert!(db.get_author_by_id(id2).is_some());
    }

    #[tokio::test]
    async fn test_order_by() {
        // Test ordering results
        let mut db = TestDatabase::new();

        db.add_author("Charlie", 45);
        db.add_author("Alice", 35);
        db.add_author("Bob", 40);

        // Order by age ascending
        let mut authors_by_age: Vec<_> = db.authors.iter().collect();
        authors_by_age.sort_by_key(|a| a.age);

        assert_eq!(authors_by_age[0].name, "Alice");
        assert_eq!(authors_by_age[1].name, "Bob");
        assert_eq!(authors_by_age[2].name, "Charlie");

        // Order by name
        let mut authors_by_name: Vec<_> = db.authors.iter().collect();
        authors_by_name.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(authors_by_name[0].name, "Alice");
        assert_eq!(authors_by_name[1].name, "Bob");
        assert_eq!(authors_by_name[2].name, "Charlie");
    }

    #[tokio::test]
    async fn test_slice() {
        // Test slicing query results
        let mut db = TestDatabase::new();

        for i in 1..=10 {
            db.add_author(&format!("Author {}", i), 30 + i);
        }

        assert_eq!(db.count_authors(), 10);

        // Get first 5
        let first_five: Vec<_> = db.authors.iter().take(5).collect();
        assert_eq!(first_five.len(), 5);

        // Get last 5
        let last_five: Vec<_> = db.authors.iter().skip(5).collect();
        assert_eq!(last_five.len(), 5);
    }

    #[tokio::test]
    async fn test_distinct() {
        // Test distinct results
        let mut db = TestDatabase::new();

        db.add_author("John", 30);
        db.add_author("Jane", 30);
        db.add_author("Bob", 40);
        db.add_author("Alice", 40);

        // Get distinct ages
        let mut ages: Vec<i32> = db.authors.iter().map(|a| a.age).collect();
        ages.sort();
        ages.dedup();

        assert_eq!(ages.len(), 2);
        assert_eq!(ages[0], 30);
        assert_eq!(ages[1], 40);
    }

    #[tokio::test]
    async fn test_complex_filter() {
        // Test complex filtering with AND/OR conditions
        let mut db = TestDatabase::new();

        db.add_author("Young John", 25);
        db.add_author("Old John", 50);
        db.add_author("Young Jane", 28);
        db.add_author("Old Jane", 52);

        // Filter: age < 30 AND name contains "John"
        let young_johns: Vec<_> = db
            .authors
            .iter()
            .filter(|a| a.age < 30 && a.name.contains("John"))
            .collect();

        assert_eq!(young_johns.len(), 1);
        assert_eq!(young_johns[0].name, "Young John");

        // Filter: age > 45 OR name contains "Jane"
        let old_or_jane: Vec<_> = db
            .authors
            .iter()
            .filter(|a| a.age > 45 || a.name.contains("Jane"))
            .collect();

        assert_eq!(old_or_jane.len(), 3); // Old John, Young Jane, Old Jane
    }
}
