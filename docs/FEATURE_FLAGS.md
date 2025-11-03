# Feature Flags Guide

## 目次

- [概要](#概要)
- [基本的な使用方法](#基本的な使用方法)
- [バンドルFeature](#バンドルfeature)
  - [Minimal - マイクロサービス向け](#minimal---マイクロサービス向け)
  - [Standard - 標準構成](#standard---標準構成)
  - [Full - フル機能](#full---フル機能)
  - [プリセット構成](#プリセット構成)
- [機能別Feature Flag](#機能別feature-flag)
  - [データベース](#データベース)
  - [認証](#認証)
  - [キャッシュ](#キャッシュ)
  - [API機能](#api機能)
  - [ミドルウェア](#ミドルウェア)
  - [その他の機能](#その他の機能)
- [主要クレートのFeature Flag](#主要クレートのfeature-flag)
- [Feature Flag依存関係マップ](#feature-flag依存関係マップ)
- [使用例とベストプラクティス](#使用例とベストプラクティス)
- [ビルド時間とバイナリサイズの比較](#ビルド時間とバイナリサイズの比較)
- [トラブルシューティング](#トラブルシューティング)
- [Quick Reference](#quick-reference)

---

## 概要

Reinhardtは**非常に細粒度なfeature flagシステム**を採用しており、必要な機能のみを選択してビルドできます。これにより、以下のような利点があります:

### 利点

- **コンパイル時間の短縮**: 不要な機能を除外することで、ビルド時間を大幅に短縮
- **バイナリサイズの削減**: 使用しない機能のコードが含まれないため、実行ファイルサイズが小さくなる
- **依存関係の最小化**: 必要な外部クレートのみをビルドに含める
- **柔軟な構成**: マイクロサービスからフル機能アプリまで、用途に応じた最適な構成を実現

### Feature Flagの粒度

Reinhardtのfeature flagは**3段階の粒度**を持ちます:

1. **バンドルFeature**: `minimal`, `standard`, `full`などの大きなグループ
2. **機能グループFeature**: `database`, `auth`, `cache`などの機能単位
3. **個別Feature**: `jwt`, `redis-backend`, `cors`などの細かい機能単位

合計で**70以上のfeature flag**が定義されており、極めて柔軟な構成が可能です。

---

## 基本的な使用方法

### デフォルト構成（standard）

何も指定しない場合、`standard`構成が有効になります:

```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"
# これは以下と同等:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }
```

### 特定の構成を選択

```toml
[dependencies]
# minimal構成
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }

# full構成
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

### カスタム構成

```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,  # デフォルトを無効化
  features = [
    "minimal",        # ベースとなる最小構成
    "database",       # データベース機能
    "db-postgres",    # PostgreSQLサポート
    "auth-jwt",       # JWT認証
    "cache",          # キャッシュ
    "redis-backend",  # Redisバックエンド
  ]
}
```

---

## バンドルFeature

バンドルFeatureは、複数の機能をまとめて有効化する便利なプリセットです。

### Minimal - マイクロサービス向け

**Feature名**: `minimal`

**用途**: 軽量なマイクロサービスやシンプルなAPI

**有効化される機能**:
- パラメータ抽出 (`reinhardt-params`)
- 依存性注入 (`reinhardt-di`)

**バイナリサイズ**: ~5-10 MB
**コンパイル時間**: 速い

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

**適したユースケース**:
- ✅ シンプルなREST API
- ✅ マイクロサービスアーキテクチャ
- ✅ 高速な起動時間が必要な場合
- ❌ データベースアクセスが必要な場合
- ❌ 複雑な認証が必要な場合

---

### Standard - 標準構成

**Feature名**: `standard` (デフォルト)

**用途**: ほとんどのプロジェクトに適したバランス型構成

**有効化される機能**:
- `minimal`のすべて
- ORM (`reinhardt-orm`)
- シリアライザ (`reinhardt-serializers`)
- ViewSets (`reinhardt-viewsets`)
- 認証 (`reinhardt-auth`)
- ミドルウェア (`reinhardt-middleware`)
- ページネーション (`reinhardt-pagination`)
- フィルタリング (`reinhardt-filters`)
- スロットリング (`reinhardt-throttling`)
- シグナル (`reinhardt-signals`)
- パーサ (`reinhardt-parsers`)
- レンダラ (`reinhardt-renderers`)
- バージョニング (`reinhardt-versioning`)
- メタデータ (`reinhardt-metadata`)
- コンテンツネゴシエーション (`reinhardt-negotiation`)
- REST APIコア (`reinhardt-rest`)

**バイナリサイズ**: ~20-30 MB
**コンパイル時間**: 中程度

**使用例**:
```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"
# または明示的に
reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }
```

**適したユースケース**:
- ✅ 一般的なREST API
- ✅ データベースを使用するアプリケーション
- ✅ 認証が必要なAPI
- ✅ ページネーションやフィルタリングが必要なAPI
- ⚠️ GraphQLやWebSocketは含まれない（別途有効化が必要）

---

### Full - フル機能

**Feature名**: `full`

**用途**: Django風のバッテリー同梱型、全機能を使用

**有効化される機能**:
- `standard`のすべて
- データベース (`database`)
- 管理画面 (`admin`)
- GraphQL (`graphql`)
- WebSocket (`websockets`)
- キャッシュ (`cache`)
- 国際化 (`i18n`)
- メール送信 (`mail`)
- セッション管理 (`sessions`)
- 静的ファイル配信 (`static-files`)
- ストレージシステム (`storage`)
- Contribアプリ (`contrib`)

**バイナリサイズ**: ~50+ MB
**コンパイル時間**: 遅い

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

**適したユースケース**:
- ✅ 大規模なWebアプリケーション
- ✅ 複雑な要件を持つシステム
- ✅ GraphQLとREST APIの両方を提供
- ✅ リアルタイム機能（WebSocket）が必要
- ✅ 多言語対応が必要
- ❌ マイクロサービス（オーバースペック）
- ❌ コンパイル時間を最小化したい場合

---

### プリセット構成

特定のユースケースに最適化されたプリセット構成も用意されています。

#### api-only - REST API専用

テンプレートやフォームが不要なREST API専用構成。

**有効化される機能**:
- `minimal`のすべて
- シリアライザ、ViewSets、認証
- パーサ、レンダラ、バージョニング
- メタデータ、コンテンツネゴシエーション
- REST APIコア
- ページネーション、フィルタリング、スロットリング

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["api-only"] }
```

#### graphql-server - GraphQLサーバー

GraphQL API中心のサーバー構成。

**有効化される機能**:
- `minimal`のすべて
- GraphQL (`reinhardt-graphql`)
- 認証 (`reinhardt-auth`)
- データベース (`database`)

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["graphql-server"] }
```

#### websocket-server - WebSocketサーバー

リアルタイム通信中心のサーバー構成。

**有効化される機能**:
- `minimal`のすべて
- WebSocket (`reinhardt-websockets`)
- 認証 (`reinhardt-auth`)
- キャッシュ (`reinhardt-cache`)

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["websocket-server"] }
```

#### cli-tools - CLI/バックグラウンドジョブ

CLIツールやバックグラウンド処理向け構成。

**有効化される機能**:
- データベース (`database`)
- マイグレーション (`reinhardt-migrations`)
- タスク (`reinhardt-tasks`)
- メール送信 (`reinhardt-mail`)

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["cli-tools"] }
```

#### test-utils - テストユーティリティ

テスト環境向け構成。

**有効化される機能**:
- テストユーティリティ (`reinhardt-test`)
- データベース (`database`)

**使用例**:
```toml
[dev-dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["test-utils"] }
```

---

## 機能別Feature Flag

### データベース

#### database

データベース機能全般を有効化。

**有効化されるクレート**:
- `reinhardt-orm` - ORM機能
- `reinhardt-migrations` - マイグレーション
- `reinhardt-contenttypes` - コンテンツタイプ
- `reinhardt-db` - データベース基盤

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database"] }
```

#### データベース固有のFeature

特定のデータベースサポートを有効化:

| Feature | データベース | 説明 |
|---------|------------|------|
| `db-postgres` | PostgreSQL | PostgreSQLサポート |
| `db-mysql` | MySQL | MySQLサポート |
| `db-sqlite` | SQLite | SQLiteサポート（軽量、ファイルベース） |
| `db-mongodb` | MongoDB | MongoDBサポート（NoSQL） |
| `db-cockroachdb` | CockroachDB | CockroachDBサポート（分散SQL） |

**使用例**:
```toml
[dependencies]
# PostgreSQL使用
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database", "db-postgres"] }

# 複数データベース対応
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database", "db-postgres", "db-sqlite"] }
```

**注意**:
- `database` featureは自動的にPostgreSQLを有効化します（`reinhardt-db`のデフォルト）
- 他のデータベースを使用する場合は、明示的に対応するfeatureを指定

---

### 認証

#### auth

基本的な認証機能を有効化。

**有効化されるクレート**:
- `reinhardt-auth` - 認証基盤

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "auth"] }
```

#### 認証方式別のFeature

特定の認証方式を有効化:

| Feature | 認証方式 | 説明 |
|---------|----------|------|
| `auth-jwt` | JWT | JSON Web Token認証 |
| `auth-session` | Session | セッションベース認証 |
| `auth-oauth` | OAuth | OAuth認証 |
| `auth-token` | Token | トークン認証 |

**使用例**:
```toml
[dependencies]
# JWT認証のみ
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "auth-jwt"] }

# JWT + セッション認証
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "auth-jwt", "auth-session"] }
```

**注意**:
- 個別の認証方式feature（`auth-jwt`など）は自動的に`auth`を有効化
- `auth-session`は自動的に`sessions` featureも有効化

---

### キャッシュ

#### cache

キャッシュ機能の基盤を有効化。

**有効化されるクレート**:
- `reinhardt-cache` - キャッシュシステム

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "cache"] }
```

#### キャッシュバックエンド別のFeature

| Feature | バックエンド | 説明 |
|---------|-------------|------|
| `redis-backend` | Redis | Redisキャッシュバックエンド |
| `redis-cluster` | Redis Cluster | Redisクラスタ対応 |
| `redis-sentinel` | Redis Sentinel | Redisセンチネル対応 |
| `memcached-backend` | Memcached | Memcachedバックエンド |

**使用例**:
```toml
[dependencies]
# Redisキャッシュ
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "cache", "redis-backend"] }

# Redisクラスタ対応
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "cache", "redis-backend", "redis-cluster"] }
```

**依存関係**:
- 外部クレート: `redis`, `deadpool-redis` (Redis使用時)
- 外部クレート: `memcache-async`, `tokio-util` (Memcached使用時)

---

### API機能

#### api

API関連の基本機能を有効化。

**有効化されるクレート**:
- `reinhardt-serializers` - シリアライザ
- `reinhardt-viewsets` - ViewSets

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "api"] }
```

#### シリアライゼーション形式

| Feature | 形式 | 説明 |
|---------|------|------|
| `serialize-json` | JSON | JSON形式（デフォルトで有効） |
| `serialize-xml` | XML | XML形式 |
| `serialize-yaml` | YAML | YAML形式 |

**使用例**:
```toml
[dependencies]
# JSON + YAML対応
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "serialize-yaml"] }
```

**注意**:
- `serialize-json`は`reinhardt-serializers`のデフォルトで有効
- XML/YAMLを使用する場合は明示的に指定が必要

---

### ミドルウェア

#### middleware

基本的なミドルウェア機能を有効化。

**有効化されるクレート**:
- `reinhardt-middleware` - ミドルウェア基盤

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "middleware"] }
```

**注意**: `middleware`は自動的に`sessions`も有効化します。

#### ミドルウェア個別機能

特定のミドルウェア機能のみを有効化:

| Feature | 機能 | 説明 |
|---------|------|------|
| `middleware-cors` | CORS | Cross-Origin Resource Sharing |
| `middleware-compression` | 圧縮 | レスポンス圧縮（gzip等） |
| `middleware-security` | セキュリティ | セキュリティヘッダー等 |
| `middleware-rate-limit` | レート制限 | リクエスト数制限 |

**使用例**:
```toml
[dependencies]
# CORS + レート制限のみ
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "middleware-cors", "middleware-rate-limit"] }
```

---

### その他の機能

#### admin - 管理画面

Django風の自動生成管理画面。

**有効化されるクレート**:
- `reinhardt-forms` - フォーム処理
- `reinhardt-template` - テンプレートエンジン

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "admin"] }
```

**注意**: `reinhardt-admin`クレートは現在開発中のため、`admin` featureから除外されています。

---

#### graphql - GraphQL

GraphQL APIサポート。

**有効化されるクレート**:
- `reinhardt-graphql` - GraphQLスキーマとリゾルバ

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "graphql"] }
```

**機能**:
- GraphQLスキーマ生成
- リゾルバ定義
- サブスクリプション対応

---

#### websockets - WebSocket

リアルタイム双方向通信。

**有効化されるクレート**:
- `reinhardt-websockets` - WebSocketサーバー

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "websockets"] }
```

**機能**:
- WebSocketチャネル
- ルーム管理
- 認証統合
- Redis統合（pub/sub）

---

#### i18n - 国際化

多言語対応。

**有効化されるクレート**:
- `reinhardt-i18n` - 翻訳カタログとロケール管理

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "i18n"] }
```

**機能**:
- 翻訳カタログ（gettext形式）
- ロケール切り替え
- 複数形対応
- タイムゾーン対応

---

#### mail - メール送信

メール送信機能。

**有効化されるクレート**:
- `reinhardt-mail` - メール送信とテンプレート

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "mail"] }
```

**機能**:
- SMTP送信
- テンプレートメール
- 添付ファイル
- HTMLメール

---

#### sessions - セッション管理

セッション管理機能。

**有効化されるクレート**:
- `reinhardt-sessions` - セッションストレージ

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "sessions"] }
```

