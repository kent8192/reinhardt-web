# reinhardt-sessions

Reinhardt向けのDjangoスタイルのセッション管理。

## 概要

HTTPリクエスト間で状態を維持するためのセッションフレームワーク。このクレートは、さまざまなストレージシステムにセッションデータを保存するためのセッションバックエンドトレイトと実装を提供します。

## 機能

### 実装済み ✓

#### コアセッションバックエンド

- **SessionBackend トレイト** - セッションストレージ操作(load, save, delete, exists)を定義する非同期トレイト
- **SessionError** - セッション操作のエラータイプ(キャッシュエラー、シリアライゼーションエラー)
- **汎用セッションストレージ** - `serde`サポート付きの型安全なセッションデータストレージ

#### キャッシュベースバックエンド

- **InMemorySessionBackend** - `InMemoryCache`を使用したインメモリセッションストレージ
  - 高速かつ揮発性のストレージ(再起動時にセッションは失われます)
  - 自動有効期限のためのTTL(Time-To-Live)サポート
  - 開発環境および単一インスタンスデプロイメントに適しています
- **CacheSessionBackend** - 汎用キャッシュベースセッションバックエンド
  - 任意の`Cache`トレイト実装で動作
  - 外部キャッシュシステム(Redis、Memcachedなど)をサポート
  - セッション有効期限の設定可能なTTL
  - 分散システムの水平スケーラビリティ

#### 依存性注入サポート

- 依存性注入のための`reinhardt-di`との統合
- セッションバックエンドの登録と解決

#### 高レベルセッションAPI

- **Session<B>** 構造体 - 辞書風インターフェースを持つDjangoスタイルのセッションオブジェクト
  - ジェネリックバックエンドパラメータ`B: SessionBackend`による型安全性
  - 辞書風メソッド: `get()`, `set()`, `delete()`, `contains_key()`
  - セッション反復メソッド: `keys()`, `values()`, `items()`
  - 手動セッションクリア: `clear()`
  - 手動変更追跡: `mark_modified()`, `mark_unmodified()`
  - セッション変更追跡: `is_modified()`, `is_accessed()`
  - セッションキー管理: `get_or_create_key()`, `generate_key()`
  - セッションライフサイクル: `flush()` (クリアして新しいキー), `cycle_key()` (データを保持して新しいキー)
  - 自動永続化: TTLサポート付き`save()`メソッド(デフォルト: 3600秒)
  - 包括的なdoctestsとユニットテスト(合計36テスト)

#### ストレージバックエンド

- **DatabaseSessionBackend** (feature: `database`) - データベース内の永続的セッションストレージ
  - 有効期限タイムスタンプ付きセッションモデル
  - `cleanup_expired()`による自動セッションクリーンアップ
  - sqlx経由でSQLite、PostgreSQL、MySQLをサポート
  - `create_table()`によるテーブル作成
  - 効率的なクリーンアップのための有効期限日付のインデックス化
  - 9つの包括的テスト
- **FileSessionBackend** (feature: `file`) - ファイルベースのセッションストレージ
  - 設定可能なディレクトリにセッションファイルを保存(デフォルト: `/tmp/reinhardt_sessions`)
  - 同時アクセスの安全性のための`fs2`を使用したファイルロック
  - TTLサポート付きJSONシリアライゼーション
  - アクセス時の期限切れセッションの自動クリーンアップ
  - 11の包括的テスト
- **CookieSessionBackend** (feature: `cookie`) - Cookie内の暗号化セッションデータ
  - セッションデータのAES-256-GCM暗号化
  - 改ざん検出のためのHMAC-SHA256署名
  - 自動サイズ制限チェック(最大4KB)
  - セキュアなクライアント側ストレージ
  - 11の包括的テスト

#### HTTPミドルウェア

- **SessionMiddleware** (feature: `middleware`) - セッション管理のためのHTTPミドルウェア
  - Cookieからの自動セッション読み込み
  - レスポンス時の自動セッション保存
  - Cookie設定: name、path、domain
  - セキュリティ設定: secure、httponly、samesite
  - TTLとmax-ageサポート
- **HttpSessionConfig** - 包括的なミドルウェア設定
- **SameSite** enum - Cookie SameSite属性(Strict、Lax、None)

### 予定

#### セッション管理機能

- セッション有効期限とクリーンアップ
- セッションキーローテーション
- クロスサイトリクエストフォージェリ(CSRF)保護との統合
- セッションシリアライゼーション形式(JSON、MessagePackなど)
- セッションストレージ移行ツール

#### 高度な機能

- 高可用性のためのセッションレプリケーション
- セッション分析とモニタリング
- カスタムセッションシリアライザ
- 大きなデータのセッション圧縮
- マルチテナントセッション分離

## インストール

`Cargo.toml`に追加:

```toml
[dependencies]
reinhardt-sessions = "0.1.0"

# オプション機能付き
reinhardt-sessions = { version = "0.1.0", features = ["database", "file", "cookie", "middleware"] }
```

## クイックスタート

### InMemorySessionBackendでセッションを使用

