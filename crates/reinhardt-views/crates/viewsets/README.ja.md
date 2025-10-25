# reinhardt-viewsets

APIエンドポイント用の組み立て可能なビュー

## 概要

ViewSetは、複数の関連するビューのロジックを単一のクラスに統合します。CRUD操作用のModelViewSet、ReadOnlyModelViewSet、およびカスタムViewSetクラスを提供します。

list、retrieve、create、update、delete操作などの一般的なパターンを自動的に処理します。

## 機能

### 実装済み ✓

#### コアViewSet型

- **ViewSet Trait** - すべてのViewSet実装の基礎となるトレイトで、ディスパッチ、ミドルウェアサポート、アクションルーティングを提供
- **GenericViewSet** - 組み立て可能なハンドラーパターンを持つ汎用ViewSet実装
- **ModelViewSet** - モデルベースのAPI用の完全なCRUD操作（list、retrieve、create、update、destroy）
- **ReadOnlyModelViewSet** - 不変リソース用の読み取り専用操作（list、retrieve）

#### アクションシステム

- **アクション型** - 標準のCRUD操作とカスタムアクションをサポートする包括的なアクション型システム
  - 標準アクション: List、Retrieve、Create、Update、PartialUpdate、Destroy
  - 詳細/リスト動作を設定可能なカスタムアクションサポート
- **アクションメタデータ** - アクション用の豊富なメタデータシステム:
  - カスタム表示名とサフィックス
  - URLパスと名前の設定
  - HTTPメソッドフィルタリング
  - アクションハンドラー統合
- **アクションレジストリ** - グローバルおよびローカルのアクション登録システム
  - `register_action()`による手動登録API
  - `register_viewset_actions!`によるマクロベースの登録
  - インベントリベースのアクション自動収集

#### Mixinシステム

- **ListMixin** - コレクションをクエリするlist()アクションを提供
- **RetrieveMixin** - 単一オブジェクトを取得するretrieve()アクションを提供
- **CreateMixin** - オブジェクト作成用のcreate()アクションを提供
- **UpdateMixin** - オブジェクト変更用のupdate()アクションを提供
- **DestroyMixin** - オブジェクト削除用のdestroy()アクションを提供
- **CrudMixin** - すべてのCRUD操作を組み合わせた複合トレイト

#### ミドルウェアサポート

- **ViewSetMiddleware Trait** - 横断的関心事のためのミドルウェア統合
  - `process_request()` - 早期レスポンス機能付き事前処理
  - `process_response()` - 後処理とレスポンス変更
- **AuthenticationMiddleware** - ログイン要件の強制
  - 設定可能なlogin_required動作
  - ログインURLリダイレクトサポート
  - セッションおよびヘッダーベースの認証検出
- **PermissionMiddleware** - パーミッションベースのアクセス制御
  - ViewSet単位のパーミッション要件
  - 未認可アクセスに対する自動403 Forbiddenレスポンス
- **CompositeMiddleware** - ミドルウェアの合成とチェーン化
  - ミドルウェア設定用のビルダーパターン
  - ミドルウェアの順次実行

#### ハンドラー統合

- **ViewSetHandler** - ルーティング統合のためのViewSetからHandlerへの変換
  - HTTPメソッドからアクションへのマッピング
  - パスパラメーター抽出
  - リクエスト属性管理（args、kwargs）
  - ミドルウェア処理パイプライン
- **ViewSetBuilder** - Handler作成用の流暢なビルダーAPI
  - `with_actions()`および`action()`によるアクションマッピング設定
  - カスタム名/サフィックスサポート（相互排他的）
  - アクションマッピングの検証
  - `viewset_actions!`によるマクロサポート

#### 依存性注入（FastAPIスタイル）

- **3つのDIパターン** - ViewSetへの依存性注入の複数の方法:
  1. **フィールドレベル注入** - 構造体フィールドに`#[derive(Injectable)]`と`#[inject]`属性を使用
  2. **メソッドレベル注入** - メソッドパラメーターに`#[endpoint]`と`#[inject]`属性を使用
  3. **ディスパッチレベル注入** - `#[inject]`パラメーターで`dispatch_with_context()`をオーバーライド
- **DiViewSet** - 完全なDIサポートを持つViewSetラッパー
  - `Depends<V>`による自動依存性解決
  - reinhardt-diフレームワークとの統合
- **ViewSetFactory Trait** - DIを使用したViewSet作成のためのファクトリーパターン
- **Injectable Dependencies** - 実装例（DatabaseConnection）
- **キャッシュ制御** - `#[inject(cache = false)]`による細粒度制御
- **後方互換性** - 非DIのViewSetは変更なしで動作し続ける