**機能**:
- 複数のバックエンド（データベース、ファイル、Cookie、JWT）
- セキュアなセッションID生成
- セッションミドルウェア統合

---

#### static-files - 静的ファイル配信

静的ファイルの配信と管理。

**有効化されるクレート**:
- `reinhardt-static` - 静的ファイルハンドラ

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "static-files"] }
```

**機能**:
- CDN統合
- ハッシュ化ストレージ
- 圧縮対応
- キャッシュ制御

---

#### storage - ストレージシステム

ファイルストレージの抽象化。

**有効化されるクレート**:
- `reinhardt-storage` - ストレージバックエンド

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "storage"] }
```

**機能**:
- ローカルファイルシステム
- S3互換ストレージ
- ストレージバックエンドの切り替え

---

#### tasks - タスク/バックグラウンドジョブ

非同期タスク処理。

**有効化されるクレート**:
- `reinhardt-tasks` - タスクキューとワーカー

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "tasks"] }
```

**機能**:
- タスクキュー
- スケジュール実行
- リトライ機能
- バックグラウンドワーカー

---

#### shortcuts - Django風ショートカット

Django風の便利関数。

**有効化されるクレート**:
- `reinhardt-shortcuts` - ショートカット関数

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "shortcuts"] }
```

**機能**:
- `get_object_or_404()` - オブジェクト取得または404エラー
- `redirect()` - リダイレクト
- `render()` - テンプレートレンダリング

