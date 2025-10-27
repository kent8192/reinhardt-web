# Database Integration Example

このexampleは、ReinhardtフレームワークでのデータベースORM、マイグレーション、データベース接続の統合方法を示します。

## 特徴

- **Django風のプロジェクト構造**: config/, settings/, apps.rsを使用
- **データベース設定管理**: 環境別のデータベース接続設定
- **マイグレーションシステム**: データベーススキーマのバージョン管理
- **manage CLI**: データベース管理コマンド（makemigrations, migrate）

## プロジェクト構造

```
src/
├── config/
│   ├── apps.rs              # インストール済みアプリの定義
│   ├── settings.rs          # 環境に応じた設定ローダー
│   ├── settings/
│   │   ├── base.rs          # 全環境共通の基本設定
│   │   ├── local.rs         # ローカル開発環境設定（DB設定含む）
│   │   ├── staging.rs       # ステージング環境設定
│   │   └── production.rs    # 本番環境設定
│   └── urls.rs              # URLルーティング設定
├── migrations.rs            # マイグレーション定義
├── migrations/              # マイグレーションファイル
│   └── 0001_initial.rs      # 初期マイグレーション
├── apps.rs                  # アプリレジストリ
├── config.rs                # configモジュール宣言
├── main.rs                  # アプリケーションエントリーポイント
└── bin/
    └── manage.rs            # 管理CLIツール
```

## セットアップ

### 前提条件

- Rust 2024 edition以降
- PostgreSQL, MySQL, またはSQLite
- Cargo

### データベースのセットアップ

#### PostgreSQL (推奨)

```bash
# PostgreSQLサーバーの起動
podman run -d \
  --name reinhardt-postgres \
  -e POSTGRES_USER=reinhardt \
  -e POSTGRES_PASSWORD=reinhardt_dev \
  -e POSTGRES_DB=reinhardt_examples \
  -p 5432:5432 \
  postgres:16
```

#### MySQL

```bash
# MySQLサーバーの起動
podman run -d \
  --name reinhardt-mysql \
  -e MYSQL_ROOT_PASSWORD=rootpass \
  -e MYSQL_DATABASE=reinhardt_examples \
  -e MYSQL_USER=reinhardt \
  -e MYSQL_PASSWORD=reinhardt_dev \
  -p 3306:3306 \
  mysql:8
```

#### SQLite

SQLiteを使用する場合は追加のセットアップは不要です。

### ビルド

```bash
# プロジェクトルートから
cargo build --package example-database-integration
```

**注**: このexampleはreinhardtが crates.ioに公開された後にビルド可能になります（version ^0.1）。

## 使用方法

### 環境変数の設定

```bash
# PostgreSQL（デフォルト）
export DATABASE_URL="postgres://reinhardt:reinhardt_dev@localhost:5432/reinhardt_examples"

# MySQL
export DATABASE_URL="mysql://reinhardt:reinhardt_dev@localhost:3306/reinhardt_examples"

# SQLite
export DATABASE_URL="sqlite://./db.sqlite3"
```

### マイグレーション管理

```bash
# 新しいマイグレーションの作成
cargo run --bin manage makemigrations

# マイグレーションの適用
cargo run --bin manage migrate

# マイグレーション計画の確認（dry-run）
cargo run --bin manage migrate --plan

# 特定のマイグレーションまで適用
cargo run --bin manage migrate app_name migration_name
```

### アプリケーションの実行

```bash
cargo run --package example-database-integration
```

出力例:
```
Database Integration Example
✅ Application initialized
Debug mode: true
Database URL: postgres://reinhardt:reinhardt_dev@localhost:5432/reinhardt_examples
✅ Application started successfully
```

## データベース設定

### local.rs での設定

```rust
use reinhardt_core::DatabaseConfig;

settings.database = Some(DatabaseConfig {
    url: database_url,
    max_connections: 10,
    min_connections: 1,
    connect_timeout: std::time::Duration::from_secs(30),
    idle_timeout: Some(std::time::Duration::from_secs(600)),
});
```

### 環境別の設定

| 環境 | ファイル | データベースURL | 接続プール |
|------|---------|----------------|-----------|
| local | local.rs | 環境変数 or デフォルト | 10 connections |
| staging | staging.rs | 環境変数必須 | 20 connections |
| production | production.rs | 環境変数必須 | 50 connections |

## マイグレーションの作成

### 1. マイグレーションファイルの作成

```bash
cargo run --bin manage makemigrations --name create_users_table
```

### 2. migrations/ディレクトリにファイルを作成

```rust
// migrations/0002_create_users_table.rs
use reinhardt::prelude::*;

pub struct Migration;

impl MigrationTrait for Migration {
    fn name(&self) -> &str {
        "0002_create_users_table"
    }

    async fn up(&self, db: &Database) -> Result<()> {
        db.execute(r#"
            CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                email VARCHAR(255) UNIQUE NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#).await?;
        Ok(())
    }

    async fn down(&self, db: &Database) -> Result<()> {
        db.execute("DROP TABLE users").await?;
        Ok(())
    }
}
```

### 3. migrations.rsに登録

```rust
// src/migrations.rs
mod _0001_initial;
mod _0002_create_users_table;

pub fn all_migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(_0001_initial::Migration),
        Box::new(_0002_create_users_table::Migration),
    ]
}
```

## トラブルシューティング

### 接続エラー

```
Error: Database connection failed
```

**解決方法:**
1. データベースサーバーが起動していることを確認
2. DATABASE_URL環境変数が正しく設定されているか確認
3. 認証情報（ユーザー名、パスワード）が正しいか確認

### マイグレーションエラー

```
Error: Migration failed: table already exists
```

**解決方法:**
1. `--fake` オプションでマイグレーションを適用済みとしてマーク
2. または `--fake-initial` で初期マイグレーションのみスキップ

```bash
cargo run --bin manage migrate --fake-initial
```

## 参考

- [Reinhardt ORM Documentation](https://docs.rs/reinhardt-orm)
- [Reinhardt Migrations Guide](https://docs.rs/reinhardt-migrations)
- [Django Migrations](https://docs.djangoproject.com/en/stable/topics/migrations/)
- [SQLAlchemy](https://www.sqlalchemy.org/)

## ライセンス

このexampleはReinhardtプロジェクトの一部として、MIT/Apache-2.0ライセンスの下で提供されています。
