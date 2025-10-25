# reinhardt-macros

フレームワーク用の手続きマクロ

## 概要

ボイラープレートコードを削減するための手続きマクロです。モデル、シリアライザー、フォーム用の派生マクロ、エンドポイントとミドルウェア用の属性マクロが含まれます。

一般的なパターンのコンパイル時コード生成を提供します。

## 機能

### 実装済み ✓

#### 関数ベースAPIビュー

- **`#[api_view]`** - 関数をAPIビューに変換
  - `methods`パラメータによる複数HTTPメソッドのサポート
  - コンパイル時のHTTPメソッド検証（GET、POST、PUT、PATCH、DELETE、HEAD、OPTIONS）
  - メソッド未指定時はGETがデフォルト
  - 例: `#[api_view(methods = "GET,POST")]`

#### ViewSetカスタムアクション

- **`#[action]`** - ViewSetのカスタムアクションを定義
  - `methods`パラメータによるHTTPメソッド指定のサポート
  - `detail`パラメータによる詳細/リストアクションのサポート（必須）
  - オプションの`url_path`と`url_name`パラメータ
  - コンパイル時のHTTPメソッド検証
  - 例: `#[action(methods = "POST", detail = true)]`

#### HTTPメソッドデコレータ

- **`#[get]`** - パス検証付きGETメソッドデコレータ
- **`#[post]`** - パス検証付きPOSTメソッドデコレータ
- **`#[put]`** - パス検証付きPUTメソッドデコレータ
- **`#[patch]`** - パス検証付きPATCHメソッドデコレータ
- **`#[delete]`** - パス検証付きDELETEメソッドデコレータ
- すべてコンパイル時のURLパターン検証をサポート
- 例: `#[get("/users/{id}")]`

#### 権限システム

- **`#[permission_required]`** - 権限デコレータ
  - コンパイル時の権限文字列検証
  - Django形式の権限フォーマットをサポート: `"app.permission"`
  - nomパーサーを使用した検証
  - 例: `#[permission_required("users.view_user")]`

#### 依存性注入（FastAPI風）

- **`#[use_injection]`** / **`#[endpoint]`** - 自動依存性注入
  - `#[inject]`を使用したFastAPI風のパラメータ属性
  - `InjectionContext`からの自動解決
  - `#[inject(cache = false)]`によるキャッシュ制御
  - エンドポイントだけでなく、任意の関数で動作
  - 例: `#[use_injection] async fn handler(#[inject] db: Database)`

#### 設定マクロ

- **`installed_apps!`** - インストール済みアプリケーションを定義
  - コンパイル時のアプリケーションパス検証
  - インストール済みすべてのアプリの型安全なenum生成
  - `reinhardt.contrib.*`モジュールの存在を検証
  - `Display`と`FromStr`の実装を生成
  - 例: `installed_apps! { auth: "reinhardt.contrib.auth", }`

#### URLパターン検証

- **`path!`** - コンパイル時のURLパターン検証
  - nomパーサーを使用したパターン検証
  - シンプルなパラメータのサポート: `{id}`
  - Django形式の型付きパラメータのサポート: `{<int:id>}`
  - パラメータ名と型指定子の検証
  - サポートされる型: `int`、`str`、`uuid`、`slug`、`path`
  - 位置情報付きの詳細なエラーメッセージ
  - 例: `path!("users/{<int:user_id>}/posts/{post_id}/")`

#### シグナルシステム

- **`#[receiver]`** - レシーバー関数をシグナルに接続
  - Django風の`@receiver`デコレータ機能
  - シグナルとsenderパラメータのサポート
  - シグナル登録用のマーカーマクロ
  - 例: `#[receiver(signal = post_save::<User>())]`

#### 型安全なクエリフィールド

- **`#[derive(QueryFields)]`** - フィールドアクセサメソッドの生成
  - モデル用の自動フィールドアクセサ生成
  - コンパイル時に検証されたフィールドルックアップ
  - フィールド型に基づく型固有のルックアップメソッド
  - 文字列フィールド: `lower()`、`upper()`、`trim()`、`contains()`
  - 数値フィールド: `abs()`、`ceil()`、`floor()`、`round()`
  - DateTimeフィールド: `year()`、`month()`、`day()`、`hour()`
  - すべてのフィールド: `eq()`、`ne()`、`gt()`、`gte()`、`lt()`、`lte()`
  - 例: `QuerySet::<User>::new().filter(User::email().lower().contains("example.com"))`

### 予定

現在、予定されている機能はすべて実装済みです。