---

#### contrib - Contribアプリ集約

すべてのcontribアプリを一括有効化。

**有効化されるクレート**:
- `reinhardt-contrib` - contrib集約クレート（auth, contenttypes, sessions, messages, static, mail, graphql, websockets, i18n）

**使用例**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "contrib"] }
```

**注意**: 個別のcontrib機能を有効化することも可能（`reinhardt-contrib`クレート内のfeature flag参照）。

---

## 主要クレートのFeature Flag

### reinhardt-micro

**目的**: 軽量なマイクロサービス向け構成

**デフォルト**: `["routing", "params", "di"]`

**利用可能なFeature**:

| Feature | 説明 | 依存関係 |
|---------|------|----------|
| `routing` | ルーティング機能 | reinhardt-routers |
| `params` | パラメータ抽出 | reinhardt-params |
| `di` | 依存性注入 | reinhardt-di |
| `database` | データベース対応 | reinhardt-db |
| `compression` | 圧縮ミドルウェア | - |
| `cors` | CORSミドルウェア | - |
| `rate-limit` | レート制限 | - |
| `security` | セキュリティミドルウェア | - |

**一時的に無効化されている機能**:
- `schema` (OpenAPIスキーマ生成) - utoipa API互換性対応中

**使用例**:
```toml
[dependencies]
reinhardt-micro = { version = "0.1.0-alpha.1", features = ["routing", "params", "di", "database"] }
```

---

### reinhardt-db

**目的**: データベース層の統合クレート

**デフォルト**: `["backends", "pool", "postgres", "orm", "migrations", "hybrid", "associations"]`

**利用可能なFeature**:

#### モジュールFeature

| Feature | 説明 | 有効化されるクレート |
|---------|------|---------------------|
| `backends` | バックエンド実装 | reinhardt-backends |
| `pool` | コネクションプール | reinhardt-backends-pool, reinhardt-pool, reinhardt-di |
| `orm` | ORM機能 | reinhardt-orm |
| `migrations` | マイグレーション | reinhardt-migrations |
| `hybrid` | ハイブリッド機能 | reinhardt-hybrid |
| `associations` | 関連機能 | reinhardt-associations |

#### データベースFeature

| Feature | データベース | 依存クレート |
|---------|------------|-------------|
| `postgres` | PostgreSQL | sqlx/postgres, tokio-postgres |
| `sqlite` | SQLite | sqlx/sqlite, rusqlite |
| `mysql` | MySQL | sqlx/mysql, mysql_async |
| `mongodb-backend` | MongoDB | mongodb, tokio |
| `cockroachdb-backend` | CockroachDB | 同postgres（プロトコル互換） |
| `all-databases` | 全データベース | 上記すべて |

**使用例**:
```toml
[dependencies]
# PostgreSQLのみ（デフォルト）
reinhardt-db = "0.1.0-alpha.1"

