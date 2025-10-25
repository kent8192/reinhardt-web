# reinhardt-views

DjangoのクラスベースビューとDjango REST Frameworkに触発された、Reinhardtフレームワーク向けのビュークラスとレスポンス処理。

## 概要

このクレートは、ReinhardtでRESTful APIを構築するためのビュー層を提供します。共通のCRUD操作のための基本ビュートレイト、具体的なビュー実装、およびOpenAPIスキーマ生成とブラウザブルAPIレンダリングのためのユーティリティが含まれています。

## 機能

### 実装済み ✓

#### コアビューインフラストラクチャ

- **Viewトレイト** - 非同期ディスパッチサポートを持つすべてのビューの基本トレイト
  - 自動OPTIONSメソッド処理
  - カスタマイズ可能な許可メソッド
  - 非同期リクエスト/レスポンス処理

#### クラスベースビュー

- **ListView** - ページネーション付きオブジェクトリストの表示
  - DRFスタイルのメタデータを持つ設定可能なページネーション
  - 複数フィールドでの並び替え（昇順/降順）
  - 空の結果セット処理
  - カスタムコンテキストオブジェクト名
  - 完全なシリアライザーサポート
  - HEADメソッドサポート
- **DetailView** - 単一オブジェクトの表示
  - 主キー（pk）による検索
  - スラッグベースの検索
  - QuerySet統合
  - カスタムコンテキストオブジェクト名
  - 完全なシリアライザーサポート
  - HEADメソッドサポート

#### ビューMixin

- **MultipleObjectMixin** - リストビュー用の共通機能
  - オブジェクト取得
  - 並び替え設定
  - ページネーション設定
  - コンテキストデータ構築
- **SingleObjectMixin** - 詳細ビュー用の共通機能
  - pk/slugによるオブジェクト取得
  - URLパラメータ設定
  - コンテキストデータ構築

#### ジェネリックAPIビュー（スタブ）

- **ListAPIView** - リストエンドポイント
- **CreateAPIView** - 作成エンドポイント
- **UpdateAPIView** - 更新エンドポイント
- **DestroyAPIView** - 削除エンドポイント
- **ListCreateAPIView** - リスト/作成の複合エンドポイント
- **RetrieveUpdateAPIView** - 取得/更新の複合エンドポイント
- **RetrieveDestroyAPIView** - 取得/削除の複合エンドポイント
- **RetrieveUpdateDestroyAPIView** - 取得/更新/削除の複合エンドポイント

#### OpenAPIスキーマ生成

- **OpenAPISpec** - OpenAPI 3.0仕様構造
- **Schema** - JSONスキーマ定義
- **PathItem** - HTTPメソッドを持つAPIパス定義
- **Operation** - HTTP操作メタデータ
- **Parameter** - リクエストパラメータ定義（クエリ、ヘッダー、パス、クッキー）
- **Response** - レスポンススキーマ定義
- **Components** - 再利用可能なスキーマコンポーネント
- **SchemaGenerator** - スキーマ生成ユーティリティ
- **EndpointInfo** - ドキュメント用のエンドポイントメタデータ

#### ブラウザブルAPI

- 基本的なレンダリングインフラストラクチャ（最小限のスタブ）

#### 管理インターフェース統合

- `reinhardt-contrib`から管理ビューを再エクスポート
- 管理変更ビューサポート
- Djangoスタイルの管理インターフェースとの統合

#### ViewSet（`reinhardt-viewsets`から）

- **ModelViewSet** - モデルの完全なCRUD操作
  - List、Create、Retrieve、Update、Partial Update、Destroyアクション
  - 自動HTTPメソッドマッピング
  - カスタムアクションサポート（`ActionType::Custom`）
- **ViewSet Builder** - ViewSet設定のためのFluent API
- **アクションシステム** - 型安全なアクション定義
  - 組み込みアクション（List、Retrieve、Create、Update、PartialUpdate、Destroy）
  - カスタムアクションサポート
  - 詳細/非詳細アクションの区別
- **ハンドラーシステム** - リクエストルーティングとディスパッチ
- **依存性注入** - ViewSetのフィールドとメソッドインジェクション
- **ミドルウェアサポート** - ViewSet固有のミドルウェア
- **レジストリ** - ViewSetの登録と検出

### 予定

#### 拡張ブラウザブルAPI

- API探索のためのHTMLレンダリング
- インタラクティブなAPIドキュメント
- POST/PUT/PATCHメソッド用のフォーム生成
- レスポンスのシンタックスハイライト

#### スキーマ生成の強化

- モデルからの自動スキーマ推論
- Rust型からの型安全なスキーマ生成
- リクエスト/レスポンスの例生成
- スキーマ検証ユーティリティ

#### 高度なViewSet機能

- ネストされたリソースの処理
- バッチ操作サポート
- 楽観的ロックサポート

#### テンプレートサポート

- テンプレートベースのレンダリング
- コンテキストプロセッサー
- テンプレート継承
- カスタムテンプレートローダー

## 使用方法

### ListViewの例

```rust
use reinhardt_views::{ListView, View};
use reinhardt_serializers::JsonSerializer;
use reinhardt_orm::Model;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
    id: Option<i64>,
    title: String,
    content: String,
}

impl Model for Article {
    type PrimaryKey = i64;
    fn table_name() -> &'static str { "articles" }
    fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
}

// ページネーション付きリストビューを作成
let articles = vec![
    Article { id: Some(1), title: "First".into(), content: "...".into() },
    Article { id: Some(2), title: "Second".into(), content: "...".into() },
];

let view = ListView::<Article, JsonSerializer<Article>>::new()
    .with_objects(articles)
    .with_paginate_by(10)
    .with_ordering(vec!["-id".into()])
    .with_context_object_name("articles");
```

### DetailViewの例

```rust
use reinhardt_views::{DetailView, View};
use reinhardt_serializers::JsonSerializer;

let article = Article {
    id: Some(1),
    title: "Hello".into(),
    content: "World".into()
};

let view = DetailView::<Article, JsonSerializer<Article>>::new()
    .with_object(article)
    .with_pk_url_kwarg("article_id")
    .with_context_object_name("article");
```

### OpenAPIスキーマ生成

```rust
use reinhardt_views::{OpenAPISpec, Info, PathItem, Operation};

let spec = OpenAPISpec::new(Info::new(
    "My API".into(),
    "1.0.0".into()
));
```

## 依存関係

- `reinhardt-apps` - Request/Responseタイプ
- `reinhardt-orm` - ORM統合
- `reinhardt-serializers` - シリアライゼーションサポート
- `reinhardt-exception` - エラーハンドリング
- `reinhardt-contrib` - 管理ビュー
- `async-trait` - 非同期トレイトサポート
- `serde` - シリアライゼーションフレームワーク
- `serde_json` - JSONシリアライゼーション

## テスト

このクレートには以下をカバーする包括的なユニットテストが含まれています：

- 基本的なビュー機能
- ListViewのページネーションと並び替え
- DetailViewのオブジェクト取得
- エラーハンドリング
- ViewSetパターン
- APIビューの動作
- 管理変更ビュー
