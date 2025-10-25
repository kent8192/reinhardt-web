# reinhardt-database

Reinhardtフレームワークのためのデータベースアブストラクションレイヤーであり、SQLデータベース向けの統一されたインターフェースを提供します。

## 概要

`reinhardt-database`は、トレイトの継承とダイアレクト固有の実装を通じて、複数のSQLデータベースをサポートする統一されたデータベースアブストラクションレイヤーを提供します。このクレートは、低レベルのデータベース操作向けに設計されており、より高レベルのORM機能のための`reinhardt-orm`と連携して動作します。

## 機能のステータス

### コアアブストラクションレイヤー ✓

#### データベースバックエンドトレイト (実装済み ✓)

- 統一されたデータベースインターフェースのための`DatabaseBackend`トレイト
- 非同期データベース操作 (execute, fetch_one, fetch_all, fetch_optional)
- データベース機能の検出 (RETURNINGクローズ、ON CONFLICTサポート)
- パラメータ化クエリのプレースホルダー生成
- sqlx経由のコネクションプーリングサポート

#### クエリビルダー (実装済み ✓)

- **SelectBuilder**: WHERE、LIMITクローズを持つSELECTクエリ
- **InsertBuilder**: オプションのRETURNINGサポートを持つINSERT
- **UpdateBuilder**: SETおよびWHEREクローズ、NOW()関数サポートを持つUPDATE
- **DeleteBuilder**: WHERE条件とIN演算子サポートを持つDELETE
- `QueryValue`列挙型による型安全なパラメータバインディング

#### 型システム (実装済み ✓)

- `QueryValue`列挙型: Null, Bool, Int, Float, String, Bytes, Timestamp
- 型安全なカラムアクセスを持つクエリ結果のための`Row`型
- 影響を受けた行を追跡するための`QueryResult`
- エラーハンドリングを伴う自動型変換
- `DatabaseType`列挙型: Postgres, Sqlite, Mysql

#### スキーマエディタシステム (実装済み ✓)

- DDL操作のための`BaseDatabaseSchemaEditor`トレイト
- CREATE/DROP TABLEサポート
- ALTER TABLE操作 (カラム、制約の追加/削除/名前変更)
- 一意インデックスと部分インデックスサポートを持つCREATE/DROP INDEX
- DDLステートメントの型と参照
- データベース固有のエディタのためのファクトリーパターン

### SQLデータベースサポート

#### PostgreSQL (実装済み ✓)

- **コネクション管理**: sqlx PgPool経由のコネクションプーリング
- **クエリ実行**: パラメータバインディングを持つ完全な非同期クエリサポート
- **型マッピング**: 包括的な型変換 (プリミティブ、タイムスタンプ、バイト、NULL)
- **データベース機能**:
  - RETURNINGクローズサポート ✓
  - ON CONFLICTクローズサポート ✓
  - パラメータ化クエリ ($1, $2, ...) ✓
- **スキーマエディタ**:
  - 標準的なDDL操作 ✓
  - CREATE/DROP INDEX CONCURRENTLY ✓
  - IDENTITYカラム (ADD/DROP IDENTITY) ✓
  - シーケンス操作 (ALTER/DROP SEQUENCE) ✓
  - LIKEパターンインデックス (varchar_pattern_ops, text_pattern_ops) ✓
  - より安全な操作のためのIF EXISTSサポート ✓

#### MySQL (実装済み ✓)

- **コネクション管理**: sqlx MySqlPool経由のコネクションプーリング
- **クエリ実行**: パラメータバインディングを持つ完全な非同期クエリサポート
- **型マッピング**: 包括的な型変換 (プリミティブ、タイムスタンプ、バイト、NULL)
- **データベース機能**:
  - RETURNINGクローズサポート: なし (MySQLの制限)
  - ON CONFLICTクローズサポート: なし (MySQLの制限)
  - パラメータ化クエリ (?) ✓
- **スキーマエディタ**: 標準的なDDL操作 ✓

#### SQLite (実装済み ✓)

