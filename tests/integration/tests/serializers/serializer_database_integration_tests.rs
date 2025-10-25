// Database integration tests for ModelSerializer
use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, JsonSerializer, ModelSerializer,
    Serializer,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

// Test models

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Post {
    id: Option<i64>,
    title: String,
    content: String,
    published: bool,
}

impl Model for Post {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "posts"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Comment {
    id: Option<i64>,
    post_id: i64,
    author: String,
    text: String,
}

impl Model for Comment {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "comments"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

// Helper function to setup in-memory SQLite database
async fn setup_database() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(":memory:")
        .await
        .expect("Failed to create pool");

    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            published BOOLEAN NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create posts table");

    sqlx::query(
        r#"
        CREATE TABLE comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL,
            author TEXT NOT NULL,
            text TEXT NOT NULL,
            FOREIGN KEY (post_id) REFERENCES posts(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create comments table");

    pool
}

// Test: Create record in database and serialize
#[tokio::test]
async fn test_create_and_serialize_from_db() {
    let pool = setup_database().await;

    // Insert a post
    let result = sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind("First Post")
        .bind("This is the content")
        .bind(true)
        .execute(&pool)
        .await
        .expect("Failed to insert post");

    let post_id = result.last_insert_rowid();

    // Fetch the post
    let row = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch post");

    let post = Post {
        id: Some(row.0),
        title: row.1,
        content: row.2,
        published: row.3,
    };

    // Serialize using ModelSerializer
    let serializer = DefaultModelSerializer::<Post>::new();
    let serialized = Serializer::serialize(&serializer, &post).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("\"First Post\""));
    assert!(json_str.contains("\"This is the content\""));
}

// Test: Deserialize and insert into database
#[tokio::test]
async fn test_deserialize_and_insert_to_db() {
    let pool = setup_database().await;

    let serializer = DefaultModelSerializer::<Post>::new();
    let json_data = r#"{"id":null,"title":"New Post","content":"New content","published":false}"#;

    let post: Post = ReinhardtDeserializer::deserialize(&serializer, json_data.as_bytes()).unwrap();

    // Insert into database
    let result = sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind(&post.title)
        .bind(&post.content)
        .bind(post.published)
        .execute(&pool)
        .await
        .expect("Failed to insert post");

    assert!(result.last_insert_rowid() > 0);

    // Verify insertion
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE title = ?")
        .bind("New Post")
        .fetch_one(&pool)
        .await
        .expect("Failed to count");

    assert_eq!(count, 1);
}

// Test: Serialize query results (multiple records)
#[tokio::test]
async fn test_serialize_query_results() {
    let pool = setup_database().await;

    // Insert multiple posts
    for i in 1..=5 {
        sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
            .bind(format!("Post {}", i))
            .bind(format!("Content {}", i))
            .bind(i % 2 == 0)
            .execute(&pool)
            .await
            .expect("Failed to insert post");
    }

    // Fetch all posts
    let rows = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts",
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch posts");

    let posts: Vec<Post> = rows
        .into_iter()
        .map(|(id, title, content, published)| Post {
            id: Some(id),
            title,
            content,
            published,
        })
        .collect();

    assert_eq!(posts.len(), 5);

    // Serialize the list
    let serializer = JsonSerializer::<Vec<Post>>::new();
    let json_str = Serializer::serialize(&serializer, &posts).unwrap();

    assert!(json_str.contains("\"Post 1\""));
    assert!(json_str.contains("\"Post 5\""));
}

// Test: Update record with serializer
#[tokio::test]
async fn test_update_record_with_serializer() {
    let pool = setup_database().await;

    // Insert a post
    let result = sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind("Original Title")
        .bind("Original Content")
        .bind(false)
        .execute(&pool)
        .await
        .expect("Failed to insert post");

    let post_id = result.last_insert_rowid();

    // Fetch the post
    let row = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch post");

    let mut post = Post {
        id: Some(row.0),
        title: row.1,
        content: row.2,
        published: row.3,
    };

    // Update using serializer
    let serializer = DefaultModelSerializer::<Post>::new();
    let updated_data = Post {
        id: Some(post_id),
        title: "Updated Title".to_string(),
        content: "Updated Content".to_string(),
        published: true,
    };

    serializer.update(&mut post, updated_data).unwrap();

    // Save to database
    sqlx::query("UPDATE posts SET title = ?, content = ?, published = ? WHERE id = ?")
        .bind(&post.title)
        .bind(&post.content)
        .bind(post.published)
        .bind(post_id)
        .execute(&pool)
        .await
        .expect("Failed to update post");

    // Verify update
    let updated_row = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT title, content, published FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch updated post");

    assert_eq!(updated_row.0, "Updated Title");
    assert_eq!(updated_row.1, "Updated Content");
    assert_eq!(updated_row.2, true);
}

// Test: Relationship serialization with database
#[tokio::test]
async fn test_relationship_serialization_with_db() {
    let pool = setup_database().await;

    // Create a post
    let post_result = sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind("Post with Comments")
        .bind("Content")
        .bind(true)
        .execute(&pool)
        .await
        .expect("Failed to insert post");

    let post_id = post_result.last_insert_rowid();

    // Create comments
    for i in 1..=3 {
        sqlx::query("INSERT INTO comments (post_id, author, text) VALUES (?, ?, ?)")
            .bind(post_id)
            .bind(format!("Author {}", i))
            .bind(format!("Comment {}", i))
            .execute(&pool)
            .await
            .expect("Failed to insert comment");
    }

    // Fetch comments
    let rows = sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, post_id, author, text FROM comments WHERE post_id = ?",
    )
    .bind(post_id)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch comments");

