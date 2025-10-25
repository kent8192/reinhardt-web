# reinhardt-db

Reinhardtフレームワーク向けのDjango風データベースレイヤー

## 概要

`reinhardt-db`は、Djangoの ORMに着想を得た、Reinhardtアプリケーション向けの包括的なデータベースレイヤーを提供します。データベース抽象化、オブジェクトリレーショナルマッピング、マイグレーション、コネクションプーリングなどの強力な機能を備えています。

このクレートは、複数のデータベース関連サブクレートを統合し、統一されたデータベースエクスペリエンスを提供する親クレートとして機能します。

## 機能

### 実装済み ✓

この親クレートは、以下のサブクレートから機能を再エクスポートしています：

- **Database** (`reinhardt-database`): 低レベルデータベース抽象化レイヤー
  - SQLデータベース用の統一DatabaseBackendトレイト
  - 非同期データベース操作（execute、fetch_one、fetch_all）
  - クエリビルダー（SelectBuilder、InsertBuilder、UpdateBuilder、DeleteBuilder）
  - 型安全なパラメータバインディング
  - コネクションプーリングサポート

- **ORM** (`reinhardt-orm`): オブジェクトリレーショナルマッピングシステム
  - Django風のModelトレイト
  - チェイン可能なクエリのためのQuerySet API
  - フィールド型（AutoField、CharField、IntegerField、DateTimeField等）
  - TimestampedとSoftDeletableトレイト
  - リレーションシップ管理
  - バリデータとchoices

- **Migrations** (`reinhardt-migrations`): スキーママイグレーションシステム
  - モデル変更からの自動マイグレーション生成
  - 前方及び後方マイグレーション
  - スキーマバージョニングと依存関係管理
  - マイグレーション操作（CreateModel、AddField、AlterField等）
  - 状態管理と自動検出

- **Pool** (`reinhardt-pool`): コネクションプール管理
  - データベースコネクションプーリング
  - コネクションライフサイクル管理
  - プール設定とサイジング

- **Hybrid** (`reinhardt-hybrid`): ハイブリッドデータベースサポート
  - マルチデータベースルーティング
  - 読み書き分離
  - データベースシャーディングサポート

- **Associations** (`reinhardt-associations`): リレーションシップ管理
  - 外部キーリレーションシップ
  - 多対多リレーションシップ
  - 一対一リレーションシップ
  - 遅延読み込みと即時読み込み

### 実装済み ✓ (追加機能)

- **高度なクエリ最適化**
  - キャッシュヒット/ミス追跡機能付きクエリ結果キャッシング
  - クエリプラン分析と最適化
  - SELECT DISTINCT最適化
  - EXISTSとINサブクエリの最適化
  - カーソルベースページネーション（OFFSETより効率的）
  - 一括操作（バルク作成、バルク更新）
  - select_relatedとprefetch_relatedによるN+1クエリ防止
  - 遅延クエリ評価
  - データ転送量削減のためのOnly/Deferフィールド最適化
  - 集約プッシュダウン最適化

- **強化されたトランザクション管理**
  - セーブポイントサポート付きネストトランザクション
  - 分離レベル制御（ReadUncommitted、ReadCommitted、RepeatableRead、Serializable）
  - 名前付きセーブポイント（セーブポイントの作成、解放、ロールバック）
  - トランザクション状態追跡（NotStarted、Active、Committed、RolledBack）
  - 分散トランザクション向け2フェーズコミット（2PC）
  - アトミックトランザクションラッパー（Django風のtransaction.atomic）
  - データベースレベルのトランザクション実行メソッド

- **データベースレプリケーションとルーティング**
  - DatabaseRouter経由の読み書き分離
  - モデルベースのデータベースルーティングルール
  - 設定可能なデフォルトデータベース
  - モデルごとの読み取り・書き込みデータベース設定
  - hybridモジュールによるマルチデータベースサポート

### 予定

- 追加のデータベースバックエンド（MongoDB、CockroachDB等）

## インストール

`Cargo.toml`に以下を追加してください：

```toml
[dependencies]
reinhardt-db = "0.1.0"
```

### オプション機能

必要に応じて特定の機能を有効化してください：

```toml
[dependencies]
reinhardt-db = { version = "0.1.0", features = ["postgres", "orm", "migrations"] }
```

利用可能な機能：

- `database` (デフォルト): 低レベルデータベースレイヤー
- `backends` (デフォルト): バックエンド実装
- `pool` (デフォルト): コネクションプーリング
- `orm` (デフォルト): ORM機能
- `migrations` (デフォルト): マイグレーションシステム
- `hybrid` (デフォルト): マルチデータベースサポート
- `associations` (デフォルト): リレーションシップ管理
- `postgres`: PostgreSQLサポート
- `sqlite`: SQLiteサポート
- `mysql`: MySQLサポート
- `all-databases`: すべてのデータベースバックエンド

## 使用方法

### モデルの定義

```rust
use reinhardt_db::{Model, CharField, IntegerField, DateTimeField};

#[derive(Model)]
struct User {
    #[primary_key]
    id: i32,
    username: CharField<50>,
    email: CharField<254>,
    age: IntegerField,
    created_at: DateTimeField,
}
```

### QuerySetでのクエリ

```rust
use reinhardt_db::{QuerySet, Model};

// すべてのユーザーを取得
let users = User::objects().all().await?;

// ユーザーをフィルタリング
let adults = User::objects()
    .filter("age__gte", 18)
    .order_by("-created_at")
    .all()
    .await?;

// 単一のユーザーを取得
let user = User::objects()
    .filter("username", "john")
    .first()
    .await?;
```

### マイグレーションの作成

```rust
use reinhardt_db::{Migration, CreateModel, AddField};

// 新しいマイグレーションを作成
let migration = Migration::new("0001_initial")
    .add_operation(CreateModel {
        name: "User",
        fields: vec![
            ("id", "AutoField"),
            ("username", "CharField(max_length=50)"),
            ("email", "EmailField"),
        ],
    });

// マイグレーションを適用
migration.apply(db).await?;
```

### コネクションプーリング

```rust
use reinhardt_db::Pool;

// コネクションプールを作成
let pool = Pool::new("postgres://user:pass@localhost/db")
    .max_connections(10)
    .build()
    .await?;

// コネクションを取得
let conn = pool.get().await?;
```

## サブクレート

この親クレートは以下のサブクレートを含んでいます：

```
reinhardt-db/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    ├── backends/       # バックエンド実装
    ├── backends-pool/  # プールバックエンド抽象化
    ├── database/       # 低レベルデータベースレイヤー
    ├── pool/           # コネクションプーリング
    ├── orm/            # ORMシステム
    ├── migrations/     # マイグレーションシステム
    ├── hybrid/         # マルチデータベースサポート
    └── associations/   # リレーションシップ管理
```

## サポートされるデータベース

- PostgreSQL
- MySQL
- SQLite

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