#### テストユーティリティ

- **TestViewSet** - ミドルウェアサポート付きの設定可能なテストViewSet
  - 設定可能なlogin_required動作
  - パーミッション設定
  - ミドルウェア統合テスト
- **SimpleViewSet** - 基本的なテストシナリオ用の最小限のViewSet

### 予定

#### 高度な機能

- **ページネーション統合** - listアクション用の自動ページネーションサポート
- **フィルタリングシステム** - コレクション用のクエリパラメーターベースのフィルタリング
- **順序付けサポート** - 複数フィールドサポート付きのソート可能なコレクション
- **一括操作** - バッチ作成/更新/削除操作
- **ネストされたViewSet** - 親子リソース関係
- **ViewSetスキーマ生成** - ViewSet定義からのOpenAPIスキーマ生成
- **キャッシュサポート** - 読み取り専用操作のレスポンスキャッシング
- **レート制限** - ViewSet単位またはアクション単位のレート制限
- **WebSocket ViewSet** - WebSocketによるリアルタイムアクションサポート

## 使用例

### パターン1: フィールドレベル依存性注入

ViewSetがインスタンス化されるときに依存性を注入:

```rust
use reinhardt_macros::Injectable;
use reinhardt_di::{Injectable, InjectionContext};

#[derive(Clone, Injectable)]
struct UserViewSet {
    #[inject]
    db: Database,
    #[inject]
    cache: RedisCache,
    name: String,  // Uses Default::default()
}

impl ViewSet for UserViewSet {
    fn get_basename(&self) -> &str {
        "users"
    }

    async fn dispatch(&self, request: Request, action: Action) -> Result<Response> {
        // Use self.db and self.cache
        let users = self.db.query_all().await?;
        Ok(Response::json(&users)?)
    }
}

// Usage
let ctx = InjectionContext::new(singleton);
let viewset = UserViewSet::inject(&ctx).await?;
```

### パターン2: メソッドレベル依存性注入

個々のアクションメソッドに依存性を注入:

```rust
use reinhardt_macros::endpoint;

impl ProductViewSet {
    #[endpoint]
    async fn list(&self, request: Request, #[inject] db: Database) -> Result<Response> {
        let products = db.query_all().await?;
        Ok(Response::json(&products)?)
    }

    #[endpoint]
    #[action(detail = true, methods = ["POST"])]
    async fn activate(
        &self,
        request: Request,
        #[inject] email: EmailService
    ) -> Result<Response> {
        email.send_activation().await?;
        Ok(Response::ok())
    }

    #[endpoint]
    async fn create(
        &self,
        request: Request,
        #[inject] db: Database,
        #[inject] logger: Logger,
    ) -> Result<Response> {
        logger.info("Creating product");
        let product = db.create(request.body()).await?;
        Ok(Response::created().json(&product)?)
    }
}
```

### パターン3: ディスパッチレベル依存性注入

集中制御のためディスパッチレベルで依存性を注入:

```rust
use reinhardt_macros::endpoint;

impl ViewSet for OrderViewSet {
    fn supports_di(&self) -> bool {
        true
    }

    #[endpoint]
    async fn dispatch_with_context(
        &self,
        request: Request,
        action: Action,
        #[inject] db: Database,
        #[inject] logger: Logger,
    ) -> Result<Response> {
        logger.log(&format!("Dispatching action: {:?}", action));

        match action.action_type {
            ActionType::List => self.handle_list(request, db).await,
            ActionType::Retrieve => self.handle_retrieve(request, db).await,
            _ => Err(Error::NotFound("Action not found".to_string()))
        }
    }
}
```

### キャッシュ制御

特定の依存性のキャッシングを無効化:

```rust
#[derive(Clone, Injectable)]
struct MyService {
    #[inject]
    db: Database,              // Cached (default)
    #[inject(cache = false)]
    fresh_data: FreshData,     // Not cached
}

// Or in methods:
#[endpoint]
async fn handler(
    &self,
    request: Request,
    #[inject] cached: Database,
    #[inject(cache = false)] fresh: Database,
) -> Result<Response> {
    // ...
}
```

### DIを使用したViewSetHandlerの設定

```rust
use reinhardt_di::{InjectionContext, SingletonScope};
use std::sync::Arc;

// Create DI context
let singleton = Arc::new(SingletonScope::new());
let ctx = Arc::new(InjectionContext::new(singleton));

// Create ViewSet handler with DI
let handler = ViewSetHandler::new(
    Arc::new(viewset),
    action_map,
    None,
    None,
).with_di_context(ctx);

// Now the handler will automatically inject dependencies
```
