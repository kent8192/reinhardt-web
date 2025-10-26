# クイックスタート

システム内のユーザーとグループを閲覧・編集できる、管理者向けのシンプルなAPIを作成します。

## プロジェクトのセットアップ

まず、グローバルツールをインストールします：

```bash
cargo install reinhardt-admin
```

tutorialという名前の新しいReinhardtプロジェクトを作成します：

```bash
# RESTful APIプロジェクトを作成
reinhardt-admin startproject tutorial --template-type restful
cd tutorial
```

これにより、以下のプロジェクト構造が生成されます：

```
tutorial/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs
    ├── config.rs
    ├── apps.rs
    ├── config/
    │   ├── settings.rs
    │   ├── settings/
    │   │   ├── base.rs
    │   │   ├── local.rs
    │   │   ├── staging.rs
    │   │   └── production.rs
    │   ├── urls.rs
    │   └── apps.rs
    └── bin/
        ├── runserver.rs
        └── manage.rs
```

生成された`Cargo.toml`には、REST API開発に必要なすべての依存関係が既に含まれています。

## モデル

このクイックスタートでは、Reinhardtの組み込み`User`と`Group`モデルを使用します。これらはauth機能から提供されます。

## シリアライザ

データ表現用のシリアライザを定義します。`src/main.rs`に以下を追加します:

```rust
use reinhardt::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSerializer {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupSerializer {
    pub id: i64,
    pub name: String,
}
```

この例ではシンプルなデータ構造を使用しています。実際のアプリケーションでは、`Serializer`トレイトを実装してバリデーションとデータ変換ロジックを追加できます。

## ViewSets

ViewSetを使用してCRUD操作を実装します。`src/main.rs`に追加:

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

// UserViewSet - 完全なCRUD操作
let user_viewset = ModelViewSet::<User, UserSerializer>::new("user");

// GroupViewSet - 読み取り専用
let group_viewset = ReadOnlyModelViewSet::<Group, GroupSerializer>::new("group");
```

`ModelViewSet`はすべての標準的なCRUD操作（list、retrieve、create、update、delete）を提供します。`ReadOnlyModelViewSet`はlistとretrieve操作のみを提供します。

## ルーティング

まず、usersアプリを作成します：

```bash
cargo run --bin manage startapp users --template-type restful
```

### モデルとシリアライザの定義

`users/models.rs`を編集：

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}
```

`users/serializers.rs`を編集：

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSerializer {
    pub id: i64,
    pub username: String,
    pub email: String,
}
```

### ViewSetの作成

`users/views.rs`を編集：

```rust
use reinhardt::viewsets::ModelViewSet;
use crate::models::User;
use crate::serializers::UserSerializer;

pub struct UserViewSet;

impl UserViewSet {
    pub fn new() -> ModelViewSet<User, UserSerializer> {
        ModelViewSet::new("user")
    }
}
```

### URLの設定

`users/urls.rs`を編集：

```rust
use reinhardt_routers::UnifiedRouter;
use crate::views::UserViewSet;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    // ViewSetを登録 - CRUD エンドポイントが自動生成されます
    router.register_viewset("users", UserViewSet::new());

    router
}
```

### プロジェクトURLへの登録

`src/config/urls.rs`を編集：

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder()
        .build();

    // usersアプリのルーターを含める
    router.include_router("/api/", users::urls::url_patterns(), Some("users".to_string()));

    Arc::new(router)
}
```

`src/config/apps.rs`を編集：

```rust
use reinhardt_macros::installed_apps;

installed_apps! {
    users: "users",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

これで以下のURLパターンが自動的に生成されます:

- `GET /api/users/` - ユーザーのリスト
- `POST /api/users/` - 新しいユーザーの作成
- `GET /api/users/{id}/` - 特定のユーザーの取得
- `PUT /api/users/{id}/` - ユーザーの更新
- `PATCH /api/users/{id}/` - ユーザーの部分更新
- `DELETE /api/users/{id}/` - ユーザーの削除

## パーミッション（オプション）

認証とパーミッションを追加するには、`reinhardt::auth::permissions`モジュールを使用します:

```rust
use reinhardt::auth::permissions::{IsAuthenticated, IsAuthenticatedOrReadOnly};

// ViewSet作成時にパーミッションを設定できます
// 注: 現在の実装では、カスタムViewSet実装が必要です
```

## ページネーション（オプション）

大量のデータを扱う場合は、ページネーションを実装できます:

```rust
use reinhardt::rest::pagination::PageNumberPagination;

// ページネーションの設定例
let pagination = PageNumberPagination::new(10); // 1ページあたり10件
```

## APIのテスト

まず、開発サーバーを起動します：

```bash
cargo run --bin runserver
```

APIをテストするには、curlまたはhttpieを使用します:

```bash
# ユーザーのリストを取得
curl http://127.0.0.1:8000/api/users/

# 新しいユーザーを作成
curl -X POST http://127.0.0.1:8000/api/users/ \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com"}'

# 特定のユーザーを取得
curl http://127.0.0.1:8000/api/users/1/

# ユーザーを更新
curl -X PUT http://127.0.0.1:8000/api/users/1/ \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"newemail@example.com"}'

# ユーザーを削除
curl -X DELETE http://127.0.0.1:8000/api/users/1/
```

## まとめ

このクイックスタートでは、以下を学びました:

1. Reinhardtプロジェクトのセットアップ
2. シリアライザの定義
3. ViewSetを使用したCRUD APIの作成
4. ルーターを使用した自動URL生成

より詳細な情報は、[チュートリアル](1-serialization.md)を参照してください。