# SQLiteとPostgreSQL
reinhardt-db = { version = "0.1.0-alpha.1", features = ["postgres", "sqlite"] }

# 全データベース対応
reinhardt-db = { version = "0.1.0-alpha.1", features = ["all-databases"] }
```

**注意**:
- `pool` featureは自動的に`reinhardt-di`を有効化（DI統合のため）
- デフォルトでPostgreSQLが有効（最も一般的なため）

---

### reinhardt-auth

**目的**: 認証システム

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

#### 認証方式

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `jwt` | JWT認証 | jsonwebtoken |
| `session` | セッション認証 | reinhardt-sessions |
| `oauth` | OAuth認証 | oauth2 |
| `token` | トークン認証 | - |

#### ストレージ

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `database` | データベースストレージ | sqlx, sea-query, sea-query-binder |
| `redis-sessions` | Redisセッション | redis, deadpool-redis |

**使用例**:
```toml
[dependencies]
# JWT認証のみ
reinhardt-auth = { version = "0.1.0-alpha.1", features = ["jwt"] }

# JWT + データベースストレージ
reinhardt-auth = { version = "0.1.0-alpha.1", features = ["jwt", "database"] }

# すべての認証方式
reinhardt-auth = { version = "0.1.0-alpha.1", features = ["jwt", "session", "oauth", "token", "database"] }
```

---

### reinhardt-sessions

**目的**: セッション管理

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `database` | データベースバックエンド | reinhardt-orm, reinhardt-db, sea-query, sea-query-binder |
| `file` | ファイルバックエンド | tokio, fs2 |
| `cookie` | Cookieベースセッション | base64, aes-gcm, rand, hmac, sha2 |
| `jwt` | JWTセッション | jsonwebtoken |
| `middleware` | HTTPミドルウェア統合 | reinhardt-http, reinhardt-types, reinhardt-exception, bytes |
| `messagepack` | MessagePackシリアライゼーション | rmp-serde |

**使用例**:
```toml
[dependencies]
# データベースセッション + ミドルウェア
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["database", "middleware"] }

# Cookieベースセッション
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["cookie", "middleware"] }

# すべてのバックエンド
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["database", "file", "cookie", "jwt", "middleware"] }
```

**バックエンドの選択**:
- `database`: 大規模アプリ、複数サーバー対応
- `file`: 開発環境、小規模アプリ
- `cookie`: ステートレス、サーバー側ストレージ不要
- `jwt`: API向け、トークンベース

---

### reinhardt-cache

**目的**: キャッシュシステム

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `redis-backend` | Redisバックエンド | redis, deadpool-redis |
| `redis-cluster` | Redisクラスタ | 同上 |
| `redis-sentinel` | Redisセンチネル | 同上 |
| `memcached-backend` | Memcachedバックエンド | memcache-async, tokio-util |
| `all-backends` | すべてのバックエンド | 上記すべて |

**使用例**:
```toml
[dependencies]
# Redis単体
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-backend"] }

# Redisクラスタ対応
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-backend", "redis-cluster"] }

# RedisとMemcached両対応
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-backend", "memcached-backend"] }
```

---

### reinhardt-middleware

**目的**: HTTPミドルウェア

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

| Feature | 説明 | 機能 |
|---------|------|------|
| `cors` | CORSミドルウェア | クロスオリジンリクエスト制御 |
| `compression` | 圧縮ミドルウェア | gzip/brotli圧縮 |
| `security` | セキュリティミドルウェア | セキュリティヘッダー設定 |
| `rate-limit` | レート制限 | リクエスト数制限 |
| `session` | セッションミドルウェア | セッション管理統合 |
| `sqlx` | SQLxデータベース対応 | データベース接続管理 |

**使用例**:
```toml
[dependencies]
# CORS + 圧縮
reinhardt-middleware = { version = "0.1.0-alpha.1", features = ["cors", "compression"] }

# すべてのミドルウェア
reinhardt-middleware = { version = "0.1.0-alpha.1", features = ["cors", "compression", "security", "rate-limit", "session"] }
```

---

### reinhardt-serializers

**目的**: データのシリアライゼーション

**デフォルト**: `["json"]`

**利用可能なFeature**:

| Feature | 形式 | 依存クレート |
|---------|------|-------------|
| `json` | JSON | serde_json |
| `xml` | XML | quick-xml, serde-xml-rs |
| `yaml` | YAML | serde_yaml |

**使用例**:
```toml
[dependencies]
# JSONのみ（デフォルト）
reinhardt-serializers = "0.1.0-alpha.1"

# JSON + YAML
reinhardt-serializers = { version = "0.1.0-alpha.1", features = ["json", "yaml"] }

# すべての形式
reinhardt-serializers = { version = "0.1.0-alpha.1", features = ["json", "xml", "yaml"] }
```

---

### reinhardt-rest

**目的**: REST APIコア機能

**デフォルト**: `["serializers", "parsers", "renderers"]`

**利用可能なFeature**:

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `serializers` | シリアライザ | reinhardt-orm |
| `parsers` | パーサ | reinhardt-parsers |
| `renderers` | レンダラ | reinhardt-renderers |
| `jwt` | JWTサポート | rest-core/jwt |

**使用例**:
```toml
[dependencies]
# デフォルト構成
reinhardt-rest = "0.1.0-alpha.1"

