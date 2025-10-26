//! Loading Strategy Tests
//!
//! Tests based on SQLAlchemy's relationship loading strategies and Django's select_related/prefetch_related.
//! Validates joinedload, selectinload, subqueryload, lazyload, raiseload, noload strategies.

use reinhardt_orm::{
    LoadContext, LoadOptionBuilder, LoadingStrategy, Model, joinedload, lazyload, noload,
    raiseload, selectinload, subqueryload,
};
use reinhardt_validators::TableName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Mock models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Author {
    id: Option<i32>,
    name: String,
}

const AUTHOR_TABLE: TableName = TableName::new_const("author");

impl Model for Author {
    type PrimaryKey = i32;
    fn table_name() -> &'static str {
        AUTHOR_TABLE.as_str()
    }
    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }
    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

#[derive(Debug, Clone)]
struct Book {
    id: i32,
    title: String,
    author_id: i32,
}

#[derive(Debug, Clone)]
struct Review {
    id: i32,
    content: String,
    book_id: i32,
}

// Mock database with loading strategy tracking
struct TestDatabase {
    authors: Vec<Author>,
    books: Vec<Book>,
    reviews: Vec<Review>,
    query_count: usize,
    join_hints: Vec<String>,
}

impl TestDatabase {
    fn new() -> Self {
        Self {
            authors: vec![
                Author {
                    id: Some(1),
                    name: "Author 1".to_string(),
                },
                Author {
                    id: Some(2),
                    name: "Author 2".to_string(),
                },
            ],
            books: vec![
                Book {
                    id: 1,
                    title: "Book 1".to_string(),
                    author_id: 1,
                },
                Book {
                    id: 2,
                    title: "Book 2".to_string(),
                    author_id: 1,
                },
                Book {
                    id: 3,
                    title: "Book 3".to_string(),
                    author_id: 2,
                },
            ],
            reviews: vec![
                Review {
                    id: 1,
                    content: "Great!".to_string(),
                    book_id: 1,
                },
                Review {
                    id: 2,
                    content: "Good!".to_string(),
                    book_id: 1,
                },
                Review {
                    id: 3,
                    content: "Nice!".to_string(),
                    book_id: 2,
                },
            ],
            query_count: 0,
            join_hints: Vec::new(),
        }
    }

    fn get_author(&mut self, id: i32) -> Option<Author> {
        self.query_count += 1;
        self.authors.iter().find(|a| a.id == Some(id)).cloned()
    }

    fn get_books_by_author(&mut self, author_id: i32) -> Vec<Book> {
        self.query_count += 1;
        self.books
            .iter()
            .filter(|b| b.author_id == author_id)
            .cloned()
            .collect()
    }

    fn get_reviews_by_book(&mut self, book_id: i32) -> Vec<Review> {
        self.query_count += 1;
        self.reviews
            .iter()
            .filter(|r| r.book_id == book_id)
            .cloned()
            .collect()
    }

    fn get_authors_with_joined_books(&mut self) -> Vec<(Author, Vec<Book>)> {
        self.query_count += 1;
        self.join_hints.push("INNER JOIN books".to_string());
        self.authors
            .iter()
            .map(|a| {
                let author_id = a.id.unwrap_or(0);
                let books = self
                    .books
                    .iter()
                    .filter(|b| b.author_id == author_id)
                    .cloned()
                    .collect();
                (a.clone(), books)
            })
            .collect()
    }

    fn get_books_with_selectin_reviews(&mut self, book_ids: &[i32]) -> HashMap<i32, Vec<Review>> {
        self.query_count += 1;
        self.join_hints
            .push(format!("SELECT WHERE book_id IN ({:?})", book_ids));
        let mut result = HashMap::new();
        for &book_id in book_ids {
            let reviews = self
                .reviews
                .iter()
                .filter(|r| r.book_id == book_id)
                .cloned()
                .collect();
            result.insert(book_id, reviews);
        }
        result
    }

    fn reset_counters(&mut self) {
        self.query_count = 0;
        self.join_hints.clear();
    }
}

// Test 1: Basic joinedload strategy
#[test]
fn test_joinedload_basic() {
    let mut db = TestDatabase::new();
    let option = joinedload("books");

    assert_eq!(option.strategy(), LoadingStrategy::Joined);
    assert_eq!(option.path(), "books");

    // Simulate joinedload: single query with JOIN
    let _results = db.get_authors_with_joined_books();
    assert_eq!(db.query_count, 1, "Joinedload should use single query");
    assert!(
        db.join_hints.iter().any(|h| h.contains("JOIN")),
        "Should contain JOIN hint"
    );
}

// Test 2: Basic selectinload strategy
#[test]
fn test_selectinload_basic() {
    let mut db = TestDatabase::new();
    let option = selectinload("reviews");

    assert_eq!(option.strategy(), LoadingStrategy::Selectin);

    // Simulate selectinload: parent query + one SELECT IN query
    let books = vec![1, 2, 3];
    let _reviews = db.get_books_with_selectin_reviews(&books);
    assert_eq!(
        db.query_count, 1,
        "Selectinload should use single SELECT IN query"
    );
    assert!(
        db.join_hints
            .iter()
            .any(|h| h.contains("SELECT") && h.contains("IN")),
        "Should contain SELECT IN hint"
    );
}

