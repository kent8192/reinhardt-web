# reinhardt-migrations

Djangoのマイグレーションシステムにインスパイアされたデータベーススキーママイグレーションツール

## 概要

PostgreSQL、MySQL、SQLiteに対応したスキーマ変更を管理するためのデータベースマイグレーションシステムです。モデルの変更から自動的にマイグレーションを生成し、前進・後退マイグレーションをサポートし、依存関係管理を備えたスキーマバージョニングを処理します。

## 機能

### 実装済み ✓

#### コアマイグレーションシステム

- **マイグレーション操作**: スキーマ変更のための包括的な操作セット
  - モデル操作: `CreateModel`, `DeleteModel`, `RenameModel`
  - フィールド操作: `AddField`, `RemoveField`, `AlterField`, `RenameField`
  - 特殊操作: `RunSQL`, `RunCode` (DjangoのRunPythonに相当するRust版)
  - PostgreSQL固有: `CreateExtension`, `DropExtension`, `CreateCollation`

- **状態管理**: マイグレーション間でスキーマ状態を追跡
  - `ProjectState`: データベーススキーマの完全な状態を維持
  - `ModelState`: 個別のモデル定義を表現
  - `FieldState`: フィールド設定を追跡
  - インデックスと制約のサポート

- **自動検出**: スキーマ変更を自動的に検出
  - `MigrationAutodetector`: 状態間の差分を検出
  - モデルの作成/削除検出
  - フィールドの追加/削除/変更検出
  - モデルとフィールドのスマートなリネーム検出
  - インデックスと制約の変更検出

- **マイグレーション実行**
  - `MigrationExecutor`: SQLiteデータベースへのマイグレーション適用
  - `DatabaseMigrationExecutor`: 複数データベースのサポート (PostgreSQL, MySQL, SQLite)
  - トランザクションサポートとロールバック機能
  - 適用済みマイグレーションを追跡するマイグレーションレコーダー

- **マイグレーション管理**
  - `MigrationLoader`: ディスクからマイグレーションを読み込み
  - `MigrationWriter`: Rustマイグレーションファイルを生成
  - マイグレーションファイルのシリアライゼーション (JSON形式)
  - 依存関係の追跡と検証

- **CLIコマンド**
  - `makemigrations`: モデル変更からマイグレーションを生成
    - 変更をプレビューするドライランモード
    - カスタムマイグレーション名
    - アプリ固有のマイグレーション生成
  - `migrate`: データベースにマイグレーションを適用
    - フェイクマイグレーションのサポート
    - マイグレーションプランのプレビュー

- **データベースバックエンドのサポート**
  - sqlxによるSQLiteサポート
  - reinhardt-databaseによるPostgreSQLサポート
  - reinhardt-databaseによるMySQLサポート
  - クロスデータベース互換性のためのSQL方言抽象化

- **依存性注入の統合**
  - `MigrationService`: マイグレーション用のDI互換サービス
  - `MigrationConfig`: 設定管理
  - reinhardt-diとの統合

### 予定

#### 高度な機能

- **マイグレーショングラフ**: 完全な依存関係解決システム (graph.rsのスケルトンが存在)
- **マイグレーションのスカッシュ**: パフォーマンスのために複数のマイグレーションを1つに結合
- **データマイグレーション**: 複雑なデータ変換のための組み込みサポート
- **ゼロダウンタイムマイグレーション**: サービス中断なしで安全にスキーマを変更
- **マイグレーションの最適化**: 操作の自動並べ替えと結合
- **アトミック操作**: 複雑なマイグレーションのためのより良いトランザクション処理
- **スキーマ履歴の可視化**: マイグレーション履歴のグラフィカル表現

#### 拡張された自動検出

- **フィールドデフォルトの検出**: デフォルト値の変更を自動的に検出
- **制約の検出**: CHECK、UNIQUE、FOREIGN KEY制約のより良いサポート
- **インデックスの最適化**: モデルのリレーションシップに基づくインデックス追加の提案

#### データベース固有の機能