# JWT付き
reinhardt-rest = { version = "0.1.0-alpha.1", features = ["serializers", "parsers", "renderers", "jwt"] }
```

**注意**: `serializers` featureは`reinhardt-orm`を依存関係に含みます。

---

### reinhardt-contrib

**目的**: Contribアプリの集約

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

| Feature | 説明 | 有効化されるクレート |
|---------|------|---------------------|
| `auth` | 認証 | reinhardt-auth |
| `contenttypes` | コンテンツタイプ | reinhardt-contenttypes |
| `sessions` | セッション | reinhardt-sessions |
| `messages` | メッセージ | reinhardt-messages |
| `static` | 静的ファイル | reinhardt-static |
| `mail` | メール | reinhardt-mail |
| `graphql` | GraphQL | reinhardt-graphql |
| `websockets` | WebSocket | reinhardt-websockets |
| `i18n` | 国際化 | reinhardt-i18n |
| `full` | すべて | 上記すべて |

**使用例**:
```toml
[dependencies]
# 個別機能
reinhardt-contrib = { version = "0.1.0-alpha.1", features = ["auth", "sessions"] }

# すべての機能
reinhardt-contrib = { version = "0.1.0-alpha.1", features = ["full"] }
```

---

### reinhardt-di

**目的**: 依存性注入システム

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `params` | パラメータ抽出 | reinhardt-params |
| `dev-tools` | 開発ツール | indexmap |
| `generator` | ジェネレータ機能 | genawaiter |

**使用例**:
```toml
[dependencies]
# パラメータ抽出付き
reinhardt-di = { version = "0.1.0-alpha.1", features = ["params"] }

# すべての機能
reinhardt-di = { version = "0.1.0-alpha.1", features = ["params", "dev-tools", "generator"] }
```

---

### reinhardt-test

**目的**: テストユーティリティ

**デフォルト**: なし（すべてオプション）

**利用可能なFeature**:

| Feature | 説明 | 依存クレート |
|---------|------|-------------|
| `testcontainers` | TestContainers統合 | testcontainers, testcontainers-modules, sqlx, memcache-async, tokio-util |
| `static` | 静的ファイルテスト | reinhardt-static |

**使用例**:
```toml
[dev-dependencies]
# TestContainers統合（データベース/キャッシュテスト用）
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers"] }

# すべてのテストユーティリティ
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers", "static"] }
```

**TestContainersの用途**:
- 実際のPostgreSQL/MySQL/SQLiteコンテナでのテスト
- Redisコンテナでのキャッシュテスト
- Memcachedコンテナでのキャッシュテスト

---

## Feature Flag依存関係マップ

### バンドルFeatureの依存関係

```
default
└── standard
    ├── minimal
    │   ├── reinhardt-params
    │   └── reinhardt-di
    ├── reinhardt-orm
    ├── reinhardt-serializers
    ├── reinhardt-viewsets
    ├── reinhardt-auth
    ├── reinhardt-middleware
    ├── reinhardt-pagination
    ├── reinhardt-filters
    ├── reinhardt-throttling
    ├── reinhardt-signals
    ├── reinhardt-parsers
    ├── reinhardt-renderers
    ├── reinhardt-versioning
    ├── reinhardt-metadata
    ├── reinhardt-negotiation
    └── reinhardt-rest

full
├── standard (上記すべて)
├── database
│   ├── reinhardt-orm
│   ├── reinhardt-migrations
│   ├── reinhardt-contenttypes
│   └── reinhardt-db
│       ├── backends
│       ├── pool (→ reinhardt-di)
│       ├── postgres
│       ├── orm
│       ├── migrations
│       ├── hybrid
│       └── associations
├── auth → reinhardt-auth
├── admin
│   ├── reinhardt-forms
│   └── reinhardt-template
├── graphql → reinhardt-graphql
├── websockets → reinhardt-websockets
├── cache → reinhardt-cache
├── i18n → reinhardt-i18n
├── mail → reinhardt-mail
├── sessions → reinhardt-sessions
├── static-files → reinhardt-static
├── storage → reinhardt-storage
└── contrib → reinhardt-contrib
```

### データベースFeatureの依存関係

```
database
├── reinhardt-orm
├── reinhardt-migrations
├── reinhardt-contenttypes
└── reinhardt-db (default features enabled)
    ├── backends
    ├── pool
    │   ├── reinhardt-backends-pool
    │   ├── reinhardt-pool
    │   └── reinhardt-di (自動有効化)
    ├── postgres (default)
    ├── orm
    ├── migrations
    ├── hybrid
    └── associations

db-postgres
├── database
└── reinhardt-db/postgres

db-mysql
├── database
└── reinhardt-db/mysql

db-sqlite
├── database
└── reinhardt-db/sqlite

db-mongodb
├── database
└── reinhardt-db/mongodb-backend

db-cockroachdb
├── database
└── reinhardt-db/cockroachdb-backend
```

### 認証Featureの依存関係

```
auth
└── reinhardt-auth

auth-jwt
├── auth
└── reinhardt-auth/jwt
    └── jsonwebtoken

auth-session
├── auth
├── reinhardt-auth/session
└── sessions (自動有効化)
    └── reinhardt-sessions

auth-oauth
├── auth
└── reinhardt-auth/oauth
    └── oauth2

auth-token
├── auth
└── reinhardt-auth/token
```

### キャッシュFeatureの依存関係

```
cache
└── reinhardt-cache

redis-backend
├── cache
└── reinhardt-cache/redis-backend
    ├── redis
    └── deadpool-redis

redis-cluster
├── redis-backend
└── reinhardt-cache/redis-cluster

redis-sentinel
├── redis-backend
└── reinhardt-cache/redis-sentinel

memcached-backend
├── cache
└── reinhardt-cache/memcached-backend
    ├── memcache-async
    └── tokio-util
```

### ミドルウェアFeatureの依存関係

```
middleware
├── reinhardt-middleware
└── sessions (自動有効化)

middleware-cors
└── reinhardt-middleware/cors

