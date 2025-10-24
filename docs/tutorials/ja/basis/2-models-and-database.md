# パート2: モデルとデータベース

このチュートリアルでは、データベースをセットアップし、投票データを保存する最初のモデルを作成します。

## データベースのセットアップ

ReinhardtはPostgreSQL、MySQL、SQLiteなど複数のデータベースをサポートしています。このチュートリアルでは、シンプルさのためSQLiteを使用します。

### データベースの設定

まず、`Cargo.toml`にデータベース依存関係を追加します：

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["standard", "database", "migrations"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }
```

データベース設定ファイルを作成します。`src/main.rs`に以下を追加します：

```rust
use sqlx::SqlitePool;

async fn setup_database() -> Result<SqlitePool, sqlx::Error> {
    // SQLiteデータベースプールを作成
    let pool = SqlitePool::connect("sqlite:polls.db").await?;

    // マイグレーションを実行
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
```

## モデルの作成

モデルは、データに関する唯一の決定的な情報源です。保存するデータの必須フィールドと動作が含まれます。

投票アプリケーション用に2つのモデルを作成しましょう：

- **Question** - 投票の質問と公開日時を保存
- **Choice** - 各質問の選択肢と投票数を保存

新しいファイル`src/models.rs`を作成します：

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
    /// この質問が最近公開されたかチェック（過去1日以内）
    pub fn was_published_recently(&self) -> bool {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);
        self.pub_date >= one_day_ago && self.pub_date <= now
    }
}
```

これらのモデルは以下を定義します：

- **Question**: 自動増分ID、質問テキスト、公開日時を持つ
- **Choice**: ID、Questionへの参照（`question_id`経由）、選択肢テキスト、投票数を持つ

## フィールドの理解

フィールドタイプを分解してみましょう：

- `i64` - IDのための整数フィールド
- `String` - テキストのための文字列フィールド
- `DateTime<Utc>` - タイムスタンプのためのDateTimeフィールド
- `i32` - 投票数のための整数フィールド

`#[derive(FromRow)]`属性により、SQLxがデータベース行を自動的に構造体に変換できます。

## データベーススキーマの作成

migrationsディレクトリと最初のマイグレーションを作成します：

```bash
mkdir -p migrations
```

`migrations/20240101000000_create_polls.sql`を作成します：

```sql
-- questionsテーブルを作成
CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_text TEXT NOT NULL,
    pub_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- choicesテーブルを作成
CREATE TABLE IF NOT EXISTS choices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER NOT NULL,
    choice_text TEXT NOT NULL,
    votes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (question_id) REFERENCES questions(id) ON DELETE CASCADE
);

-- 高速検索のためのインデックスを作成
CREATE INDEX IF NOT EXISTS idx_choices_question_id ON choices(question_id);
```

## データベースAPIで遊ぶ

では、データベースと対話するヘルパー関数を作成しましょう。`src/models.rs`に追加します：

```rust
use sqlx::SqlitePool;

impl Question {
    /// 新しい質問を作成
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

    /// すべての質問を取得
    pub async fn all(pool: &SqlitePool) -> Result<Vec<Question>, sqlx::Error> {
        let questions = sqlx::query_as!(
            Question,
            "SELECT id, question_text, pub_date FROM questions ORDER BY pub_date DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(questions)
    }

    /// IDで質問を取得
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

    /// この質問のすべての選択肢を取得
    pub async fn choices(&self, pool: &SqlitePool) -> Result<Vec<Choice>, sqlx::Error> {
        Choice::filter_by_question(pool, self.id).await
    }
}

impl Choice {
    /// 新しい選択肢を作成
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

    /// 質問のすべての選択肢を取得
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

## モデルのテスト

モデルが動作することを確認する簡単なテストを追加しましょう。`src/main.rs`に追加します：

```rust
mod models;

use sqlx::SqlitePool;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // データベースをセットアップ
    let pool = SqlitePool::connect("sqlite:polls.db").await?;

    // マイグレーションを実行
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    // 質問を作成
    let question_id = models::Question::create(
        &pool,
        "What's your favorite programming language?".to_string(),
        Utc::now(),
    )
    .await?;

    println!("Created question with ID: {}", question_id);

    // 選択肢を追加
    models::Choice::create(&pool, question_id, "Rust".to_string()).await?;
    models::Choice::create(&pool, question_id, "Python".to_string()).await?;
    models::Choice::create(&pool, question_id, "JavaScript".to_string()).await?;

    // 質問を取得
    let question = models::Question::get(&pool, question_id)
        .await?
        .expect("Question not found");

    println!("Question: {}", question.question_text);
    println!("Published: {}", question.pub_date);
    println!("Recently published? {}", question.was_published_recently());

    // 選択肢を取得
    let choices = question.choices(&pool).await?;
    println!("Choices:");
    for choice in choices {
        println!("  - {} (votes: {})", choice.choice_text, choice.votes);
    }

    Ok(())
}
```

プログラムを実行します：

```bash
cargo run
```

次のような出力が表示されるはずです：

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

## Reinhardt管理画面の紹介

Reinhardt管理画面は、データを管理するための自動生成インターフェースです。モデル用に有効化しましょう。

`Cargo.toml`に管理画面機能を追加します：

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["standard", "database", "migrations", "admin"] }
```

管理画面インターフェースについては、パート7で詳しく説明しますが、今のところ、モデルを登録してWebインターフェースから編集可能にできることを知っておいてください。

## まとめ

このチュートリアルで学んだこと：

- データベース接続の設定方法
- フィールドとメソッドを持つモデルの定義方法
- データベースマイグレーションの作成方法
- CRUD操作（作成、読み取り、更新、削除）の実行方法
- モデル間の関係（外部キー）
- モデルAPIを使用したデータベースクエリ方法

## 次は何をする？

モデルがセットアップできたので、このデータをユーザーに表示するビューの構築を始めることができます。次のチュートリアルでは、投票の質問とその詳細を表示するビューを作成します。

[パート3: ビューとURL](3-views-and-urls.md)に進んでください。
