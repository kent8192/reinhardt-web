# reinhardt-di

FastAPIにインスパイアされた依存性注入システムfor Reinhardt。

## 概要

FastAPIスタイルの依存性注入システムを提供します。リクエストスコープ、シングルトンスコープの依存性キャッシング、ネストされた依存性の自動解決、認証やデータベース接続との統合をサポートします。

型安全で非同期ファーストな設計により、FastAPIの開発体験をRustで実現します。

## コアコンセプト

### 依存性スコープ

- **リクエストスコープ**: リクエストごとにキャッシュされる依存性（デフォルト）
- **シングルトンスコープ**: アプリケーション全体で共有される依存性

### 自動注入

`Default + Clone + Send + Sync + 'static`を実装する型は、自動的に`Injectable`トレイトが実装され、依存性として使用できます。

## 実装済みの機能 ✓

### コア依存性注入

- ✓ **`Depends<T>` ラッパー**: FastAPIスタイルの依存性注入ラッパー
  - `Depends::<T>::new()` - キャッシュ有効（デフォルト）
  - `Depends::<T>::no_cache()` - キャッシュ無効
  - `resolve(&ctx)` - 依存性の解決
  - `from_value(value)` - テスト用の値からの生成

- ✓ **Injectable トレイト**: 依存性として注入可能な型を定義
  - 自動実装: `Default + Clone + Send + Sync + 'static`型に対して
  - カスタム実装: 複雑な初期化ロジックが必要な場合

- ✓ **InjectionContext**: 依存性解決のためのコンテキスト
  - `get_request<T>()` / `set_request<T>()` - リクエストスコープ
  - `get_singleton<T>()` / `set_singleton<T>()` - シングルトンスコープ
  - リクエストごとに新しいコンテキストを生成

- ✓ **RequestScope**: リクエスト内でのキャッシング
  - 型ベースのキャッシュ（`TypeId`をキーとして使用）
  - スレッドセーフな実装（`Arc<RwLock<HashMap>>`）

- ✓ **SingletonScope**: アプリケーション全体でのキャッシング
  - すべてのリクエスト間で共有される依存性
  - スレッドセーフな実装

### 高度な機能

- ✓ **依存性キャッシング**: リクエストスコープ内での自動キャッシング
  - 同じ依存性を複数回要求しても1回だけ生成される
  - ネストされた依存性間でキャッシュが共有される
  - キャッシュの有効/無効を制御可能

- ✓ **ネストされた依存性**: 依存性が他の依存性に依存できる
  - 依存性グラフの自動解決
  - 循環依存の検出とエラー処理

- ✓ **依存性オーバーライド**: テスト用の依存性オーバーライド
  - 本番とテストで異なる実装を使用可能
  - アプリケーションレベルでのオーバーライド管理
  - サブ依存性を持つオーバーライドのサポート

- ✓ **プロバイダーシステム**: 非同期ファクトリーパターン
  - `Provider` trait - 依存性を提供するための汎用インターフェース
  - `ProviderFn` - 関数ベースのプロバイダー
  - 任意の非同期クロージャをプロバイダーとして使用可能

### エラーハンドリング

- ✓ **DiError**: 包括的なエラー型
  - `NotFound` - 依存性が見つからない
  - `CircularDependency` - 循環依存の検出
  - `ProviderError` - プロバイダーのエラー
  - `TypeMismatch` - 型の不一致
  - `ScopeError` - スコープ関連のエラー

### 統合サポート

- ✓ **HTTP統合**: HTTPリクエスト/レスポンスとの統合
  - リクエストからの依存性注入
  - 接続情報の注入サポート

- ✓ **WebSocketサポート**: WebSocket接続への依存性注入
  - WebSocketハンドラーでの`Depends<T>`使用

### 高度な依存性パターン ✓

#### ジェネレーターベースの依存性（yieldパターン）

- **ライフサイクル管理**: セットアップ/ティアダウンパターン
- **コンテキストマネージャー**: 自動リソースクリーンアップ
- **エラーハンドリング**: エラー時でもクリーンアップ実行
- **ストリーミングサポート**: ストリーミングレスポンス対応
- **WebSocketサポート**: WebSocketハンドラーとの統合