    let comments: Vec<Comment> = rows
        .into_iter()
        .map(|(id, post_id, author, text)| Comment {
            id: Some(id),
            post_id,
            author,
            text,
        })
        .collect();

    assert_eq!(comments.len(), 3);

    // Serialize comments
    let serializer = JsonSerializer::<Vec<Comment>>::new();
    let json_str = Serializer::serialize(&serializer, &comments).unwrap();

    assert!(json_str.contains("\"Author 1\""));
    assert!(json_str.contains("\"Comment 3\""));
}

// Test: Transaction with serializer
#[tokio::test]
async fn test_transaction_with_serializer() {
    let pool = setup_database().await;

    let serializer = DefaultModelSerializer::<Post>::new();

    // Create post using serializer
    let post = Post {
        id: None,
        title: "Transactional Post".to_string(),
        content: "Content in transaction".to_string(),
        published: false,
    };

    let created = serializer.create(post.clone()).unwrap();

    // Begin transaction
    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    // Insert in transaction
    let result = sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind(&created.title)
        .bind(&created.content)
        .bind(created.published)
        .execute(&mut *tx)
        .await
        .expect("Failed to insert in transaction");

    let post_id = result.last_insert_rowid();

    // Commit transaction
    tx.commit().await.expect("Failed to commit transaction");

    // Verify
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count");

    assert_eq!(count, 1);
}

// Test: Rollback transaction
#[tokio::test]
async fn test_serializer_db_transaction_rollback() {
    let pool = setup_database().await;

    // Begin transaction
    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    // Insert in transaction
    sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind("Will be rolled back")
        .bind("This won't persist")
        .bind(false)
        .execute(&mut *tx)
        .await
        .expect("Failed to insert in transaction");

    // Rollback transaction (implicitly by dropping tx without commit)
    drop(tx);

    // Verify rollback
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE title = ?")
        .bind("Will be rolled back")
        .fetch_one(&pool)
        .await
        .expect("Failed to count");

    assert_eq!(count, 0);
}

// Test: Serialize with filtering (published posts only)
#[tokio::test]
async fn test_serialize_with_filtering() {
    let pool = setup_database().await;

    // Insert posts with different published status
    for i in 1..=10 {
        sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
            .bind(format!("Post {}", i))
            .bind(format!("Content {}", i))
            .bind(i % 2 == 0) // Even numbered posts are published
            .execute(&pool)
            .await
            .expect("Failed to insert post");
    }

    // Fetch only published posts
    let rows = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts WHERE published = ?",
    )
    .bind(true)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch posts");

