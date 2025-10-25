# reinhardt-urls

Reinhardtフレームワーク向けのURLルーティングおよびプロキシユーティリティ

## 概要

`reinhardt-urls`は、DjangoのURLシステムにインスパイアされた、Reinhardtアプリケーション向けの包括的なURLルーティングおよび遅延ロードプロキシ機能を提供します。この親クレートは、ルーター、ルーティングマクロ、遅延ロードプロキシユーティリティを統合し、強力なURL管理機能を提供します。

## 機能

### 実装済み ✓

この親クレートは、以下のサブクレートから機能を再エクスポートします:

- **ルーター** (`reinhardt-routers`): 自動URL ルーティング設定
  - DjangoにインスパイアされたURLルーティング
  - ViewSetの自動URL生成
  - 名前空間とバージョニングのサポート
  - URL逆引き機能
  - URLパターンマッチング用のPathPattern
  - 自動エンドポイント生成機能付きDefaultRouter
  - カスタムアクションのサポート(リストレベルおよび詳細レベル)

- **ルーターマクロ** (`reinhardt-routers-macros`): ルーティング関連の手続き型マクロ
  - コンパイル時のルート検証
  - 型安全なURLパターン生成
  - ルート登録マクロ

- **プロキシ** (`reinhardt-proxy`): 遅延ロードプロキシシステム
  - Djangoスタイルの SimpleLazyObject 実装
  - スレッドセーフな遅延評価
  - ORMとの統合による遅延モデルロード
  - 初回アクセス時の自動初期化
  - 複雑な初期化ロジックのサポート
  - 高度なプロキシ機能:
    - アソシエーションプロキシ(SQLAlchemyスタイル)
    - 比較操作を持つスカラープロキシ
    - リレーションシップ管理用のコレクションプロキシ
    - クエリフィルタリングとJoin操作
    - 遅延/即時ロード戦略
    - リレーションシップキャッシング

- **高度なURLパターンマッチング**:
  - `path!` マクロによるコンパイル時パス検証
  - パラメータ抽出を伴う実行時パターンマッチング
  - パス制約の検証(snake_caseパラメータ、二重スラッシュ禁止など)
  - 名前付きキャプチャグループを使用した正規表現ベースのURLマッチング

### 予定

- ルートミドルウェアのサポート

## インストール

`Cargo.toml`に以下を追加してください:

```toml
[dependencies]
reinhardt-urls = "0.1.0"
```

### オプション機能

必要に応じて特定のサブクレートを有効化できます:

```toml
[dependencies]
reinhardt-urls = { version = "0.1.0", features = ["routers", "proxy"] }
```

利用可能な機能:

- `routers` (デフォルト): URLルーティングシステム
- `routers-macros` (デフォルト): ルーティングマクロ
- `proxy` (デフォルト): 遅延ロードプロキシ

## 使用例

### URLルーティング

```rust
use reinhardt_urls::{Router, DefaultRouter, Route};

// ルーターを作成
let mut router = DefaultRouter::new();

// ViewSetを登録
router.register("users", UserViewSet::new());

// カスタムルートを追加
router.add_route(Route::new("/custom/", custom_handler));

// リクエストをマッチング
if let Some((handler, params)) = router.match_request(&request) {
    handler.handle(request, params).await?;
}
```

### URL逆引き

```rust
use reinhardt_urls::reverse;

// 名前によるURL逆引き
let url = reverse("user-detail", &[("id", "123")]);
// 返り値: /users/123/

// 名前空間付き
let url = reverse("api:v1:user-list", &[]);
// 返り値: /api/v1/users/
```

### 遅延ロードプロキシ

```rust
use reinhardt_proxy::SimpleLazyObject;

// 遅延オブジェクトを作成
let lazy_user = SimpleLazyObject::new(|| {
    // 重い初期化処理
    User::from_database(user_id)
});

// アクセス時に初期化がトリガーされる
let name = lazy_user.name; // ここで初期化が行われる
```

## サブクレート

この親クレートには以下のサブクレートが含まれています:

```
reinhardt-urls/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    ├── routers/         # URLルーティングシステム
    ├── routers-macros/  # ルーティング手続き型マクロ
    └── proxy/           # 遅延ロードプロキシ
```

## ライセンス

Apache License, Version 2.0 または MIT ライセンスのいずれかの条件の下でライセンスされています。