middleware-compression
└── reinhardt-middleware/compression

middleware-security
└── reinhardt-middleware/security

middleware-rate-limit
└── reinhardt-middleware/rate-limit

middleware + session
└── reinhardt-middleware/session
    └── reinhardt-sessions
```

### 相互依存関係の重要な注意点

1. **pool → reinhardt-di**: コネクションプールのDI統合のため自動有効化
2. **middleware → sessions**: ミドルウェアがセッション機能を使用するため自動有効化
3. **auth-session → sessions**: セッション認証がセッション管理を使用するため自動有効化
4. **serializers → reinhardt-orm**: シリアライザがORMモデルを扱うため依存

---

## 使用例とベストプラクティス

### シナリオ1: シンプルなマイクロサービスAPI

**要件**:
- データベース不要
- 軽量で高速起動
- 基本的なルーティングとパラメータ抽出

**推奨構成**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

**または**:
```toml
[dependencies]
reinhardt-micro = "0.1.0-alpha.1"  # デフォルトでrouting + params + di
```

**バイナリサイズ**: ~5-10 MB
**コンパイル時間**: 1-2分

---

### シナリオ2: PostgreSQLを使用するREST API

**要件**:
- PostgreSQLデータベース
- JSON API
- JWT認証
- ページネーションとフィルタリング

**推奨構成**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = [
    "api-only",      # REST API基本機能
    "db-postgres",   # PostgreSQL対応
    "auth-jwt",      # JWT認証
  ]
}
```

**バイナリサイズ**: ~20-25 MB
**コンパイル時間**: 3-5分

---

### シナリオ3: GraphQL + WebSocketサーバー

**要件**:
- GraphQL API
- WebSocketでのリアルタイム通信
- Redisキャッシュ
- PostgreSQLデータベース

**推奨構成**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = [
    "minimal",
    "graphql",
    "websockets",
    "db-postgres",
    "cache",
    "redis-backend",
    "auth-jwt",
  ]
}
```

**バイナリサイズ**: ~30-35 MB
**コンパイル時間**: 5-7分

---

### シナリオ4: フル機能Webアプリケーション

**要件**:
- REST API + GraphQL
- WebSocket
- 管理画面
- 多言語対応
- メール送信
- 静的ファイル配信

**推奨構成**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

**バイナリサイズ**: ~50+ MB
**コンパイル時間**: 10-15分

---

### シナリオ5: CLIツール/バックグラウンドジョブ

**要件**:
- データベースマイグレーション
- メール送信バッチ
- タスクスケジューリング

**推奨構成**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = [
    "cli-tools",  # database, migrations, tasks, mail
  ]
}
```

**バイナリサイズ**: ~15-20 MB
**コンパイル時間**: 3-4分

---

### ベストプラクティス

#### 1. default-featuresの制御

**最小構成から始める**:
```toml
# ❌ 悪い例: 不要な機能が含まれる
[dependencies]
reinhardt = "0.1.0-alpha.1"  # standardがすべて有効化

# ✅ 良い例: 必要な機能のみ選択
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = ["minimal", "database", "db-postgres"]
}
```

#### 2. データベースバックエンドの明示的指定

**使用するデータベースを明示**:
```toml
# ❌ 悪い例: デフォルトのPostgreSQLが有効化される
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["database"] }

# ✅ 良い例: 使用するデータベースを明示
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = ["database", "db-sqlite"]  # SQLiteを明示
}
```

#### 3. 開発環境と本番環境の分離

**環境ごとにfeatureを切り替え**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false }

[features]
# 開発環境: テストユーティリティを含む
dev = ["reinhardt/standard", "reinhardt/test-utils"]

# 本番環境: 最小構成
prod = ["reinhardt/minimal", "reinhardt/database", "reinhardt/db-postgres"]
```

ビルド時:
```bash
# 開発環境
cargo build --features dev

# 本番環境
cargo build --release --features prod
```

#### 4. キャッシュバックエンドの適切な選択

**用途に応じたバックエンド選択**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = [
    "cache",
    # 開発環境: Memcached（簡単セットアップ）
    "memcached-backend",

    # 本番環境: Redis Cluster（高可用性）
    # "redis-backend",
    # "redis-cluster",
  ]
}
```

#### 5. テスト用の構成

**dev-dependenciesでテストユーティリティを追加**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = ["minimal", "database"]
}