```rust
use reinhardt_di::{Injectable, InjectionContext};

#[derive(Clone)]
struct DatabaseConnection {
    // セットアップ
}

impl DatabaseConnection {
    async fn setup() -> Self {
        // コネクションの初期化
        DatabaseConnection { }
    }

    async fn cleanup(self) {
        // コネクションのクローズ
    }
}
```

#### 依存性クラス（クラスベース依存性）

- **呼び出し可能な依存性**: callメソッドを持つ構造体ベースの依存性
- **非同期呼び出し可能**: 非同期依存性メソッドのサポート
- **ステートフルな依存性**: 内部状態を持つ依存性
- **メソッドベースの注入**: 柔軟な依存性構築

```rust
#[derive(Clone)]
struct CallableDependency {
    prefix: String,
}

impl CallableDependency {
    fn call(&self, value: String) -> String {
        format!("{}{}", self.prefix, value)
    }
}
```

#### パラメータ化された依存性（パラメータ化依存性）

- **パスパラメータ統合**: 依存性からパスパラメータへのアクセス
- **共有パラメータ**: エンドポイントと依存性でパスパラメータを共有
- **型安全な抽出**: コンパイル時に検証されたパラメータ渡し

```rust
// 依存性内でパスパラメータにアクセス可能
#[async_trait::async_trait]
impl Injectable for UserValidator {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let user_id = UserId::inject(ctx).await?;
        Ok(UserValidator { user_id: user_id.0 })
    }
}
```

#### スキーマ生成（スキーマ生成）

- **依存性の重複排除**: 共有依存性はスキーマに1度だけ出現
- **推移的な依存性**: ネストされた依存性の自動キャッシング
- **スキーマの最適化**: 効率的な依存性グラフ表現

#### セキュリティオーバーライド（セキュリティオーバーライド）

- **セキュリティ依存性**: OAuth2、JWT、その他の認証スキーム
- **セキュリティスコープ**: スコープベースのアクセス制御
- **オーバーライドサポート**: テストフレンドリーなセキュリティ依存性の置き換え

```rust
// スコープを持つセキュリティ依存性
#[async_trait::async_trait]
impl Injectable for UserData {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let scopes = ctx.get_request::<SecurityScopes>()?;
        Ok(UserData { scopes: scopes.scopes })
    }
}
```

## 予定されている機能

現在、すべての主要な依存性注入機能が実装済みです。以下は将来的な拡張の可能性があります：

### 将来の機能拡張

- **非同期ジェネレーター構文**: Rustの非同期ジェネレーターが安定版になった際の統合
- **依存性の可視化**: 依存性グラフの可視化ツール
- **パフォーマンスプロファイリング**: 依存性注入のパフォーマンス分析ツール
- **高度なキャッシング戦略**: より高度なキャッシング戦略

## 使用例

### 基本的な使い方

```rust
use reinhardt_di::{Depends, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

#[derive(Clone, Default)]
struct Config {
    api_key: String,
    database_url: String,
}

#[tokio::main]
async fn main() {
    // シングルトンスコープの作成
    let singleton = Arc::new(SingletonScope::new());

    // リクエストコンテキストの作成
    let ctx = InjectionContext::new(singleton);

    // 依存性の解決（キャッシュ有効）
    let config = Depends::<Config>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    println!("API Key: {}", config.api_key);
}
```

### カスタムInjectableの実装

```rust
use reinhardt_di::{Injectable, InjectionContext, DiResult};

struct Database {
    pool: DbPool,
}

#[async_trait::async_trait]
impl Injectable for Database {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // カスタム初期化ロジック
        let config = Config::inject(ctx).await?;
        let pool = create_pool(&config.database_url).await?;

        Ok(Database { pool })
    }
}
```

### ネストされた依存性

