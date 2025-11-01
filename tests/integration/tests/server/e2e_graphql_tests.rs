#![cfg(feature = "graphql")]

use async_graphql::*;
use reinhardt_server::graphql_handler;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::test_helpers::{shutdown_test_server, spawn_test_server};

// GraphQL Schema for a simple book library

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Book {
    id: ID,
    title: String,
    author: String,
    year: i32,
}

#[Object]
impl Book {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn title(&self) -> &str {
        &self.title
    }

    async fn author(&self) -> &str {
        &self.author
    }

    async fn year(&self) -> i32 {
        self.year
    }
}

#[derive(Clone)]
struct BookStore {
    books: Arc<Mutex<HashMap<String, Book>>>,
    next_id: Arc<Mutex<u32>>,
}

impl BookStore {
    fn new() -> Self {
        let mut books = HashMap::new();
        books.insert(
            "1".to_string(),
            Book {
                id: ID::from("1"),
                title: "1984".to_string(),
                author: "George Orwell".to_string(),
                year: 1949,
            },
        );
        books.insert(
            "2".to_string(),
            Book {
                id: ID::from("2"),
                title: "To Kill a Mockingbird".to_string(),
                author: "Harper Lee".to_string(),
                year: 1960,
            },
        );

        Self {
            books: Arc::new(Mutex::new(books)),
            next_id: Arc::new(Mutex::new(3)),
        }
    }
}

struct QueryRoot {
    store: BookStore,
}

#[Object]
impl QueryRoot {
    async fn books(&self) -> Vec<Book> {
        let books = self.store.books.lock().unwrap();
        books.values().cloned().collect()
    }

    async fn book(&self, id: ID) -> Option<Book> {
        let books = self.store.books.lock().unwrap();
        books.get(id.as_str()).cloned()
    }

    async fn search_books(&self, title: String) -> Vec<Book> {
        let books = self.store.books.lock().unwrap();
        books
            .values()
            .filter(|book| book.title.to_lowercase().contains(&title.to_lowercase()))
            .cloned()
            .collect()
    }
}

struct MutationRoot {
    store: BookStore,
}

#[Object]
impl MutationRoot {
    async fn add_book(&self, title: String, author: String, year: i32) -> Book {
        let mut next_id = self.store.next_id.lock().unwrap();
        let id = ID::from(next_id.to_string());
        *next_id += 1;

        let book = Book {
            id: id.clone(),
            title,
            author,
            year,
        };

        let mut books = self.store.books.lock().unwrap();
        books.insert(id.to_string(), book.clone());

        book
    }

    async fn update_book(&self, id: ID, title: String, author: String, year: i32) -> Option<Book> {
        let mut books = self.store.books.lock().unwrap();
        if books.contains_key(id.as_str()) {
            let book = Book {
                id: id.clone(),
                title,
                author,
                year,
            };
            books.insert(id.to_string(), book.clone());
            Some(book)
        } else {
            None
        }
    }

    async fn delete_book(&self, id: ID) -> bool {
        let mut books = self.store.books.lock().unwrap();
        books.remove(id.as_str()).is_some()
    }
}

#[tokio::test]
async fn test_e2e_graphql_query_all_books() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let query = r#"{"query": "{ books { id title author year } }"}"#;

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("1984"));
    assert!(body.contains("To Kill a Mockingbird"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_query_single_book() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let query = r#"{"query": "{ book(id: \"1\") { id title author year } }"}"#;

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("1984"));
    assert!(body.contains("George Orwell"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_search_books() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let query = r#"{"query": "{ searchBooks(title: \"mockingbird\") { id title author } }"}"#;

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("To Kill a Mockingbird"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_add_book_mutation() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let mutation_query = r#"{"query": "mutation { addBook(title: \"The Great Gatsby\", author: \"F. Scott Fitzgerald\", year: 1925) { id title author year } }"}"#;

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(mutation_query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("The Great Gatsby"));
    assert!(body.contains("F. Scott Fitzgerald"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_update_book_mutation() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let mutation_query = r#"{"query": "mutation { updateBook(id: \"1\", title: \"Nineteen Eighty-Four\", author: \"George Orwell\", year: 1949) { id title } }"}"#;

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(mutation_query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("Nineteen Eighty-Four"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_delete_book_mutation() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Delete a book
    let mutation_query = r#"{"query": "mutation { deleteBook(id: \"1\") }"}"#;
    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(mutation_query)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // Try to fetch the deleted book
    let query = r#"{"query": "{ book(id: \"1\") { id title } }"}"#;
    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(query)
        .send()
        .await
        .unwrap();

    let body = response.text().await.unwrap();
    assert!(body.contains("null"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_full_workflow() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // 1. Query all books
    let query = r#"{"query": "{ books { id title } }"}"#;
    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(query)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // 2. Add a new book
    let add_query = r#"{"query": "mutation { addBook(title: \"The Catcher in the Rye\", author: \"J.D. Salinger\", year: 1951) { id title } }"}"#;
    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(add_query)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("The Catcher in the Rye"));

    // 3. Search for the new book
    let search_query = r#"{"query": "{ searchBooks(title: \"Catcher\") { id title } }"}"#;
    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(search_query)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("The Catcher in the Rye"));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_graphql_invalid_query() {
    let store = BookStore::new();
    let query = QueryRoot {
        store: store.clone(),
    };
    let mutation = MutationRoot { store };

    let handler = graphql_handler(query, mutation);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let invalid_query = r#"{"query": "{ invalidField { id } }"}"#;

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .body(invalid_query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200); // GraphQL returns 200 even for query errors
    let body = response.text().await.unwrap();
    assert!(body.contains("errors") || body.contains("error"));

    shutdown_test_server(handle).await;
}