- **コネクション管理**: sqlx SqlitePool経由のコネクションプーリング
- **クエリ実行**: パラメータバインディングを持つ完全な非同期クエリサポート
- **型マッピング**: 包括的な型変換 (プリミティブ、タイムスタンプ、バイト、NULL)
- **データベース機能**:
  - RETURNINGクローズサポート ✓
  - ON CONFLICTクローズサポート ✓
  - パラメータ化クエリ (?) ✓
- **スキーマエディタ**: 標準的なDDL操作 ✓

### 予定されている機能

#### PostgreSQL高度な機能 (予定)

- 配列フィールド操作
- JSONBフィールド操作と演算子
- HStoreフィールドサポート
- 全文検索 (tsvector, tsquery, 検索設定)
- 範囲型 (int4range, int8range, tsrange, など)
- 幾何型
- ネットワークアドレス型 (inet, cidr, macaddr)
- UUID型サポート
- カスタム型とドメイン

#### MySQL高度な機能 (予定)

- JSONフィールド操作とパス式
- 全文検索 (FULLTEXTインデックス、MATCH AGAINST)
- 空間データ型と操作
- XAトランザクションサポート
- パーティション管理

#### SQLite高度な機能 (予定)

- JSON1拡張操作
- FTS5全文検索
- R\*Tree空間インデックス
- 仮想テーブルサポート
- 共通テーブル式 (CTE)

#### 一般的な拡張 (予定)

- トランザクション管理
- コネクションプール設定
- 大規模データセットのためのクエリ結果ストリーミング
- プリペアドステートメントキャッシング
- データベースマイグレーションサポート
- コネクションヘルスチェック
- 一時的な障害のための再試行ロジック
- データベース固有のエラーハンドリング
- クエリログとメトリクス

## インストール

```toml
[dependencies]
# デフォルト: PostgreSQLサポートのみ
reinhardt-database = "0.1.0"

# すべてのSQLデータベース
reinhardt-database = { version = "0.1.0", features = ["all-databases"] }

# カスタム組み合わせ
reinhardt-database = { version = "0.1.0", default-features = false, features = ["postgres", "mysql"] }

# SQLiteのみ
reinhardt-database = { version = "0.1.0", default-features = false, features = ["sqlite"] }
```

## 使用例

### 基本的なクエリ操作

```rust
use reinhardt_database::{DatabaseConnection, QueryValue};

// PostgreSQLに接続
let conn = DatabaseConnection::connect_postgres("postgresql://localhost/mydb").await?;

// データを挿入
let result = conn
    .insert("users")
    .value("name", "Alice")
    .value("email", "alice@example.com")
    .execute()
    .await?;

// データを更新
conn.update("users")
    .set("email", "newemail@example.com")
    .where_eq("name", "Alice")
    .execute()
    .await?;

// データを選択
let rows = conn
    .select()
    .columns(vec!["id", "name", "email"])
    .from("users")
    .where_eq("name", "Alice")
    .limit(10)
    .fetch_all()
    .await?;

// データを削除
conn.delete("users")
    .where_eq("id", QueryValue::Int(1))
    .execute()
    .await?;
```

### スキーマ操作

```rust
use reinhardt_database::schema::factory::{SchemaEditorFactory, DatabaseType};

let factory = SchemaEditorFactory::new();
let editor = factory.create_for_database(DatabaseType::PostgreSQL);

// CREATE TABLE SQLを生成
let sql = editor.create_table_sql("users", &[
    ("id", "SERIAL PRIMARY KEY"),
    ("name", "VARCHAR(100) NOT NULL"),
    ("email", "VARCHAR(255) UNIQUE"),
    ("created_at", "TIMESTAMP DEFAULT NOW()"),
]);

// CREATE INDEX SQLを生成
let index_sql = editor.create_index_sql(
    "idx_users_email",
    "users",
    &["email"],
    false,
    None,
);
```

### PostgreSQL固有の機能