```rust
#[derive(Clone)]
struct ServiceA {
    db: Arc<Database>,
}

#[async_trait::async_trait]
impl Injectable for ServiceA {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Databaseに依存
        let db = Database::inject(ctx).await?;
        Ok(ServiceA { db: Arc::new(db) })
    }
}

#[derive(Clone)]
struct ServiceB {
    service_a: Arc<ServiceA>,
    config: Config,
}

#[async_trait::async_trait]
impl Injectable for ServiceB {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // ServiceAとConfigに依存（ネストされた依存性）
        let service_a = ServiceA::inject(ctx).await?;
        let config = Config::inject(ctx).await?;

        Ok(ServiceB {
            service_a: Arc::new(service_a),
            config,
        })
    }
}
```

### テスト用の依存性オーバーライド

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockDatabase {
        // テスト用のモック実装
    }

    #[tokio::test]
    async fn test_with_mock_database() {
        let singleton = Arc::new(SingletonScope::new());
        let ctx = InjectionContext::new(singleton);

        // テスト用のモックをセット
        let mock_db = MockDatabase { /* ... */ };
        ctx.set_request(mock_db);

        // テストコード
    }
}
```

### キャッシュ制御

```rust
// キャッシュ有効（デフォルト） - 同じインスタンスを返す
let config1 = Depends::<Config>::new().resolve(&ctx).await?;
let config2 = Depends::<Config>::new().resolve(&ctx).await?;
// config1とconfig2は同じインスタンス

// キャッシュ無効 - 毎回新しいインスタンスを作成
let config3 = Depends::<Config>::no_cache().resolve(&ctx).await?;
let config4 = Depends::<Config>::no_cache().resolve(&ctx).await?;
// config3とconfig4は異なるインスタンス
```

## アーキテクチャ

### 型ベースのキャッシング

依存性のキャッシュは型（`TypeId`）をキーとして管理されます。これにより、同じ型の依存性は自動的にキャッシュされます。

### スコープの階層構造

```
SingletonScope（アプリケーションレベル）
    ↓ 共有
InjectionContext（リクエストレベル）
    ↓ 保持
RequestScope（リクエスト内キャッシュ）
```

### スレッド安全性

- すべてのスコープは`Arc<RwLock<HashMap>>`を使用してスレッドセーフ
- `Injectable` トレイトは`Send + Sync`を要求
- 非同期コードで安全に使用可能

## テストサポート

テストフレームワークには包括的なテストスイートが含まれています：

- **ユニットテスト**: 各コンポーネントの単体テスト
- **統合テスト**: FastAPIのテストケースを移植した統合テスト
- **機能テスト**:
  - 自動Injectable実装のテスト
  - 循環依存の検出テスト
  - キャッシュ動作のテスト
  - 依存性オーバーライドのテスト
  - ネストされた依存性のテスト

## パフォーマンスに関する考慮事項

- **遅延初期化**: 依存性は必要になるまで生成されません
- **キャッシュの効率性**: リクエストスコープ内で同じ依存性は1回だけ生成されます
- **ゼロコスト抽象化**: Rustの型システムを活用したオーバーヘッドの少ない設計
- **Arcベースの共有**: `Arc`を使用した効率的なインスタンス共有

## FastAPIとの比較

| 機能                         | FastAPI（Python） | reinhardt-di（Rust） |
| ---------------------------- | ----------------- | -------------------- |
| 基本的なDI                   | ✓                 | ✓                    |
| リクエストスコープ           | ✓                 | ✓                    |
| シングルトンスコープ         | ✓                 | ✓                    |
| 依存性キャッシング           | ✓                 | ✓                    |
| ネストされた依存性           | ✓                 | ✓                    |
| 依存性オーバーライド         | ✓                 | ✓                    |
| `yield`パターン              | ✓                 | ⏳ 予定              |
| 型安全性                     | ランタイム        | **コンパイル時**     |
| パフォーマンス               | 動的              | **静的・高速**       |

## ライセンス

このクレートはReinhardtプロジェクトの一部であり、同じデュアルライセンス構造（MITまたはApache-2.0）に従います。