    let posts: Vec<Post> = rows
        .into_iter()
        .map(|(id, title, content, published)| Post {
            id: Some(id),
            title,
            content,
            published,
        })
        .collect();

    assert_eq!(posts.len(), 5); // Only even numbered posts

    // Serialize
    let serializer = JsonSerializer::<Vec<Post>>::new();
    let serialized = Serializer::serialize(&serializer, &posts).unwrap();
    let deserialized: Vec<Post> =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    // Verify all are published
    assert!(deserialized.iter().all(|p| p.published));
}

// Test: Bulk insert with serializer
#[tokio::test]
async fn test_bulk_insert_with_serializer() {
    let pool = setup_database().await;

    let serializer = DefaultModelSerializer::<Post>::new();

    // Create multiple posts
    let posts = vec![
        Post {
            id: None,
            title: "Bulk Post 1".to_string(),
            content: "Content 1".to_string(),
            published: false,
        },
        Post {
            id: None,
            title: "Bulk Post 2".to_string(),
            content: "Content 2".to_string(),
            published: true,
        },
        Post {
            id: None,
            title: "Bulk Post 3".to_string(),
            content: "Content 3".to_string(),
            published: false,
        },
    ];

    // Create and insert each
    for post in posts {
        let created = serializer.create(post).unwrap();
        sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
            .bind(&created.title)
            .bind(&created.content)
            .bind(created.published)
            .execute(&pool)
            .await
            .expect("Failed to insert post");
    }

    // Verify
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE title LIKE ?")
        .bind("Bulk Post%")
        .fetch_one(&pool)
        .await
        .expect("Failed to count");

    assert_eq!(count, 3);
}

