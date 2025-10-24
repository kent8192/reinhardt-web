# Part 2: Models and Database

In this tutorial, we'll set up a database and create our first models to store poll data.

## Database Setup

Reinhardt supports multiple databases including PostgreSQL, MySQL, and SQLite. For this tutorial, we'll use SQLite for simplicity.

### Configuring the Database

First, add the database dependencies to `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["standard"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }
```

Create a database configuration file. Add this to your `src/main.rs`:

```rust
use sqlx::SqlitePool;

async fn setup_database() -> Result<SqlitePool, sqlx::Error> {
    // Create a SQLite database pool
    let pool = SqlitePool::connect("sqlite:polls.db").await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
```

## Creating Models

A model is the single, definitive source of information about your data. It contains the essential fields and behaviors of the data you're storing.

Let's create two models for our polls application:

- **Question** - Stores poll questions with their publication date
- **Choice** - Stores choices for each question with their vote counts

Create a new file `src/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Question {
    pub id: i64,
    pub question_text: String,
    pub pub_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Choice {
    pub id: i64,
    pub question_id: i64,
    pub choice_text: String,
    pub votes: i32,
}

impl Question {
    /// Check if this question was published recently (within the last day)
    pub fn was_published_recently(&self) -> bool {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);
        self.pub_date >= one_day_ago && self.pub_date <= now
    }
}
```

These models define:

- **Question**: Has an auto-incrementing ID, question text, and publication date
- **Choice**: Has an ID, references a Question (via `question_id`), choice text, and vote count

## Understanding Fields

Let's break down the field types:

- `i64` - Integer field for IDs
- `String` - Character field for text
- `DateTime<Utc>` - DateTime field for timestamps
- `i32` - Integer field for vote counts

The `#[derive(FromRow)]` attribute allows SQLx to automatically convert database rows into our structs.

## Creating the Database Schema

Create a migrations directory and your first migration:

```bash
mkdir -p migrations
```

Create `migrations/20240101000000_create_polls.sql`:

```sql
-- Create questions table
CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_text TEXT NOT NULL,
    pub_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create choices table
CREATE TABLE IF NOT EXISTS choices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER NOT NULL,
    choice_text TEXT NOT NULL,
    votes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (question_id) REFERENCES questions(id) ON DELETE CASCADE
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_choices_question_id ON choices(question_id);
```

## Playing with the Database API

Now let's create some helper functions to interact with the database. Add to `src/models.rs`:

```rust
use sqlx::SqlitePool;

impl Question {
    /// Create a new question
    pub async fn create(
        pool: &SqlitePool,
        question_text: String,
        pub_date: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            "INSERT INTO questions (question_text, pub_date) VALUES (?, ?)",
            question_text,
            pub_date
        )
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get all questions
    pub async fn all(pool: &SqlitePool) -> Result<Vec<Question>, sqlx::Error> {
        let questions = sqlx::query_as!(
            Question,
            "SELECT id, question_text, pub_date FROM questions ORDER BY pub_date DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(questions)
    }

    /// Get a question by ID
    pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Question>, sqlx::Error> {
        let question = sqlx::query_as!(
            Question,
            "SELECT id, question_text, pub_date FROM questions WHERE id = ?",
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(question)
    }

    /// Get all choices for this question
    pub async fn choices(&self, pool: &SqlitePool) -> Result<Vec<Choice>, sqlx::Error> {
        Choice::filter_by_question(pool, self.id).await
    }
}

impl Choice {
    /// Create a new choice
    pub async fn create(
        pool: &SqlitePool,
        question_id: i64,
        choice_text: String,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            "INSERT INTO choices (question_id, choice_text, votes) VALUES (?, ?, 0)",
            question_id,
            choice_text
        )
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get all choices for a question
    pub async fn filter_by_question(
        pool: &SqlitePool,
        question_id: i64,
    ) -> Result<Vec<Choice>, sqlx::Error> {
        let choices = sqlx::query_as!(
            Choice,
            "SELECT id, question_id, choice_text, votes FROM choices WHERE question_id = ?",
            question_id
        )
        .fetch_all(pool)
        .await?;

        Ok(choices)
    }
}
```

## Testing the Models

Let's add a simple test to verify our models work. Add to `src/main.rs`:

```rust
mod models;

use sqlx::SqlitePool;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup database
    let pool = SqlitePool::connect("sqlite:polls.db").await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    // Create a question
    let question_id = models::Question::create(
        &pool,
        "What's your favorite programming language?".to_string(),
        Utc::now(),
    )
    .await?;

    println!("Created question with ID: {}", question_id);

    // Add some choices
    models::Choice::create(&pool, question_id, "Rust".to_string()).await?;
    models::Choice::create(&pool, question_id, "Python".to_string()).await?;
    models::Choice::create(&pool, question_id, "JavaScript".to_string()).await?;

    // Retrieve the question
    let question = models::Question::get(&pool, question_id)
        .await?
        .expect("Question not found");

    println!("Question: {}", question.question_text);
    println!("Published: {}", question.pub_date);
    println!("Recently published? {}", question.was_published_recently());

    // Get choices
    let choices = question.choices(&pool).await?;
    println!("Choices:");
    for choice in choices {
        println!("  - {} (votes: {})", choice.choice_text, choice.votes);
    }

    Ok(())
}
```

Run the program:

```bash
cargo run
```

You should see output like:

```
Created question with ID: 1
Question: What's your favorite programming language?
Published: 2024-01-15 10:30:00 UTC
Recently published? true
Choices:
  - Rust (votes: 0)
  - Python (votes: 0)
  - JavaScript (votes: 0)
```

## Introduction to the Reinhardt Admin

The Reinhardt admin is an automatically-generated interface for managing your data. Let's enable it for our models.

Add the admin dependency to `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["standard", "admin"] }
```

The admin interface will be covered in more detail in Part 7, but for now, know that you can register your models to make them editable through a web interface.

## Summary

In this tutorial, you learned:

- How to configure a database connection
- How to define models with fields and methods
- How to create database migrations
- How to perform CRUD operations (Create, Read, Update, Delete)
- The relationship between models (foreign keys)
- How to query the database using the model API

## What's Next?

Now that our models are set up, we can start building views that display this data to users. In the next tutorial, we'll create views that show poll questions and their details.

Continue to [Part 3: Views and URLs](3-views-and-urls.md).
