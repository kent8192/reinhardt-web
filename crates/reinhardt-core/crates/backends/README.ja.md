# reinhardt-backends

Reinhardtフレームワークのための共有バックエンドインフラストラクチャ

## 概要

このクレートは、スロットリング、キャッシング、セッションストレージなど、Reinhardtフレームワークのさまざまなコンポーネント間でデータを保存・取得するための統一的なバックエンドシステムを提供します。

## 機能

### コア機能

- **Backendトレイト**: TTLサポート付きの汎用キーバリューインターフェース
- **MemoryBackend**: 自動期限切れ機能を備えた高性能インメモリストレージ
- **RedisBackend**: Redisを使用した分散ストレージ(機能ゲート)

### 主要な機能

- `async-trait`を使用した非同期ファーストデザイン
- TTLサポートによる自動期限切れ
- `serde`による型安全なシリアライズ/デシリアライズ
- スレッドセーフな並行アクセス
- カウンター用のインクリメント操作

## インストール

`Cargo.toml`に追加:

```toml
[dependencies]
reinhardt-backends = { workspace = true }

# Redisサポートを有効化
reinhardt-backends = { workspace = true, features = ["redis-backend"] }
```

## 使用例

### Memory Backend

```rust
use reinhardt_backends::{Backend, MemoryBackend};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let backend = MemoryBackend::new();

    // TTL付きで保存
    backend.set("user:123", "active", Some(Duration::from_secs(3600))).await.unwrap();

    // 取得
    let value: Option<String> = backend.get("user:123").await.unwrap();
    assert_eq!(value, Some("active".to_string()));

    // カウンター操作
    let count = backend.increment("api:calls", Some(Duration::from_secs(60))).await.unwrap();
    println!("API call count: {}", count);
}
```

### Redis Backend

```rust
use reinhardt_backends::{Backend, RedisBackend};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let backend = RedisBackend::new("redis://localhost:6379").await.unwrap();

    // MemoryBackendと同じAPI
    backend.set("session:abc", vec![1, 2, 3], Some(Duration::from_secs(3600))).await.unwrap();

    let data: Option<Vec<u8>> = backend.get("session:abc").await.unwrap();
    assert_eq!(data, Some(vec![1, 2, 3]));
}
```

### 共有バックエンドパターン

```rust
use reinhardt_backends::{Backend, MemoryBackend};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 共有バックエンドを作成
    let backend = Arc::new(MemoryBackend::new());

    // スロットリングで使用
    let throttle_backend = backend.clone();

    // キャッシュで使用
    let cache_backend = backend.clone();

    // セッションストレージで使用
    let session_backend = backend.clone();

    // すべてのコンポーネントが同じ状態を共有
}
```

## API ドキュメント

### Backendトレイト

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn set<V>(&self, key: &str, value: V, ttl: Option<Duration>) -> BackendResult<()>;
    async fn get<V>(&self, key: &str) -> BackendResult<Option<V>>;
    async fn delete(&self, key: &str) -> BackendResult<bool>;
    async fn exists(&self, key: &str) -> BackendResult<bool>;
    async fn increment(&self, key: &str, ttl: Option<Duration>) -> BackendResult<i64>;
    async fn clear(&self) -> BackendResult<()>;
}
```

### Memory Backend

- **スレッドセーフ**: 並行アクセスのために`DashMap`を使用
- **自動クリーンアップ**: 期限切れエントリは自動的に削除されます
- **ゼロコスト**: メモリバックエンド使用時は外部依存なし

### Redis Backend

- **分散**: 複数のサーバー間で状態を共有
- **永続化**: アプリケーションの再起動後もデータが存続
- **スケーラブル**: Redisは毎秒数百万の操作を処理

## 機能フラグ

- `memory` (デフォルト): インメモリバックエンドを有効化
- `redis-backend`: Redisバックエンドを有効化

## テスト

```bash
# メモリバックエンドのテストを実行
cargo test --package reinhardt-backends

# Redisテストを実行 (Redisサーバーが必要)
cargo test --package reinhardt-backends --features redis-backend -- --ignored
```

## パフォーマンス

### Memory Backend

- **スループット**: ~1M ops/sec (シングルスレッド)
- **レイテンシ**: get/set操作で <1μs
- **メモリ**: O(n) nはキーの数

### Redis Backend

- **スループット**: ~100K ops/sec (Redisに依存)
- **レイテンシ**: ~1-5ms (ネットワーク + Redis)
- **メモリ**: Redisが管理

## 統合例

### スロットリング統合

```rust
use reinhardt_backends::{Backend, MemoryBackend};
use std::sync::Arc;

pub struct Throttle {
    backend: Arc<dyn Backend>,
    rate: String,
}

impl Throttle {
    pub fn new(backend: Arc<dyn Backend>, rate: &str) -> Self {
        Self {
            backend,
            rate: rate.to_string(),
        }
    }

    pub async fn allow(&self, key: &str) -> bool {
        let count = self.backend.increment(key, Some(std::time::Duration::from_secs(60))).await.unwrap();
        count <= 100 // 1分あたり100リクエストを許可
    }
}
```

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。