// Test 3: Lazy loading causes N+1 queries
#[test]
fn test_lazyload_n_plus_one() {
    let mut db = TestDatabase::new();
    let option = lazyload("books");

    assert_eq!(option.strategy(), LoadingStrategy::Lazy);
    assert!(option.strategy().is_lazy());

    // Simulate N+1 problem: 1 query for authors + N queries for books
    let author_ids: Vec<i32> = db.authors.iter().filter_map(|a| a.id).collect();
    db.reset_counters();

    for author_id in author_ids {
        let _books = db.get_books_by_author(author_id);
    }

    assert_eq!(
        db.query_count, 2,
        "Lazy load should create N queries (one per author)"
    );
}

// Test 4: Raiseload prevents loading
#[test]
fn test_raiseload_prevents_access() {
    let option = raiseload("profile");

    assert_eq!(option.strategy(), LoadingStrategy::Raise);
    assert!(option.strategy().prevents_load());

    // In real implementation, accessing relationship would raise error
    // Here we just verify the strategy properties
    assert!(
        !option.strategy().is_eager(),
        "Raiseload is not eager loading"
    );
    assert!(
        !option.strategy().is_lazy(),
        "Raiseload is not lazy loading"
    );
}

// Test 5: Noload strategy
#[test]
fn test_noload_strategy() {
    let option = noload("comments");

    assert_eq!(option.strategy(), LoadingStrategy::NoLoad);
    assert!(option.strategy().prevents_load());

    // NoLoad means relationship is never loaded, even on access
}

// Test 6: Subqueryload strategy
#[test]
fn test_subqueryload_strategy() {
    let option = subqueryload("tags");

    assert_eq!(option.strategy(), LoadingStrategy::Subquery);
    assert!(option.strategy().is_eager());
    assert!(option.strategy().sql_hint().is_some());
}

// Test 7: LoadOption path parsing
#[test]
fn test_load_option_path_parsing() {
    let option = joinedload("author.books.reviews");

    assert_eq!(option.path(), "author.books.reviews");
    let components = option.path_components();
    assert_eq!(components, vec!["author", "books", "reviews"]);
}

// Test 8: LoadOptionBuilder multiple options
#[test]
fn test_load_option_builder_multiple() {
    let options = LoadOptionBuilder::<Author>::new()
        .joinedload("books")
        .selectinload("books.reviews")
        .raiseload("profile")
        .build();

    assert_eq!(options.len(), 3);
    assert_eq!(options[0].path(), "books");
    assert_eq!(options[0].strategy(), LoadingStrategy::Joined);
    assert_eq!(options[1].path(), "books.reviews");
    assert_eq!(options[1].strategy(), LoadingStrategy::Selectin);
    assert_eq!(options[2].path(), "profile");
    assert_eq!(options[2].strategy(), LoadingStrategy::Raise);
}

// Test 9: LoadContext tracks loaded paths
#[test]
fn test_load_context_tracking() {
    let mut ctx = LoadContext::new();

    ctx.mark_loaded("books".to_string(), LoadingStrategy::Joined);
    ctx.mark_loaded("reviews".to_string(), LoadingStrategy::Selectin);

    assert!(ctx.is_loaded("books"));
    assert!(ctx.is_loaded("reviews"));
    assert!(!ctx.is_loaded("tags"));

    assert_eq!(ctx.strategy_for("books"), Some(LoadingStrategy::Joined));
    assert_eq!(ctx.strategy_for("reviews"), Some(LoadingStrategy::Selectin));
    assert_eq!(ctx.strategy_for("tags"), None);
}

// Test 10: Eager loading strategies
#[test]
fn test_eager_loading_strategies() {
    assert!(LoadingStrategy::Joined.is_eager());
    assert!(LoadingStrategy::Selectin.is_eager());
    assert!(LoadingStrategy::Subquery.is_eager());
    assert!(!LoadingStrategy::Lazy.is_eager());
    assert!(!LoadingStrategy::Raise.is_eager());
}

// Test 11: SQL hints for query planner
#[test]
fn test_loading_strategy_sql_hints() {
    assert_eq!(
        LoadingStrategy::Joined.sql_hint(),
        Some("/* +JOINEDLOAD */")
    );
    assert_eq!(
        LoadingStrategy::Selectin.sql_hint(),
        Some("/* +SELECTINLOAD */")
    );
    assert_eq!(
        LoadingStrategy::Subquery.sql_hint(),
        Some("/* +SUBQUERYLOAD */")
    );
    assert_eq!(LoadingStrategy::Lazy.sql_hint(), None);
    assert_eq!(LoadingStrategy::Raise.sql_hint(), None);
}

