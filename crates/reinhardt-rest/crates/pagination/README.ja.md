# reinhardt-pagination

Django REST Frameworkのページネーションにインスパイアされた、Reinhardtフレームワーク用のページネーション戦略。

## 概要

大規模データセット向けの複数のページネーション戦略を提供します。主に3つのページネーションスタイルを提供:PageNumberPaginationは従来のページベースのページネーション、LimitOffsetPaginationはSQLスタイルのlimit/offset、CursorPaginationはオフセットのパフォーマンス問題なく大規模データセットを効率的にページネーションします。

## 実装済み ✓

### コア機能

- **PaginatedResponse** - カウント、次/前のリンク、および結果を含む汎用的なページネーションレスポンスラッパー
- **PaginationMetadata** - APIレスポンス用のページネーションメタデータ構造
- **Page** - DjangoライクなAPIを持つ包括的なページ表現
  - ナビゲーション: `has_next()`, `has_previous()`, `has_other_pages()`
  - ページ番号: `next_page_number()`, `previous_page_number()`, `page_range()`
  - 省略範囲: `get_elided_page_range()` - 長いページリストを省略記号で短縮
  - インデックス: `start_index()`, `end_index()`, `len()`, `is_empty()`
  - 直接アクセス: `get()`, `get_slice()`, Indexトレイトサポート, IntoIteratorサポート

### ページネーション戦略

#### PageNumberPagination

ページ番号を使用した従来のページベースのページネーション。

- URL形式: `?page=2&page_size=10`
- ページサイズと最大ページサイズの設定可能
- カスタムクエリパラメータ名
- orphansサポート: 小さい最終ページを前のページにマージ
- 最終ページショートカット: "last"キーワードのサポート
- 空の最初のページの処理
- 寛容な`get_page()`: 無効な入力でも有効なページを返す
- カスタムエラーメッセージ
- 非同期サポート: `aget_page()`と`apaginate()`

#### LimitOffsetPagination

SQLスタイルのlimit/offsetページネーション。

- URL形式: `?limit=10&offset=20`
- デフォルトと最大limitの設定可能
- カスタムクエリパラメータ名
- 次/前のリンクの自動URL構築
- limitとoffset値の入力検証
- 非同期サポート: `apaginate()`

#### CursorPagination

大規模で変化するデータセットで一貫した結果を得るためのカーソルベースのページネーション。

- URL形式: `?cursor=<encoded_cursor>&page_size=10`
- 不透明なカーソルトークン(チェックサム付きbase64エンコード)
- カーソルセキュリティ: タイムスタンプ検証とチェックサム検証
- カーソル有効期限: 24時間の自動期限切れ
- 最大制限付きの設定可能なページサイズ
- カスタム順序付けフィールドのサポート
- 改ざん防止カーソルエンコーディング
- 非同期サポート: `apaginate()`

### トレイト

- **Paginator** - カスタム実装用の同期ページネーショントレイト
- **AsyncPaginator** - `apaginate()`メソッドを持つ非同期ページネーショントレイト
- **SchemaParameter** - OpenAPI/ドキュメント生成スキーマのサポート

### ビルダーパターン

すべてのページネーション戦略は流暢なビルダーパターンをサポートします:

```rust
// PageNumberPagination
let paginator = PageNumberPagination::new()
    .page_size(20)
    .max_page_size(100)
    .page_size_query_param("limit")
    .orphans(3)
    .allow_empty_first_page(false);

// LimitOffsetPagination
let paginator = LimitOffsetPagination::new()
    .default_limit(25)
    .max_limit(100);

// CursorPagination
let paginator = CursorPagination::new()
    .page_size(20)
    .max_page_size(50)
    .ordering(vec!["-created_at".to_string(), "id".to_string()]);
```

## 予定

- 直接QuerySetページネーション用のデータベース統合
- カスタムカーソルエンコーディング戦略
- 設定可能なカーソル有効期限
- 双方向カーソルページネーション
- Relayスタイルのカーソルページネーション
- カーソルページネーション用のカスタム順序付け戦略
- 非常に大規模なデータセット向けのパフォーマンス最適化
