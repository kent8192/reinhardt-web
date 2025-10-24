# チュートリアル 4: 認証とパーミッション

APIを保護するために、認証とパーミッションシステムを実装します。

## パーミッションシステム

Reinhardtは柔軟なパーミッションシステムを提供します。`reinhardt-auth`クレートで定義されています。

### 組み込みパーミッション

```rust
use reinhardt_auth::permissions::{
    Permission, PermissionContext,
    AllowAny, IsAuthenticated, IsAdminUser, IsActiveUser, IsAuthenticatedOrReadOnly
};

// 全てのリクエストを許可
let perm = AllowAny;

// 認証が必要
let perm = IsAuthenticated;

// 認証済みユーザーまたは読み取り専用
let perm = IsAuthenticatedOrReadOnly;

// 管理者のみ
let perm = IsAdminUser;

// アクティブなユーザーのみ
let perm = IsActiveUser;
```

### パーミッションの仕組み

パーミッションは`PermissionContext`を使用してチェックされます:

```rust
use reinhardt_auth::permissions::{Permission, PermissionContext};
use reinhardt_core::Request;
use async_trait::async_trait;

// カスタムパーミッションの例
struct IsOwner;

#[async_trait]
impl Permission for IsOwner {
    async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
        // カスタムロジックを実装
        context.is_authenticated && context.is_active
    }
}
```

### パーミッションの使用

ハンドラでパーミッションをチェック:

```rust
use reinhardt_core::{Request, Response, Result, Error};
use reinhardt_auth::permissions::{Permission, PermissionContext, IsAuthenticated};

async fn protected_handler(request: Request) -> Result<Response> {
    let permission = IsAuthenticated;

    // PermissionContextを作成（実際の実装では認証状態を確認）
    let context = PermissionContext {
        request: &request,
        is_authenticated: true,  // 実際の認証状態
        is_admin: false,
        is_active: true,
    };

    // パーミッションチェック
    if !permission.has_permission(&context).await {
        return Err(Error::Http("Forbidden".to_string()));
    }

    Response::ok().with_json(&serde_json::json!({"message": "Authorized"}))
}
```

## 複合パーミッション

複数のパーミッションを組み合わせる:

```rust
use reinhardt_auth::permissions::{AndPermission, OrPermission, NotPermission};

// 従来の方法: 明示的なコンストラクタ
let and_perm = AndPermission::new(IsAuthenticated, IsActiveUser);
let or_perm = OrPermission::new(IsAdminUser, IsOwner);
let not_perm = NotPermission::new(IsAuthenticated);
```

### 演算子を使用した簡潔な記法

Reinhardtは演算子を使用したパーミッション合成もサポートしています:

```rust
use reinhardt_auth::permissions::{IsAuthenticated, IsActiveUser, IsAdminUser};

// & 演算子: 全てのパーミッションが必要（AND）
let and_perm = IsAuthenticated & IsActiveUser;

// | 演算子: いずれかのパーミッションが必要（OR）
let or_perm = IsAdminUser | IsOwner;

// ! 演算子: パーミッションを反転（NOT）
let not_perm = !IsAuthenticated;

// 複雑な組み合わせ
// 意味: (認証済み かつ アクティブ) または 管理者
let complex_perm = (IsAuthenticated & IsActiveUser) | IsAdminUser;
```

## カスタムパーミッションの実装

オブジェクトレベルのパーミッション:

```rust
use reinhardt_auth::permissions::{Permission, PermissionContext};
use async_trait::async_trait;

struct IsOwnerOrReadOnly;

#[async_trait]
impl Permission for IsOwnerOrReadOnly {
    async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
        // 読み取りメソッドは誰でもOK
        if matches!(context.request.method.as_str(), "GET" | "HEAD" | "OPTIONS") {
            return true;
        }

        // 書き込みメソッドは認証済みかつアクティブなユーザーのみ
        context.is_authenticated && context.is_active
    }
}
```

## まとめ

このチュートリアルで学んだこと:

1. 組み込みパーミッションの使用
2. カスタムパーミッションの実装
3. PermissionContextの使用
4. 複合パーミッションの作成

次のチュートリアル: [チュートリアル 5: リレーションシップとハイパーリンクAPI](5-relationships-and-hyperlinked-apis.md)