- **PostgreSQL**: 高度な型 (JSONB、配列、カスタム型)
- **MySQL**: ストレージエンジン管理、パーティションのサポート
- **SQLite**: ALTER TABLEの制限のより良い処理

#### 開発者体験

- **対話モード**: ガイド付きマイグレーション作成
- **競合解決**: マイグレーションの競合の自動処理
- **マイグレーションテスト**: マイグレーションをテストするための組み込みツール
- **パフォーマンスプロファイリング**: マイグレーション実行時間の測定とボトルネックの特定

## 使い方

### 基本的な例

```rust
use reinhardt_migrations::{
    MigrationAutodetector, ProjectState, ModelState, FieldState,
    MakeMigrationsCommand, MakeMigrationsOptions,
};

// モデルを定義
let mut to_state = ProjectState::new();
let mut user_model = ModelState::new("myapp", "User");
user_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
user_model.add_field(FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false));
to_state.add_model(user_model);

// 変更を検出
let from_state = ProjectState::new(); // 初期マイグレーション用の空の状態
let detector = MigrationAutodetector::new(from_state, to_state);
let migrations = detector.generate_migrations();

// マイグレーションファイルを生成
let options = MakeMigrationsOptions {
    app_label: Some("myapp".to_string()),
    dry_run: false,
    ..Default::default()
};
let command = MakeMigrationsCommand::new(options);
let created_files = command.execute();
```

### 複合主キー

マイグレーションは、`CreateModel`操作を通じて複合主キーをサポートします:

```rust
use reinhardt_migrations::{
    operations::{CreateModel, FieldDefinition},
    Migration,
};

// 複合主キーを持つマイグレーションを作成
let migration = Migration::new("0001_initial", "myapp")
    .add_operation(
        CreateModel::new(
            "post_tags",
            vec![
                FieldDefinition::new("post_id", "INTEGER", true, false, None),
                FieldDefinition::new("tag_id", "INTEGER", true, false, None),
                FieldDefinition::new("description", "VARCHAR(200)", false, false, None),
            ],
        )
        .with_composite_primary_key(vec!["post_id".to_string(), "tag_id".to_string()])
    );

// これは以下のようなSQLを生成します:
// CREATE TABLE post_tags (
//     post_id INTEGER NOT NULL,
//     tag_id INTEGER NOT NULL,
//     description VARCHAR(200) NOT NULL,
//     PRIMARY KEY (post_id, tag_id)
// );
```

複数の`primary_key = true`フィールドを持つ`#[derive(Model)]`マクロを使用する場合、マイグレーションは自動的に複合主キー制約を検出して生成します:

```rust
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Clone, Debug)]
#[model(app_label = "myapp", table_name = "post_tags")]
struct PostTag {
    #[field(primary_key = true)]
    post_id: i64,

    #[field(primary_key = true)]
    tag_id: i64,

    #[field(max_length = 200)]
    description: String,
}

// マイグレーション自動検出器はこれを複合主キーとして認識し、
// composite_primary_keyを持つ適切なCreateModel操作を生成します
```

### マイグレーションの適用

```rust
use reinhardt_migrations::{
    MigrationExecutor, Migration, Operation, ColumnDefinition,
};
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    let mut executor = MigrationExecutor::new(pool);

    let migration = Migration::new("0001_initial", "myapp")
        .add_operation(Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
                ColumnDefinition::new("email", "VARCHAR(255) NOT NULL"),
            ],
            constraints: vec![],
        });

    let result = executor.apply_migrations(&[migration]).await?;
    println!("Applied migrations: {:?}", result.applied);

    Ok(())
}
```

## Reinhardtフレームワークとの統合

このクレートはReinhardtフレームワークの一部であり、以下と統合されています:

- `reinhardt-database`: データベース抽象化レイヤー
- `reinhardt-di`: 依存性注入システム
- `reinhardt-orm`: オブジェクト関係マッピング (将来の統合)

## ライセンス

以下のいずれかのライセンスの下でライセンスされています:

- Apache License, Version 2.0
- MIT license

お好きな方を選択してください.
