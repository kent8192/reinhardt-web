# reinhardt-proxy

SQLAlchemy形式のアソシエーションプロキシによるリレーションを通じた透過的な属性アクセス

## 概要

アソシエーションプロキシを使用すると、関連オブジェクトの属性を、あたかも親オブジェクトの属性であるかのようにアクセスできます。これは、関連オブジェクトの属性を直接操作したい多対多のリレーションシップで特に便利です。

## 機能

### 実装済み ✓

#### コアアソシエーションプロキシ（`proxy.rs`）

- `AssociationProxy<T, U>` - リレーションシップトラバーサル用のメインプロキシタイプ
- `ProxyAccessor` トレイト - プロキシターゲットの取得/設定用インターフェース
- `ProxyTarget` enum - スカラーまたはコレクションプロキシ結果を表現
- `ScalarValue` enum - 型安全なスカラー値の表現（String、Integer、Float、Boolean、Null）
- 新しいアソシエーション用のクリエーター関数サポート
- 包括的な型変換メソッド（`as_string()`、`as_integer()`、`as_float()`、`as_boolean()`）

#### スカラープロキシ（`scalar.rs`）

- `ScalarProxy` - 一対一および多対一のリレーションシップ用
- `ScalarComparison` enum - 豊富な比較演算子（Eq、Ne、Gt、Gte、Lt、Lte、In、NotIn、IsNull、IsNotNull、Like、NotLike）
- スカラー値の非同期取得/設定操作
- すべての比較タイプ用のビルダーメソッド

#### コレクションプロキシ（`collection.rs`）

- `CollectionProxy` - 一対多および多対多のリレーションシップ用
- 重複排除付きのユニーク値サポート
- コレクション操作メソッド:
  - `get_values()` - 関連オブジェクトからすべての値を抽出
  - `set_values()` - コレクション全体を置換
  - `append()` - 単一の値を追加
  - `remove()` - 一致する値を削除
  - `contains()` - 値の存在チェック
  - `count()` - コレクションサイズの取得
- 高度なフィルタリング:
  - `filter()` - FilterConditionでフィルタリング
  - `filter_by()` - カスタム述語でフィルタリング
- `CollectionOperations` - 変換操作のラッパー（filter、map、sort、distinct）
- `CollectionAggregations` - 集約操作のラッパー（sum、avg、min、max）

#### クエリフィルタリング（`query.rs`）

- `FilterOp` enum - フィルター操作（Eq、Ne、Lt、Le、Gt、Ge、In、NotIn、Contains、StartsWith、EndsWith）
- `FilterCondition` - フィールド、演算子、値を含む条件
- `QueryFilter` - 複数の条件のコンテナ
- ScalarValueに対して条件を評価する`matches()`メソッド

#### 結合操作（`joins.rs`）

- `JoinConfig` - 即時/遅延ロードの設定
- `LoadingStrategy` enum - Eager、Lazy、Select戦略
- `NestedProxy` - 多層リレーションシップトラバーサル
- `RelationshipPath` - リレーションシップのパス表現
- ヘルパー関数:
  - `extract_through_path()` - ドット区切りパスの解析
  - `filter_through_path()` - パスセグメントのフィルタリング
  - `traverse_and_extract()` - ネストされたプロキシからの抽出
  - `traverse_relationships()` - リレーションシップパスのナビゲーション

#### ビルダーパターン（`builder.rs`）

- `ProxyBuilder<T, U>` - プロキシ構築用のフルエントAPI
- 設定のためのメソッドチェーン:
  - `relationship()` - リレーションシップ名を設定
  - `attribute()` - 属性名を設定
  - `creator()` - クリエーター関数を設定
- 安全な構築メソッド:
  - `build()` - 設定不足時にパニックでビルド
  - `try_build()` - Optionを返すビルド
- `association_proxy()`ヘルパー関数

#### リフレクションシステム（`reflection.rs`）

- `Reflectable` トレイト - ランタイムイントロスペクション用のコアトレイト
  - `get_relationship()` / `get_relationship_mut()` - リレーションシップへのアクセス
  - `get_attribute()` / `set_attribute()` - 属性へのアクセス
  - `get_relationship_attribute()` / `set_relationship_attribute()` - ネストされたアクセス
  - `has_relationship()` / `has_attribute()` - 存在チェック
- `ProxyCollection` トレイト - 統一されたコレクションインターフェース
  - `Vec<T>`のジェネリック実装
  - メソッド: `items()`、`add()`、`remove()`、`contains()`、`len()`、`clear()`
- `AttributeExtractor` トレイト - スカラー値抽出インターフェース
- ヘルパー関数:
  - `downcast_relationship()` - 型安全なダウンキャスト
  - `extract_collection_values()` - 一括値抽出

#### エラー処理

- 包括的なエラータイプを持つ`ProxyError` enum:
  - `RelationshipNotFound` - リレーションシップが見つからない
  - `AttributeNotFound` - 属性が見つからない
  - `TypeMismatch` - 型変換エラー
  - `InvalidConfiguration` - 設定エラー
  - `DatabaseError` - データベース操作エラー
  - `SerializationError` - シリアライゼーションエラー
- `ProxyResult<T>` ProxyErrorを含むResult用の型エイリアス

### 予定

現在、すべての予定機能は実装済みです。
