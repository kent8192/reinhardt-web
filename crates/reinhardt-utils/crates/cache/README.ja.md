# reinhardt-cache

Reinhardtのためのキャッシュフレームワークとバックエンド。

## 概要

メモリ、Redis、ファイルベースのキャッシュを含む複数のバックエンドをサポートする柔軟なキャッシュフレームワークです。キャッシュデコレータ、低レベルキャッシュAPI、およびビューとクエリセットとの統合による自動キャッシュ機能を提供します。

## 機能

### コアキャッシュAPI - 実装済み ✓

- **`Cache` トレイト**: ジェネリック型をサポートする非同期ファーストのキャッシュ操作トレイト
  - `get<T>()`: 自動デシリアライゼーションでキャッシュから値を取得
  - `set<T>()`: オプションのTTL(有効期限)付きで値を保存
  - `delete()`: 個別のキャッシュエントリを削除
  - `has_key()`: キャッシュキーの存在確認
  - `clear()`: キャッシュからすべてのエントリを削除
  - `get_many()`: 複数のキャッシュキーの一括取得
  - `set_many()`: 複数の値の一括保存
  - `delete_many()`: 複数のキーの一括削除
  - `incr()`: 数値のアトミックなインクリメント
  - `decr()`: 数値のアトミックなデクリメント

### キャッシュバックエンド - 実装済み ✓

- **InMemoryCache**: スレッドセーフなインメモリキャッシュバックエンド
  - 並行アクセスのための`Arc<RwLock<HashMap>>`を基盤に構築
  - TTLサポート付きの自動有効期限
  - `with_default_ttl()`: デフォルトの有効期限を設定
  - `cleanup_expired()`: 期限切れエントリの手動クリーンアップ
  - 型安全性のためのserdeによるJSONシリアライゼーション

- **RedisCache**: Redis対応の分散キャッシュ(`redis-backend`フィーチャーが必要)
  - 効率的な接続再利用のための`ConnectionManager`による接続プーリング
  - `with_default_ttl()`: デフォルトTTLの設定
  - `with_key_prefix()`: マルチテナントシナリオのための名前空間サポート
  - 整理されたキャッシュエントリのための自動キープレフィックス付与
  - すべてのコア操作が実装された完全なRedis統合
  - パフォーマンス向上のためのバッチ操作(`get_many`、`set_many`、`delete_many`)
  - Redisネイティブコマンドを使用したアトミック操作(`incr`、`decr`)

### キャッシュキー管理 - 実装済み ✓

- **CacheKeyBuilder**: バージョン付きキャッシュキーを生成するユーティリティ
  - `new()`: カスタムプレフィックスでビルダーを作成
  - `with_version()`: バージョンベースのキャッシュ無効化
  - `build()`: プレフィックスとバージョン付きのキーを生成
  - `build_many()`: 一括キー生成
  - フォーマット: `{prefix}:{version}:{key}`

### HTTPミドルウェア - 実装済み ✓

- **CacheMiddleware**: HTTPレスポンスの自動キャッシュ
  - リクエストメソッドフィルタリング(デフォルトでGETのみ、`cache_get_only`経由)
  - レスポンスステータスコードフィルタリング(デフォルトで2xxのみ、`cache_success_only`経由)
  - Cache-Controlヘッダーのパース(max-age、no-cache、no-storeディレクティブ)
  - `CacheMiddlewareConfig`による設定可能なキャッシュタイムアウト
  - クエリパラメータを考慮したキャッシュキー生成
  - 完全なレスポンスキャッシュ(ステータス、ヘッダー、ボディ)

- **CacheMiddlewareConfig**: ミドルウェアの設定
  - `with_default_timeout()`: デフォルトのキャッシュ期間を設定
  - `with_key_prefix()`: キャッシュ名前空間を設定
  - `cache_all_methods()`: GET以外のリクエストのキャッシュを有効化
  - `cache_all_responses()`: 2xx以外のレスポンスをキャッシュ
  - カスタムCache-Controlヘッダー名のサポート

### 依存性注入サポート - 実装済み ✓

- **CacheService**: DI統合を備えた高レベルサービス
  - `reinhardt-di`による自動注入
  - 自動キープレフィックス付与のための統合された`CacheKeyBuilder`
  - メソッド: 自動キー構築付きの`get()`、`set()`、`delete()`
  - `cache()`メソッドによる基盤キャッシュへのアクセス
  - `key_builder()`メソッドによるキービルダーへのアクセス

- **RedisConfig**: DIのためのRedis設定(`redis-backend`フィーチャーが必要)
  - `new()`: カスタムRedis URLの設定
  - `localhost()`: localhostの簡易セットアップ
  - シングルトンスコープからの自動注入
  - 設定されていない場合はlocalhostにフォールバック

- **Injectableトレイトの実装**:
  - `InMemoryCache`: デフォルトのシングルトンベース注入を使用
  - `CacheKeyBuilder`: カスタムデフォルト("app"プレフィックス、バージョン1)
  - `RedisCache`: `RedisConfig`依存関係で注入
  - `CacheService`: DIを介してキャッシュとキービルダーを合成

### フィーチャーフラグ - 実装済み ✓

- `redis-backend`: Redisサポートを有効化(オプション依存)
- `memcached-backend`: Memcachedサポートを有効化(オプション依存)
- `all-backends`: すべてのバックエンド実装を有効化

## 予定されている機能

### キャッシュバックエンド

- **ファイルベースキャッシュ**: 永続的なファイルシステムキャッシュ
- **Memcachedバックエンド**: Memcached統合(依存関係は宣言済みだが未実装)
- **ハイブリッドキャッシュ**: 多層キャッシュ(メモリ + 分散)