```rust
use reinhardt_sessions::Session;
use reinhardt_sessions::backends::InMemorySessionBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = InMemorySessionBackend::new();
    let mut session = Session::new(backend);

    // セッションデータの設定
    session.set("user_id", 42)?;
    session.set("username", "alice")?;

    // セッションデータの取得
    let user_id: i32 = session.get("user_id")?.unwrap();
    assert_eq!(user_id, 42);

    // キーの存在確認
    assert!(session.contains_key("username"));

    // キーの削除
    session.delete("username");

    // セッションの保存
    session.save().await?;

    Ok(())
}
```

### SessionBackendを直接使用

```rust
use reinhardt_sessions::backends::{InMemorySessionBackend, SessionBackend};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // セッションバックエンドの作成
    let backend = InMemorySessionBackend::new();

    // セッションデータの保存
    let session_data = json!({
        "user_id": 42,
        "username": "alice",
        "authenticated": true,
    });

    backend.save("session_key_123", &session_data, Some(3600)).await?;

    // セッションデータの取得
    let retrieved: Option<serde_json::Value> = backend.load("session_key_123").await?;
    assert!(retrieved.is_some());

    // セッションの存在確認
    assert!(backend.exists("session_key_123").await?);

    // セッションの削除
    backend.delete("session_key_123").await?;

    Ok(())
}
```

### カスタムキャッシュでCacheSessionBackendを使用

```rust
use reinhardt_sessions::backends::{CacheSessionBackend, SessionBackend};
use reinhardt_cache::InMemoryCache;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // キャッシュとバックエンドの作成
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    // ユーザー設定の保存
    let preferences = json!({
        "theme": "dark",
        "language": "en",
        "notifications": true,
    });

    backend.save("pref_session_789", &preferences, Some(7200)).await?;

    // 設定の読み込み
    let loaded: Option<serde_json::Value> = backend.load("pref_session_789").await?;
    assert_eq!(loaded.unwrap()["theme"], "dark");

    Ok(())
}
```

## フィーチャーフラグ

- `database` - データベースバックセッションを有効化(`reinhardt-orm`が必要)
- `file` - ファイルバックセッションを有効化(fs機能付き`tokio`が必要)
- `cookie` - 暗号化付きCookieバックセッションを有効化(`base64`、`aes-gcm`、`rand`が必要)
- `middleware` - HTTPミドルウェアサポートを有効化(`reinhardt-http`、`reinhardt-types`、`reinhardt-exception`が必要)

## アーキテクチャ

### SessionBackend トレイト

セッションフレームワークのコアは`SessionBackend`トレイトで、セッションストレージのインターフェースを定義します:

```rust
#[async_trait]
pub trait SessionBackend: Send + Sync {
    /// キーでセッションデータを読み込み
    async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
    where
        T: for<'de> Deserialize<'de> + Send;

    /// オプションのTTL(秒単位)でセッションデータを保存
    async fn save<T>(
        &self,
        session_key: &str,
        data: &T,
        ttl: Option<u64>,
    ) -> Result<(), SessionError>
    where
        T: Serialize + Send + Sync;

    /// キーでセッションを削除
    async fn delete(&self, session_key: &str) -> Result<(), SessionError>;

    /// セッションの存在確認
    async fn exists(&self, session_key: &str) -> Result<bool, SessionError>;
}
```

### 型安全性

すべてのセッションバックエンドは、Rustの型システムを使用して型安全なシリアライゼーションとデシリアライゼーションを保証します:

- ジェネリック型パラメータにより、任意の`serde`互換データを保存可能
- コンパイル時の型チェックにより、ランタイム型エラーを防止
- 自動シリアライゼーション/デシリアライゼーション処理

## Djangoとの比較

このクレートはDjangoのセッションフレームワークにインスパイアされています:

| 機能                         | Django | Reinhardt Sessions                 |
| ---------------------------- | ------ | ---------------------------------- |
| セッションバックエンド       | ✓      | ✓                                  |
| セッションオブジェクト       | ✓      | ✓                                  |
| インメモリバックエンド       | ✓      | ✓                                  |
| データベースバックエンド     | ✓      | ✓ (SQLite、PostgreSQL、MySQL)     |
| ファイルバックエンド         | ✓      | ✓ (ファイルロック付き)             |
| Cookieバックエンド           | ✓      | ✓ (AES-GCM暗号化)                  |
| セッションミドルウェア       | ✓      | ✓                                  |
| TTL/有効期限                 | ✓      | ✓                                  |
| セッション反復               | ✓      | ✓ (keys、values、items)            |
| 手動変更コントロール         | ✓      | ✓ (mark_modified、mark_unmodified) |
| 型安全性                     | -      | ✓ (Rust型)                         |
| 非同期操作                   | -      | ✓                                  |

## ライセンス

以下のいずれかでライセンスされています:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好きな方をお選びください。

## コントリビューション

コントリビューションを歓迎します! ガイドラインについては、メインの[CONTRIBUTING.md](../../CONTRIBUTING.md)をご覧ください。