[dev-dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = ["test-utils"]
}
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers"] }
```

---

## ビルド時間とバイナリサイズの比較

### 構成別の比較表

| 構成 | Features | コンパイル時間 | バイナリサイズ | 推奨用途 |
|------|----------|---------------|---------------|----------|
| **Minimal** | `minimal` | 1-2分 | ~5-10 MB | マイクロサービス、シンプルAPI |
| **Minimal + DB** | `minimal`, `database`, `db-postgres` | 2-3分 | ~15-20 MB | データベース使用の小規模API |
| **API Only** | `api-only`, `db-postgres` | 3-4分 | ~20-25 MB | REST API専用 |
| **Standard** | `standard` (デフォルト) | 5-7分 | ~25-30 MB | 一般的なWebアプリ |
| **Standard + Extra** | `standard`, `graphql`, `cache` | 7-9分 | ~35-40 MB | REST + GraphQL + キャッシュ |
| **Full** | `full` | 10-15分 | ~50+ MB | フル機能Webアプリ |

### データベースバックエンド別の影響

| データベース | 追加コンパイル時間 | 追加バイナリサイズ |
|-------------|-------------------|-------------------|
| PostgreSQL | +30秒 | +2-3 MB |
| MySQL | +30秒 | +2-3 MB |
| SQLite | +10秒 | +1 MB |
| MongoDB | +1分 | +4-5 MB |
| 全データベース | +2分 | +8-10 MB |

### キャッシュバックエンド別の影響

| キャッシュ | 追加コンパイル時間 | 追加バイナリサイズ |
|-----------|-------------------|-------------------|
| Redis | +20秒 | +1-2 MB |
| Memcached | +15秒 | +1 MB |
| Redis Cluster | +30秒 | +2 MB |

### 計測環境

- **CPU**: Apple M1/M2またはIntel Core i5以上
- **メモリ**: 16GB以上
- **Rustバージョン**: 1.70以上
- **ビルドモード**: `--release`

**注意**: 実際のコンパイル時間とバイナリサイズは、ハードウェア、Rustバージョン、依存関係のキャッシュ状態により変動します。

---

## トラブルシューティング

### 問題1: コンパイルエラー「feature not found」

**エラーメッセージ例**:
```
error: feature `foo` is not available in package `reinhardt`
```

**原因**: 存在しないfeature名を指定している

**解決方法**:
1. [Quick Reference](#quick-reference)で正しいfeature名を確認
2. タイポがないかチェック（例: `databse` → `database`）
3. バージョンによる差異を確認（古いバージョンでは未実装の可能性）

---

### 問題2: 依存関係の競合

**エラーメッセージ例**:
```
error: multiple versions of `sqlx` found
```

**原因**: 複数のfeatureが異なるバージョンの同じクレートを要求

**解決方法**:
```toml
[patch.crates-io]
sqlx = { git = "https://github.com/launchbadge/sqlx", branch = "main" }
```

または、Cargo.lockを削除して再ビルド:
```bash
rm Cargo.lock
cargo build
```

---

### 問題3: リンカエラー

**エラーメッセージ例**:
```
error: linking with `cc` failed
```

**原因**: データベースドライバの共有ライブラリが見つからない

**解決方法**:

**PostgreSQL**:
```bash
# macOS
brew install postgresql

# Ubuntu/Debian
sudo apt-get install libpq-dev

# Fedora/RHEL
sudo dnf install postgresql-devel
```

**MySQL**:
```bash
# macOS
brew install mysql

# Ubuntu/Debian
sudo apt-get install libmysqlclient-dev

# Fedora/RHEL
sudo dnf install mysql-devel
```

**SQLite**:
```bash
# macOS
brew install sqlite

# Ubuntu/Debian
sudo apt-get install libsqlite3-dev

# Fedora/RHEL
sudo dnf install sqlite-devel
```

---

### 問題4: バイナリサイズが大きすぎる

**症状**: releaseビルドでも50MB以上のバイナリ

**原因**: 不要なfeatureが有効化されている

**解決方法**:

1. **使用していないfeatureを無効化**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,  # これが重要
  features = ["minimal", "database", "db-postgres"]
}
```

2. **Cargo.tomlでLTO（Link Time Optimization）を有効化**:
```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = "z"  # サイズ最適化
strip = true     # デバッグシンボル削除
```

3. **実際に使用しているfeatureを確認**:
```bash
cargo tree --features standard | grep reinhardt
```

---

### 問題5: コンパイル時間が長すぎる

**症状**: ビルドに10分以上かかる

**原因**: 不要なfeatureが有効化、またはキャッシュが効いていない

**解決方法**:

1. **並列ビルドを有効化**:
```bash
# ~/.cargo/config.toml
[build]
jobs = 8  # CPUコア数に応じて調整
```

2. **sccacheを使用（ビルドキャッシュ）**:
```bash
# インストール
cargo install sccache

# 環境変数設定
export RUSTC_WRAPPER=sccache
```

3. **不要なfeatureを無効化**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = ["minimal"]  # 必要最小限
}
```

---

### 問題6: ランタイムエラー「feature not enabled」

**エラーメッセージ例**:
```
thread 'main' panicked at 'Redis backend not enabled'
```

**原因**: コードで使用している機能のfeatureが有効化されていない

**解決方法**:

1. **エラーメッセージから必要なfeatureを特定**:
   - `Redis backend not enabled` → `redis-backend` featureが必要
   - `JWT support not enabled` → `auth-jwt` featureが必要

2. **Cargo.tomlに該当featureを追加**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = ["cache", "redis-backend"]  # 追加
}
```

---

### 問題7: TestContainersが動作しない

**症状**: テスト実行時にDockerコンテナが起動しない

**原因**: `testcontainers` featureが有効化されていない、またはDockerが起動していない

**解決方法**:

1. **Dockerが起動しているか確認**:
```bash
docker ps
```

2. **dev-dependenciesで`testcontainers` featureを有効化**:
```toml
[dev-dependencies]
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers"] }
```

3. **環境変数を設定（Podman使用時）**:
```bash
export DOCKER_HOST=unix:///run/podman/podman.sock
```

---

### デバッグのヒント

#### 1. 有効化されているfeatureを確認

```bash
# 依存関係ツリーを表示（feature付き）
cargo tree -e features

# reinhardtクレートのfeatureのみ表示
cargo tree -e features | grep reinhardt
```

#### 2. 条件付きコンパイルの確認

```rust
// コード内でfeatureの有効状態を確認
#[cfg(feature = "redis-backend")]
println!("Redis backend is enabled");

#[cfg(not(feature = "redis-backend"))]
println!("Redis backend is NOT enabled");
```

#### 3. ビルド時の詳細ログ

```bash
# ビルド時の詳細ログを表示
cargo build -vv

# 特定のクレートのビルドログのみ表示
cargo build -vv 2>&1 | grep reinhardt
```

---

## Quick Reference

### 全Feature Flag一覧表（アルファベット順）