### 高度なキャッシュ機能

- **ビュー単位のキャッシュ**: ビューレベルのキャッシュデコレータ
- **テンプレートフラグメントキャッシュ**: 選択的なテンプレート出力のキャッシュ
- **QuerySetキャッシュ**: ORMクエリ結果の自動キャッシュ
- **キャッシュウォーミング**: 起動時のキャッシュ事前投入
- **キャッシュタグ**: 関連エントリのタグベース無効化

### キャッシュ戦略

- **ライトスルー**: 同期的なキャッシュ更新
- **ライトビハインド**: 非同期的なキャッシュ更新
- **キャッシュアサイド**: アプリケーション管理のキャッシュ
- **リードスルー**: ミス時の自動キャッシュ投入

### 監視と管理

- **キャッシュ統計**: ヒット/ミス率、エントリ数、メモリ使用量
- **キャッシュ検査**: キーのリスト化、エントリの表示、キャッシュ状態のエクスポート
- **自動クリーンアップ**: 期限切れエントリ削除のためのバックグラウンドタスク
- **イベントフック**: キャッシュ操作前後のコールバック

### Redisバックエンドの完成

- **完全なRedis統合**: Redis操作の完全な実装
- **接続プーリング**: 効率的な接続管理
- **Redis Clusterサポート**: 分散Redisデプロイメント
- **Redis Sentinelサポート**: 高可用性構成
- **Pub/Subサポート**: Redisチャネル経由のキャッシュ無効化

## インストール

`Cargo.toml`に追加:

```toml
[dependencies]
reinhardt-cache = { workspace = true }

# Redisサポート付き
reinhardt-cache = { workspace = true, features = ["redis-backend"] }

# すべてのバックエンド付き
reinhardt-cache = { workspace = true, features = ["all-backends"] }
```

## 使用例

### 基本的なインメモリキャッシュ

```rust
use reinhardt_cache::{Cache, InMemoryCache};
use std::time::Duration;

let cache = InMemoryCache::new();

// TTL付きで値を設定
cache.set("user:123", &user_data, Some(Duration::from_secs(300))).await?;

// 値を取得
let user: Option<UserData> = cache.get("user:123").await?;

// 値を削除
cache.delete("user:123").await?;
```

### キャッシュキービルダーの使用

```rust
use reinhardt_cache::CacheKeyBuilder;

let builder = CacheKeyBuilder::new("myapp").with_version(2);

// 単一のキーを構築
let key = builder.build("user:123"); // "myapp:2:user:123"

// 複数のキーを構築
let keys = builder.build_many(&["user:1", "user:2"]);
```

### HTTPレスポンスキャッシュミドルウェア

```rust
use reinhardt_cache::{CacheMiddleware, CacheMiddlewareConfig, InMemoryCache};
use std::sync::Arc;
use std::time::Duration;

let cache = Arc::new(InMemoryCache::new());
let config = CacheMiddlewareConfig::new()
    .with_default_timeout(Duration::from_secs(600))
    .with_key_prefix("api_cache");

let middleware = CacheMiddleware::with_config(cache, config);
// アプリケーションにミドルウェアを追加
```

### 依存性注入

```rust
use reinhardt_cache::CacheService;
use reinhardt_di::{Injectable, InjectionContext};

// CacheServiceを注入
let service = CacheService::inject(&ctx).await?;

// 自動キー構築で使用
service.set("session", &session_data, Some(Duration::from_secs(3600))).await?;
let session: Option<SessionData> = service.get("session").await?;
```

### Redisキャッシュ(フィーチャーゲート)

```rust
use reinhardt_cache::{Cache, RedisCache, RedisConfig};
use std::time::Duration;

// DI経由
let config = RedisConfig::new("redis://localhost:6379");
ctx.set_singleton(config);

// 直接インスタンス化
let cache = RedisCache::new("redis://localhost:6379")
    .await?
    .with_default_ttl(Duration::from_secs(300))
    .with_key_prefix("myapp");

// キャッシュを使用
cache.set("user:123", &user_data, Some(Duration::from_secs(3600))).await?;
let user: Option<UserData> = cache.get("user:123").await?;

// バッチ操作
let mut values = HashMap::new();
values.insert("key1".to_string(), "value1".to_string());
values.insert("key2".to_string(), "value2".to_string());
cache.set_many(values, None).await?;

// アトミック操作
cache.incr("counter", 1).await?;
cache.decr("counter", 1).await?;
```

## アーキテクチャ

### キャッシュエントリの構造

- シリアライズされた値は`Vec<u8>`として保存(serdeを使ったJSON)
- オプションの有効期限タイムスタンプ(`SystemTime`)
- 取得時の自動有効期限チェック

### スレッドセーフティ

- `InMemoryCache`での並行アクセスのための`Arc<RwLock<HashMap>>`
- `RwLock`による読み取り重視の最適化
- すべてのキャッシュ実装は`Send + Sync`

### エラー処理

- `reinhardt-exception::Error`による統一されたエラー型
- シリアライゼーションエラーは`Error::Serialization`としてラップ
- すべての操作は`Result<T, Error>`を返す

## テスト

すべての機能には以下を含む包括的なテストカバレッジがあります:

- すべてのキャッシュ操作のユニットテスト
- TTL有効期限動作のテスト
- バッチ操作のテスト
- ミドルウェア統合テスト
- DI注入テスト
- キービルダー機能のテスト

テストの実行:

```bash
cargo test -p reinhardt-cache
cargo test -p reinhardt-cache --features redis-backend
cargo test -p reinhardt-cache --features all-backends
```

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
