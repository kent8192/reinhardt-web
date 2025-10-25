# reinhardt-filters

Reinhardtフレームワーク向けの型安全なフィルタリングと並び替え機能

## 概要

reinhardt-ormのFieldシステムを使用してコンパイル時に型安全なフィルタリングを提供する強力なフィルタリングおよび並び替えシステムです。フィールドルックアップ、マルチタームサーチ、並び替えを使用した複雑なクエリを、完全な型安全性とゼロランタイムオーバーヘッドで構築できます。

## 実装済み ✓

### コアフィルタシステム

- **FilterBackend trait** - クエリセット操作用の非同期フィルタリングインターフェース
- **FilterError** - 無効なパラメータとクエリに対する包括的なエラー処理
- **型安全なフィルタリング** - reinhardt-ormのField<M, T>を使用したコンパイル時チェック付きフィールドアクセス

### クエリフィルタリング

- **QueryFilter<M>** - ルックアップと並び替えを組み合わせた型安全なクエリフィルタ
  - `.add()` および `.add_all()` でルックアップ条件を追加
  - `.order_by()` および `.order_by_all()` で複数の並び替えフィールドを指定
  - `.add_or_group()` で複雑なクエリ用のORグループをサポート
  - `.add_multi_term()` でマルチタームサーチを統合
  - SQL WHERE句とORDER BY句の自動コンパイル
  - すべての条件はデフォルトでANDで結合

### フィールド並び替え

- **OrderingField<M>** - 方向を指定した型安全なフィールド並び替え
- **OrderDirection** - 昇順（Asc）または降順（Desc）の並び替え
- **FieldOrderingExt** - Field<M, T>に `.asc()` と `.desc()` を追加する拡張トレイト
- **SQL生成** - `.to_sql()` による自動ORDER BY句生成
- **ネストされたフィールドのサポート** - 複雑なフィールドパス（例：「author.username」）を処理

### マルチタームサーチ

- **MultiTermSearch** - 複数のフィールドにわたって複数の用語を検索
  - `.search_terms()` - 大文字小文字を区別しない部分一致検索（ICONTAINS）
  - `.exact_terms()` - 大文字小文字を区別しない完全一致（IEXACT）
  - `.prefix_terms()` - 前方一致検索（STARTSWITH）
  - `.parse_search_terms()` - 引用符サポート付きでカンマ区切りの検索文字列を解析
  - `.compile_to_sql()` - マルチタームサーチ用のSQL WHERE句を生成
- **クエリロジック** - 用語はANDで結合、各用語内のフィールドはORで結合

### 検索可能モデルシステム

- **SearchableModel trait** - モデルの検索可能フィールドとデフォルトの並び替えを定義
  - `.searchable_fields()` - テキスト検索をサポートする文字列フィールドを指定
  - `.default_ordering()` - モデルクエリのデフォルトソート順を定義
  - `.searchable_field_names()` - フィールド名を文字列として抽出するヘルパー

## 予定

### 高度なフィルタリング

- 日付と数値フィールドの範囲フィルタ
- 地理的/空間フィルタリング
- 全文検索統合
- 特殊なユースケース向けのカスタムフィルタバックエンド

### クエリ最適化

- クエリ結果のキャッシング
- インテリジェントなインデックス使用
- クエリプラン最適化ヒント

### 強化された検索

- ファジー検索サポート
- 関連性スコアリング
- 同義語処理
- 検索結果のハイライト

## 使用例

### 基本的なクエリフィルタリング

```rust
use reinhardt_filters::QueryFilter;
use reinhardt_orm::Field;

// ルックアップと並び替えを使用してフィルタを作成
let filter = QueryFilter::<Post>::new()
    .add(Field::new(vec!["title"]).icontains("rust"))
    .add(Field::new(vec!["created_at"]).year().gte(2024))
    .order_by(Field::new(vec!["title"]).asc());
```

### マルチタームサーチ

```rust
use reinhardt_filters::MultiTermSearch;

// 「rust」AND「programming」を含む投稿を検索
let terms = vec!["rust", "programming"];
let lookups = MultiTermSearch::search_terms::<Post>(terms);

// 生成されるクエリ: (title ICONTAINS 'rust' OR content ICONTAINS 'rust')
//        AND (title ICONTAINS 'programming' OR content ICONTAINS 'programming')
```

### 検索可能モデル

```rust
use reinhardt_filters::{SearchableModel, FieldOrderingExt};
use reinhardt_orm::{Model, Field};

impl SearchableModel for Post {
    fn searchable_fields() -> Vec<Field<Self, String>> {
        vec![
            Field::new(vec!["title"]),
            Field::new(vec!["content"]),
        ]
    }

    fn default_ordering() -> Vec<OrderingField<Self>> {
        vec![Field::new(vec!["created_at"]).desc()]
    }
}
```

## 統合

以下とシームレスに連携します：

- **reinhardt-orm** - 型安全なField<M, T>システムとQuerySet
- **reinhardt-viewsets** - ViewSet レスポンスでの自動フィルタリング
- **reinhardt-rest** - クエリパラメータの解析と検証