// Test 12: Joinedload avoids N+1 with single relationship
#[test]
fn test_joinedload_avoids_n_plus_one() {
    let mut db = TestDatabase::new();

    // Without joinedload: N+1 queries
    db.reset_counters();
    let author_ids: Vec<i32> = db.authors.iter().filter_map(|a| a.id).collect();
    for author_id in author_ids {
        let _books = db.get_books_by_author(author_id);
    }
    let n_plus_one_count = db.query_count;

    // With joinedload: single query
    db.reset_counters();
    let _results = db.get_authors_with_joined_books();
    let joined_count = db.query_count;

    assert!(
        joined_count < n_plus_one_count,
        "Joinedload should use fewer queries than lazy loading"
    );
    assert_eq!(joined_count, 1);
}

// Test 13: Selectinload for collections
#[test]
fn test_selectinload_for_collections() {
    let mut db = TestDatabase::new();

    // Selectinload: parent query + 1 SELECT IN query
    db.reset_counters();
    let book_ids: Vec<i32> = db.books.iter().map(|b| b.id).collect();
    let reviews_map = db.get_books_with_selectin_reviews(&book_ids);

    assert_eq!(db.query_count, 1, "Should use single SELECT IN query");
    assert_eq!(reviews_map.len(), 3);
    assert_eq!(reviews_map[&1].len(), 2); // Book 1 has 2 reviews
    assert_eq!(reviews_map[&2].len(), 1); // Book 2 has 1 review
}

// Test 14: Multiple loading strategies on same query
#[test]
fn test_multiple_loading_strategies() {
    let options = LoadOptionBuilder::<Author>::new()
        .joinedload("books")
        .selectinload("books.reviews")
        .lazyload("profile")
        .build();

    assert_eq!(options.len(), 3);

    // Verify each strategy is independent
    let eager_count = options.iter().filter(|o| o.strategy().is_eager()).count();
    let lazy_count = options.iter().filter(|o| o.strategy().is_lazy()).count();

    assert_eq!(eager_count, 2); // joinedload + selectinload
    assert_eq!(lazy_count, 1); // lazyload
}

// Test 15: LoadContext prevents duplicate loading
#[test]
fn test_load_context_prevents_duplicates() {
    let mut ctx = LoadContext::new();

    ctx.mark_loaded("books".to_string(), LoadingStrategy::Joined);

    if !ctx.is_loaded("books") {
        panic!("Should not reload already loaded relationship");
    }

    // Verify strategy is recorded
    assert_eq!(ctx.strategy_for("books"), Some(LoadingStrategy::Joined));
}

// Test 16: WriteOnly strategy prevents reading
#[test]
fn test_writeonly_strategy() {
    let strategy = LoadingStrategy::WriteOnly;

    assert!(strategy.prevents_load());
    assert!(!strategy.is_eager());
    assert!(!strategy.is_lazy());

    // WriteOnly is for write-only collections
}

// Test 17: Dynamic strategy for filtered relationships
#[test]
fn test_dynamic_strategy() {
    let strategy = LoadingStrategy::Dynamic;

    // Dynamic returns a query object instead of loading
    assert!(!strategy.is_eager());
    assert!(!strategy.is_lazy());
    assert!(!strategy.prevents_load());
}

// Test 18: Nested relationship loading paths
#[test]
fn test_nested_relationship_paths() {
    let option = joinedload("author.books.reviews.user");

    let components = option.path_components();
    assert_eq!(components.len(), 4);
    assert_eq!(components[0], "author");
    assert_eq!(components[1], "books");
    assert_eq!(components[2], "reviews");
    assert_eq!(components[3], "user");
}

// Test 19: LoadContext with nested paths
#[test]
fn test_load_context_nested_paths() {
    let mut ctx = LoadContext::new();

    ctx.mark_loaded("author".to_string(), LoadingStrategy::Joined);
    ctx.mark_loaded("author.books".to_string(), LoadingStrategy::Selectin);
    ctx.mark_loaded(
        "author.books.reviews".to_string(),
        LoadingStrategy::Subquery,
    );

    assert_eq!(ctx.loaded_paths().len(), 3);
    assert!(ctx.is_loaded("author"));
    assert!(ctx.is_loaded("author.books"));
    assert!(ctx.is_loaded("author.books.reviews"));
}

// Test 20: Strategy selection based on relationship type
#[test]
fn test_strategy_selection_guidelines() {
    // Single object relationships: prefer joinedload
    let single_obj_strategy = LoadingStrategy::Joined;
    assert!(single_obj_strategy.is_eager());

    // Collection relationships: prefer selectinload (avoids cartesian product)
    let collection_strategy = LoadingStrategy::Selectin;
    assert!(collection_strategy.is_eager());

    // Development mode: use raiseload to catch N+1
    let dev_strategy = LoadingStrategy::Raise;
    assert!(dev_strategy.prevents_load());

    // Large collections: use writeonly
    let large_collection_strategy = LoadingStrategy::WriteOnly;
    assert!(large_collection_strategy.prevents_load());
}