```rust
use reinhardt_database::backends::postgresql::schema::PostgreSQLSchemaEditor;

let editor = PostgreSQLSchemaEditor::new();

// 書き込みをブロックせずにインデックスを作成
let sql = editor.create_index_concurrently_sql(
    "idx_email",
    "users",
    &["email"],
    false,
    None,
);

// IDENTITYカラムを追加
let identity_sql = editor.add_identity_sql("users", "id");

// テキスト検索用のLIKEパターンインデックスを作成
let like_index = editor.create_like_index_sql("users", "name", "varchar(100)");
```

### マルチデータベースサポート

```rust
use reinhardt_database::{DatabaseConnection, backend::DatabaseBackend};

// 異なるデータベースに接続
let pg_conn = DatabaseConnection::connect_postgres("postgresql://localhost/db").await?;
let mysql_conn = DatabaseConnection::connect_mysql("mysql://localhost/db").await?;
let sqlite_conn = DatabaseConnection::connect_sqlite("sqlite::memory:").await?;

// 統一されたインターフェースを使用
async fn insert_user(conn: &DatabaseConnection, name: &str) -> Result<()> {
    conn.insert("users")
        .value("name", name)
        .execute()
        .await?;
    Ok(())
}
```

## 機能フラグ

| 機能            | 説明                         | デフォルト |
| --------------- | ---------------------------- | ---------- |
| `postgres`      | PostgreSQLサポート           | ✅         |
| `mysql`         | MySQLサポート                | ❌         |
| `sqlite`        | SQLiteサポート               | ❌         |
| `all-databases` | すべてのSQLデータベースを有効化 | ❌         |

## アーキテクチャ

### トレイトベースの設計

```
DatabaseBackend (トレイト)
├── PostgresBackend (PostgreSQL実装)
├── MySqlBackend (MySQL実装)
└── SqliteBackend (SQLite実装)

BaseDatabaseSchemaEditor (トレイト)
├── PostgreSQLSchemaEditor (PG固有の操作を持つ)
├── MySQLSchemaEditor (標準的なDDL)
└── SQLiteSchemaEditor (標準的なDDL)
```

### コンポーネントレイヤー

1. **型システム**: `QueryValue`, `Row`, `QueryResult`, `DatabaseType`
2. **バックエンドアブストラクション**: データベース操作のための`DatabaseBackend`トレイト
3. **コネクション管理**: コネクションプーリングを持つ`DatabaseConnection`ラッパー
4. **クエリビルダー**: `SelectBuilder`, `InsertBuilder`, `UpdateBuilder`, `DeleteBuilder`
5. **スキーマエディタ**: `BaseDatabaseSchemaEditor`トレイトとデータベース固有の実装
6. **ダイアレクト実装**: PostgreSQL, MySQL, SQLiteバックエンド

## 設計哲学

- **低レベル操作**: ORM機能ではなく、データベースアブストラクションに焦点を当てています
- **型安全性**: コンパイル時の保証のためにRustの型システムを活用します
- **非同期ファースト**: 効率的なI/Oのためのasync/awaitとsqlxに基づいて構築されています
- **データベース非依存**: データベース固有の拡張を持つ統一されたインターフェース
- **拡張可能**: 新しいデータベースを追加したり、既存のデータベースを拡張したりするのが簡単です

## 他のクレートとの関係

- **reinhardt-orm**: 低レベル操作のために`reinhardt-database`を使用し、高レベルのORM機能を提供します
- **reinhardt-migrations**: データベースマイグレーションのためにスキーマエディタを使用します
- **reinhardt**: すべてのコンポーネントを統合するメインフレームワーク

## パフォーマンスに関する考慮事項

- 効率的なリソース使用のためのsqlx経由のコネクションプーリング
- SQLインジェクションを防ぎ、プリペアドステートメントキャッシングを有効にするためのパラメータ化クエリ
- 型変換は可能な限りゼロコストです
- 非同期操作により並行的なデータベースアクセスが可能になります

## ライセンス

次のいずれかの下でライセンスされています:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好きな方を選択してください。

## コントリビューション

コントリビューションを歓迎します！このクレートはReinhardtフレームワークの一部です。ガイドラインについては、メインの[CONTRIBUTING.md](../../CONTRIBUTING.md)を参照してください。