// Test: Serializer with NULL values in database
#[tokio::test]
async fn test_null_values_in_database() {
    let pool = setup_database().await;

    // Create table with nullable column
    sqlx::query(
        r#"
        CREATE TABLE articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            subtitle TEXT,
            content TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create articles table");

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Article {
        id: Option<i64>,
        title: String,
        subtitle: Option<String>,
        content: String,
    }

    impl Model for Article {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "articles"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Insert article with NULL subtitle
    let result = sqlx::query("INSERT INTO articles (title, subtitle, content) VALUES (?, ?, ?)")
        .bind("Article Title")
        .bind(None::<String>)
        .bind("Article Content")
        .execute(&pool)
        .await
        .expect("Failed to insert article");

    let article_id = result.last_insert_rowid();

    // Fetch and serialize
    let row = sqlx::query_as::<_, (i64, String, Option<String>, String)>(
        "SELECT id, title, subtitle, content FROM articles WHERE id = ?",
    )
    .bind(article_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch article");

    let article = Article {
        id: Some(row.0),
        title: row.1,
        subtitle: row.2,
        content: row.3,
    };

    assert_eq!(article.subtitle, None);

    let serializer = DefaultModelSerializer::<Article>::new();
    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let deserialized: Article =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(article, deserialized);
    assert_eq!(deserialized.subtitle, None);
}

// Test: Pagination with serializer
#[tokio::test]
async fn test_serializer_database_pagination() {
    let pool = setup_database().await;

    // Insert 20 posts
    for i in 1..=20 {
        sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
            .bind(format!("Post {:02}", i))
            .bind(format!("Content {}", i))
            .bind(true)
            .execute(&pool)
            .await
            .expect("Failed to insert post");
    }

    // Fetch page 1 (first 10)
    let rows = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts ORDER BY id LIMIT ? OFFSET ?",
    )
    .bind(10)
    .bind(0)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch posts");

    let page1: Vec<Post> = rows
        .into_iter()
        .map(|(id, title, content, published)| Post {
            id: Some(id),
            title,
            content,
            published,
        })
        .collect();

    assert_eq!(page1.len(), 10);
    assert!(page1[0].title.contains("Post 01"));

    // Serialize page
    let serializer = JsonSerializer::<Vec<Post>>::new();
    let json_str = Serializer::serialize(&serializer, &page1).unwrap();

    assert!(json_str.contains("\"Post 01\""));
    assert!(!json_str.contains("\"Post 11\"")); // Not in page 1
}

// Test: Foreign key constraint with serializer
#[tokio::test]
async fn test_serializer_foreign_key_constraint() {
    let pool = setup_database().await;

    // Insert a post
    let post_result = sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
        .bind("Parent Post")
        .bind("Content")
        .bind(true)
        .execute(&pool)
        .await
        .expect("Failed to insert post");

    let post_id = post_result.last_insert_rowid();

    // Create comment using serializer
    let serializer = DefaultModelSerializer::<Comment>::new();
    let comment = Comment {
        id: None,
        post_id,
        author: "Test Author".to_string(),
        text: "Test comment".to_string(),
    };

    let created = serializer.create(comment).unwrap();

    // Insert comment with valid FK
    let result = sqlx::query("INSERT INTO comments (post_id, author, text) VALUES (?, ?, ?)")
        .bind(created.post_id)
        .bind(&created.author)
        .bind(&created.text)
        .execute(&pool)
        .await
        .expect("Failed to insert comment");

    assert!(result.last_insert_rowid() > 0);
}

// Test: Ordering query results
#[tokio::test]
async fn test_ordering_query_results() {
    let pool = setup_database().await;

    // Insert posts in random order
    let titles = vec!["Zebra", "Apple", "Mango", "Banana"];
    for title in titles {
        sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
            .bind(title)
            .bind("Content")
            .bind(true)
            .execute(&pool)
            .await
            .expect("Failed to insert post");
    }

    // Fetch ordered by title
    let rows = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts ORDER BY title ASC",
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch posts");

    let posts: Vec<Post> = rows
        .into_iter()
        .map(|(id, title, content, published)| Post {
            id: Some(id),
            title,
            content,
            published,
        })
        .collect();

    // Verify order
    assert_eq!(posts[0].title, "Apple");
    assert_eq!(posts[1].title, "Banana");
    assert_eq!(posts[2].title, "Mango");
    assert_eq!(posts[3].title, "Zebra");

    // Serialize ordered results
    let serializer = JsonSerializer::<Vec<Post>>::new();
    let serialized = Serializer::serialize(&serializer, &posts).unwrap();
    let deserialized: Vec<Post> =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(posts, deserialized);
}

// Test: Count and aggregation with serializer
#[tokio::test]
async fn test_count_and_aggregation() {
    let pool = setup_database().await;

    // Insert posts
    for i in 1..=5 {
        sqlx::query("INSERT INTO posts (title, content, published) VALUES (?, ?, ?)")
            .bind(format!("Post {}", i))
            .bind(format!("Content {}", i))
            .bind(i % 2 == 0)
            .execute(&pool)
            .await
            .expect("Failed to insert post");
    }

    // Count total posts
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&pool)
        .await
        .expect("Failed to count");

    assert_eq!(total, 5);

    // Count published posts
    let published_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE published = ?")
        .bind(true)
        .fetch_one(&pool)
        .await
        .expect("Failed to count published");

    assert_eq!(published_count, 2);

    // Fetch and serialize published posts
    let rows = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, title, content, published FROM posts WHERE published = ?",
    )
    .bind(true)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch posts");

    let posts: Vec<Post> = rows
        .into_iter()
        .map(|(id, title, content, published)| Post {
            id: Some(id),
            title,
            content,
            published,
        })
        .collect();

    assert_eq!(posts.len(), published_count as usize);
}