| Feature | カテゴリ | 説明 | デフォルト |
|---------|---------|------|-----------|
| `admin` | 機能 | 管理画面（forms, template） | ❌ |
| `api` | 機能 | API基本機能（serializers, viewsets） | ❌ |
| `api-only` | バンドル | REST API専用構成 | ❌ |
| `auth` | 機能 | 認証基盤 | ❌ |
| `auth-jwt` | 認証 | JWT認証 | ❌ |
| `auth-oauth` | 認証 | OAuth認証 | ❌ |
| `auth-session` | 認証 | セッション認証 | ❌ |
| `auth-token` | 認証 | トークン認証 | ❌ |
| `cache` | 機能 | キャッシュシステム | ❌ |
| `cli-tools` | バンドル | CLI/バックグラウンドジョブ構成 | ❌ |
| `conf` | クレート | 設定管理 | ❌ |
| `contrib` | 機能 | Contribアプリ集約 | ❌ |
| `core` | クレート | コア機能 | ❌ |
| `database` | 機能 | データベース全般 | ❌ |
| `db-cockroachdb` | データベース | CockroachDBサポート | ❌ |
| `db-mongodb` | データベース | MongoDBサポート | ❌ |
| `db-mysql` | データベース | MySQLサポート | ❌ |
| `db-postgres` | データベース | PostgreSQLサポート | ❌ |
| `db-sqlite` | データベース | SQLiteサポート | ❌ |
| `default` | - | デフォルト構成（standard） | ✅ |
| `di` | クレート | 依存性注入 | ❌ |
| `di-generator` | DI | DIジェネレータ | ❌ |
| `forms` | 機能 | フォーム処理 | ❌ |
| `full` | バンドル | 全機能有効化 | ❌ |
| `graphql` | 機能 | GraphQLサポート | ❌ |
| `graphql-server` | バンドル | GraphQLサーバー構成 | ❌ |
| `i18n` | 機能 | 国際化 | ❌ |
| `mail` | 機能 | メール送信 | ❌ |
| `memcached-backend` | キャッシュ | Memcachedバックエンド | ❌ |
| `middleware` | 機能 | ミドルウェア基盤 | ❌ |
| `middleware-compression` | ミドルウェア | 圧縮ミドルウェア | ❌ |
| `middleware-cors` | ミドルウェア | CORSミドルウェア | ❌ |
| `middleware-rate-limit` | ミドルウェア | レート制限ミドルウェア | ❌ |
| `middleware-security` | ミドルウェア | セキュリティミドルウェア | ❌ |
| `minimal` | バンドル | 最小構成 | ❌ |
| `redis-backend` | キャッシュ | Redisバックエンド | ❌ |
| `redis-cluster` | キャッシュ | Redisクラスタ | ❌ |
| `redis-sentinel` | キャッシュ | Redisセンチネル | ❌ |
| `rest` | クレート | REST APIコア | ❌ |
| `serialize-json` | シリアライズ | JSON形式 | ✅ (serializers) |
| `serialize-xml` | シリアライズ | XML形式 | ❌ |
| `serialize-yaml` | シリアライズ | YAML形式 | ❌ |
| `server` | 機能 | サーバーコンポーネント | ❌ |
| `sessions` | 機能 | セッション管理 | ❌ |
| `shortcuts` | 機能 | Django風ショートカット | ❌ |
| `standard` | バンドル | 標準構成（デフォルト） | ✅ |
| `static-files` | 機能 | 静的ファイル配信 | ❌ |
| `storage` | 機能 | ストレージシステム | ❌ |
| `tasks` | 機能 | タスク/バックグラウンドジョブ | ❌ |
| `templates` | 機能 | テンプレートエンジン | ❌ |
| `test` | クレート | テストユーティリティ | ❌ |
| `test-utils` | バンドル | テスト環境構成 | ❌ |
| `websocket-server` | バンドル | WebSocketサーバー構成 | ❌ |
| `websockets` | 機能 | WebSocketサポート | ❌ |

### カテゴリ別索引

#### バンドルFeature
- `minimal`, `standard`, `full`
- `api-only`, `graphql-server`, `websocket-server`, `cli-tools`, `test-utils`

#### データベース
- `database`, `db-postgres`, `db-mysql`, `db-sqlite`, `db-mongodb`, `db-cockroachdb`

#### 認証
- `auth`, `auth-jwt`, `auth-session`, `auth-oauth`, `auth-token`

#### キャッシュ
- `cache`, `redis-backend`, `redis-cluster`, `redis-sentinel`, `memcached-backend`

#### ミドルウェア
- `middleware`, `middleware-cors`, `middleware-compression`, `middleware-security`, `middleware-rate-limit`

#### API
- `api`, `rest`, `graphql`, `serialize-json`, `serialize-xml`, `serialize-yaml`

#### その他
- `admin`, `forms`, `templates`, `websockets`, `i18n`, `mail`, `sessions`, `static-files`, `storage`, `tasks`, `shortcuts`, `server`

### 構成テンプレート

#### マイクロサービス
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

#### REST API
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["api-only", "db-postgres"] }
```

#### GraphQLサーバー
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["graphql-server"] }
```

#### フル機能
```toml
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

---

## まとめ

Reinhardtのfeature flagシステムは、**3段階の粒度**（バンドル、機能グループ、個別機能）で**70以上のfeature**を提供しています。

### 主な特徴

1. **柔軟な構成**: マイクロサービスからフル機能アプリまで、用途に応じた最適な構成を実現
2. **自動依存解決**: 上位featureを有効化すると、必要な下位featureが自動的に有効化
3. **パフォーマンス**: 不要な機能を除外することで、ビルド時間とバイナリサイズを削減
4. **デフォルト構成**: `standard`がデフォルトで、ほとんどのプロジェクトに適したバランス型

### 選択ガイド

| 用途 | 推奨構成 | バイナリサイズ |
|-----|---------|--------------|
| シンプルAPI | `minimal` | ~5-10 MB |
| REST API | `api-only` + データベース | ~20-25 MB |
| 一般的なWebアプリ | `standard` | ~25-30 MB |
| フル機能アプリ | `full` | ~50+ MB |

詳細については、各セクションを参照してください。

---

**関連ドキュメント**:
- [README.md](../README.md) - プロジェクト概要
- [GETTING_STARTED.md](GETTING_STARTED.md) - 入門ガイド
- [CLAUDE.md](../CLAUDE.md) - 開発者向けガイドライン
