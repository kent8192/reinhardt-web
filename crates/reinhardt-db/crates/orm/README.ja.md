# reinhardt-orm

クエリセットAPIとデータベース抽象化を備えたDjango風ORM

## 概要

DjangoのORMとSQLAlchemyからインスピレーションを得た強力なオブジェクトリレーショナルマッピングシステムです。チェイン可能なクエリのためのクエリセットAPI、モデル定義、フィールド型、バリデータ、リレーションシップ管理、複数のデータベースバックエンド(PostgreSQL、MySQL、SQLite)のサポートなどの機能を提供します。

## 実装済み ✓

### コアモデルシステム

- **Model trait** - コンポジションベースの設計によるデータベースモデルのコアトレイト
- **Timestamped trait** - created_at/updated_atタイムスタンプの自動管理
- **SoftDeletable trait** - deleted_atタイムスタンプによる論理削除機能
- **Timestamps struct** - コンポーザブルなタイムスタンプフィールド(created_at、updated_at)
- **SoftDelete struct** - 復元機能付きのコンポーザブルな論理削除フィールド

### フィールド型

- **AutoField** - 自動インクリメント整数型主キー
- **BigIntegerField** - 64ビット整数フィールド
- **BooleanField** - デフォルト値サポート付き真偽値フィールド
- **CharField** - max_length、null/blankオプション、選択肢を持つテキストフィールド
- **IntegerField** - 選択肢サポート付き標準整数フィールド
- **DateField** - auto_nowとauto_now_addオプション付き日付フィールド
- **DateTimeField** - auto_nowとauto_now_addオプション付き日時フィールド
- **DecimalField** - 精度設定(max_digits、decimal_places)付き10進数フィールド
- **EmailField** - バリデーションとカスタマイズ可能なmax_length付きメールフィールド
- **FloatField** - 浮動小数点数フィールド
- **TextField** - 大きなテキストフィールド
- **TimeField** - auto_nowオプション付き時刻フィールド
- **URLField** - バリデーション付きURLフィールド
- **BinaryField** - 生のバイナリデータフィールド(デフォルトで編集不可)
- **SlugField** - db_index付きURL対応文字列フィールド
- **SmallIntegerField** - 小整数フィールド(-32768から32767)
- **PositiveIntegerField** - 正の整数フィールド(0から2147483647)
- **PositiveSmallIntegerField** - 小正の整数フィールド(0から32767)
- **PositiveBigIntegerField** - 大きな正の整数フィールド
- **GenericIPAddressField** - プロトコルフィルタリング付きIPv4/IPv6アドレスフィールド
- **FilePathField** - パターンマッチング付きファイルシステムパス選択フィールド

### PostgreSQL固有フィールド

- **ArrayField** - PostgreSQL配列型サポート
- **JSONBField** - PostgreSQL JSONB型サポート
- **HStoreField** - PostgreSQLキーバリューストアフィールド
- **CITextField** - 大文字小文字を区別しないテキストフィールド
- **IntegerRangeField** - 整数範囲フィールド
- **BigIntegerRangeField** - 大整数範囲フィールド
- **DateRangeField** - 日付範囲フィールド
- **DateTimeRangeField** - 日時範囲フィールド

### リレーションシップフィールド

- **ForeignKey** - on_deleteオプション付き多対一リレーションシップ
- **OneToOneField** - 一対一リレーションシップ
- **ManyToManyField** - 中間テーブルサポート付き多対多リレーションシップ

### フィールド設定

- **BaseField** - 共通フィールド属性(null、blank、default、db_default、db_column、db_tablespace、primary_key、unique、editable、choices)
- **Field deconstruction** - マイグレーション用のシリアライズ可能なフィールド表現

### バリデータ

- **RequiredValidator** - 必須フィールドバリデーション
- **MinLengthValidator** - 最小長バリデーション
- **MaxLengthValidator** - 最大長バリデーション
- **RangeValidator** - 数値範囲バリデーション
- **RegexValidator** - 正規表現パターンバリデーション
- **EmailValidator** - メールアドレス形式バリデーション
- **URLValidator** - URL形式バリデーション
- **FieldValidators** - フィールドレベルバリデーションコンテナ
- **ModelValidators** - モデルレベルバリデーションコンテナ

### クエリシステム

- **QuerySet** - フィルタリング機能付きチェイン可能なクエリインターフェース
- **Filter** - 演算子によるクエリフィルタリング(Eq、Ne、Gt、Gte、Lt、Lte、In、NotIn、Contains、StartsWith、EndsWith)
- **Query** - クエリ構築と実行
- **FilterOperator** - フィルタリング用比較演算子
- **FilterValue** - 型安全なフィルタ値処理(String、Integer、Float、Boolean、Null)
- **select_related** - JOINクエリを使用した関連オブジェクトの即時ロード
- **prefetch_related** - 個別クエリを使用した関連オブジェクトの即時ロード
- **create()** - 新規レコード作成(`django-compat`フィーチャーが必要)

### データベースマネージャー(Django互換)

- **Manager** - データベース操作用のDjangoスタイルモデルマネージャー
- **all()** - 全レコードをクエリセットとして取得
- **filter()** - フィールドと演算子によるレコードフィルタリング
- **get()** - 主キーによる単一レコード取得
- **create()** - 新規レコード作成
- **update()** - 既存レコード更新
- **delete()** - 主キーによるレコード削除
- **count()** - レコード数カウント
- **bulk_create()** - 競合処理付きバッチによる効率的な複数レコード作成
- **bulk_update()** - バッチによる効率的な複数レコード更新
- **get_or_create()** - アトミック操作による既存レコード取得または新規作成
- **グローバルデータベース接続** - 接続管理用のinit_database()とget_connection()

### 式とクエリフィールド

- **Q** - AND/OR論理によるコンプレックスクエリ式
- **F** - フィールド参照式
- **Subquery** - サブクエリ式
- **Exists** - EXISTS句サポート
- **OuterRef** - 外部クエリフィールドへの参照
- **QOperator** - クエリ演算子(And、Or、Not)
- **Field** - クエリフィールド表現
- **Lookup** - フィールドルックアップ操作(exact、iexact、contains、icontains、in、gt、gte、lt、lte、startswith、istartswith、endswith、iendswith、range、isnull、regex、iregex)
- **LookupType** - 型付きルックアップ操作
- **Comparable** - 型安全な比較操作
- **StringType、NumericType、DateTimeType** - 型固有の操作

### 関数

- **集約関数** - Abs、Ceil、Floor、Round、Power、Sqrt、Mod
- **文字列関数** - Concat、Length、Lower、Upper、Substr、Trim(TrimType付き)
- **日付/時刻関数** - CurrentDate、CurrentTime、Now、Extract(ExtractComponent付き)
- **ユーティリティ関数** - Cast(SqlType付き)、Coalesce、NullIf、Greatest、Least

### ウィンドウ関数

- **Window** - ウィンドウ関数サポート
- **Frame** - フレーム仕様(FrameType、FrameBoundary)
- **ランキング関数** - RowNumber、Rank、DenseRank、NTile
- **値関数** - FirstValue、LastValue、NthValue、Lead、Lag

### アノテーションと集約

- **Annotation** - クエリアノテーション
- **Expression** - 値式
- **Value** - クエリ内のリテラル値
- **When** - 条件式

### 集合演算

- **SetOperation** - UNION、INTERSECT、EXCEPT演算
- **CombinedQuery** - 結合されたクエリ結果
- **SetOperationBuilder** - 集合演算用流暢なAPI

### トランザクション

- **Transaction** - SQL生成機能付きデータベーストランザクション管理
- **TransactionScope** - ドロップ時の自動ロールバック付きRAIIトランザクションガード
- **IsolationLevel** - トランザクション分離レベル(ReadUncommitted、ReadCommitted、RepeatableRead、Serializable)
- **TransactionState** - トランザクション状態追跡(NotStarted、Active、Committed、RolledBack)
- **Savepoint** - SQL生成機能付きネストトランザクションセーブポイント
- **atomic()** - トランザクション内でコードを実行するヘルパー関数
- **atomic_with_isolation()** - 特定の分離レベルでのアトミック実行
- **Atomic** - アトミックトランザクションコンテキスト(レガシー)

### データベース接続

- **DatabaseConnection** - トランザクションサポート付き接続抽象化
  - `begin_transaction()` - トランザクション開始
  - `begin_transaction_with_isolation()` - 特定の分離レベルで開始
  - `commit_transaction()` - 現在のトランザクションをコミット
  - `rollback_transaction()` - 現在のトランザクションをロールバック
  - `savepoint()` - ネストトランザクション用セーブポイント作成
  - `release_savepoint()` - セーブポイント解放
  - `rollback_to_savepoint()` - セーブポイントへロールバック
- **DatabaseExecutor** - クエリ実行トレイト
- **DatabaseBackend** - 複数データベースサポート(PostgreSQL、MySQL、SQLite)
- **QueryRow** - クエリ結果行表現

### インデックス

- **Index** - 基本インデックスサポート
- **BTreeIndex** - 順序付きデータ用B-treeインデックス
- **HashIndex** - 完全一致用ハッシュインデックス
- **GinIndex** - 全文検索用PostgreSQL GINインデックス
- **GistIndex** - 幾何データ用PostgreSQL GiSTインデックス

### 制約

- **Constraint** - 基本制約トレイト
- **UniqueConstraint** - 一意フィールド制約
- **CheckConstraint** - 条件付きCHECK制約
- **ForeignKeyConstraint** - 外部キー制約
- **OnDelete** - カスケード削除動作(Cascade、SetNull、SetDefault、Restrict、NoAction)
- **OnUpdate** - カスケード更新動作(Cascade、SetNull、SetDefault、Restrict、NoAction)

### リレーションシップ(SQLAlchemy風)

- **Relationship** - リレーションシップ設定
- **RelationshipType** - OneToOne、OneToMany、ManyToOne、ManyToMany
- **RelationshipDirection** - 双方向リレーションシップサポート
- **CascadeOption** - カスケード操作(All、Delete、SaveUpdate、Merge、Expunge、DeleteOrphan、Refresh)

### ロード戦略

- **LoadingStrategy** - イーガーローディングvs遅延ローディング
- **LoadOption** - ロードオプション設定
- **LoadOptionBuilder** - ロードオプション用流暢なAPI
- **LoadContext** - ロードコンテキスト管理
- **selectinload** - 個別SELECTでリレーションシップをロード
- **joinedload** - JOINでリレーションシップをロード
- **subqueryload** - サブクエリでリレーションシップをロード
- **lazyload** - リレーションシップを遅延ロード
- **noload** - リレーションシップをロードしない
- **raiseload** - リレーションシップアクセス時にエラー発生

### イベントシステム

- **EventRegistry** - グローバルイベント登録
- **EventListener** - イベントリスナートレイト
- **EventResult** - イベント処理結果
- **MapperEvents** - モデルマッピングイベント
- **SessionEvents** - セッションライフサイクルイベント
- **AttributeEvents** - 属性変更イベント
- **InstanceEvents** - インスタンスライフサイクルイベント

### クエリ実行

- **QueryExecution** - クエリ実行インターフェース
- **ExecutionResult** - 実行結果
- **SelectExecution** - SELECTクエリ実行
- **QueryCompiler** - SQLへのクエリコンパイル
- **ExecutableQuery** - 実行可能なクエリトレイト
- **QueryFieldCompiler** - フィールドレベルクエリコンパイル

### 型システム

- **SqlValue** - SQL値型
- **SqlTypeDefinition** - SQL型定義
- **TypeRegistry** - 型登録システム
- **TypeDecorator** - カスタム型デコレータ
- **DatabaseDialect** - 方言固有の型処理
- **UuidType** - UUID型サポート
- **JsonType** - JSON型サポート
- **ArrayType** - 配列型サポート
- **HstoreType** - PostgreSQL HStore型サポート
- **InetType** - IPアドレス型サポート
- **TypeError** - 型変換エラー

### レジストリシステム

- **MapperRegistry** - モデルマッパー登録
- **Mapper** - モデル対テーブルマッピング
- **TableInfo** - テーブルメタデータ
- **ColumnInfo** - カラムメタデータ
- **registry()** - グローバルレジストリアクセス

### SQLAlchemyスタイルクエリAPI

- **SelectQuery** - SQLAlchemyスタイルSELECTクエリ
- **select()** - SELECTクエリ作成
- **column()** - クエリ内カラム参照
- **SqlColumn** - カラム表現
- **JoinType** - 結合型(Inner、Left、Right、Full、Cross)

### エンジンと接続管理

- **Engine** - データベースエンジン
- **EngineConfig** - エンジン設定
- **create_engine()** - データベースエンジン作成
- **create_engine_with_config()** - 設定付きエンジン作成

### クエリオプション

- **QueryOptions** - クエリ実行オプション
- **QueryOptionsBuilder** - クエリオプション用流暢なAPI
- **ExecutionOptions** - 実行固有オプション
- **ForUpdateMode** - 行ロックモード(NoWait、SkipLocked、Update、KeyShare)
- **CompiledCacheOption** - クエリコンパイルキャッシング

### 非同期クエリサポート

- **AsyncQuery** - 非同期クエリ実行
- **AsyncSession** - 非同期セッション管理

### 多対多サポート

- **ManyToMany** - 多対多リレーションシップヘルパー
- **AssociationTable** - 中間テーブル表現
- **association_table()** - 中間テーブル作成

### バルク操作

- **bulk_update** - フィールド指定による効率的なバルク更新

### 型付き結合

- **TypedJoin** - 型安全な結合操作

### 複合主キー

- **composite_primary_key()** - 複数フィールドを複合主キーとして定義
- **get_composite_pk_values()** - HashMapとして複合主キー値を取得
- **get_composite()** - 複合主キー値によるクエリ

例:
```rust
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Clone, Debug)]
#[model(app_label = "test_app", table_name = "post_tags")]
struct PostTag {
    #[field(primary_key = true)]
    post_id: i64,

    #[field(primary_key = true)]
    tag_id: i64,

    #[field(max_length = 200)]
    description: String,
}

// 複合主キーメタデータへのアクセス
let composite_pk = PostTag::composite_primary_key();
assert!(composite_pk.is_some());

// インスタンスから複合主キー値を取得
let post_tag = PostTag { post_id: 1, tag_id: 5, description: "Tech".to_string() };
let pk_values = post_tag.get_composite_pk_values();
```

### データベースインデックス

- **index** - `#[field(index = true)]`によるデータベースインデックス用フィールドマーク
- **index_metadata()** - モデルフィールドのインデックス情報取得

例:
```rust
#[derive(Model, Serialize, Deserialize, Clone, Debug)]
#[model(app_label = "test_app", table_name = "users")]
struct User {
    #[field(primary_key = true)]
    id: i64,

    #[field(index = true, max_length = 100)]
    email: String,

    #[field(index = true, max_length = 50)]
    username: String,
}

// インデックスメタデータへのアクセス
let indexes = User::index_metadata();
assert_eq!(indexes.len(), 2);
```

### CHECK制約

- **check** - `#[field(check = "expression")]`によるCHECK制約の定義
- **constraint_metadata()** - モデルフィールドの制約情報取得
- **ConstraintType** - 制約型(Check、ForeignKey、Unique)

例:
```rust
#[derive(Model, Serialize, Deserialize, Clone, Debug)]
#[model(app_label = "test_app", table_name = "products")]
struct Product {
    #[field(primary_key = true)]
    id: i64,

    #[field(max_length = 100)]
    name: String,

    #[field(check = "price > 0")]
    price: f64,

    #[field(check = "quantity >= 0")]
    quantity: i32,
}

// 制約メタデータへのアクセス
let constraints = Product::constraint_metadata();
let price_constraint = constraints.iter()
    .find(|c| c.name == "price_check")
    .expect("price_check constraint should exist");
assert_eq!(price_constraint.definition, "price > 0");
```

### フィールドバリデータ

- **email** - `#[field(email = true)]`によるメール形式バリデーション
- **url** - `#[field(url = true)]`によるURL形式バリデーション
- **min_length** - `#[field(min_length = N)]`による文字列最小長
- **min_value** - `#[field(min_value = N)]`による数値最小値
- **max_value** - `#[field(max_value = N)]`による数値最大値

バリデータはフィールドメタデータ属性に保存され、実行時にアクセスできます。

例:
```rust
#[derive(Model, Serialize, Deserialize, Clone, Debug)]
#[model(app_label = "test_app", table_name = "users")]
struct User {
    #[field(primary_key = true)]
    id: i64,

    #[field(max_length = 100, email = true)]
    email: String,

    #[field(max_length = 200, url = true)]
    website: String,

    #[field(max_length = 100, min_length = 3)]
    username: String,

    #[field(min_value = 0, max_value = 120)]
    age: i32,
}

// field_metadata()経由でバリデータメタデータへアクセス
let fields = User::field_metadata();
let email_field = fields.iter()
    .find(|f| f.name == "email")
    .expect("email field should exist");
assert!(email_field.attributes.contains_key("email"));
```

### ハイブリッドプロパティ(reinhardt-hybrid経由)

- **HybridProperty** - インスタンスレベルとクラスレベルの両方で機能するプロパティ
- **HybridMethod** - インスタンスレベルとクラスレベルの両方で機能するメソッド
- **HybridComparator** - ハイブリッドプロパティ用カスタム比較ロジック

## 予定

### マイグレーションシステム

マイグレーション機能は別の`reinhardt-migrations`クレートによって提供されます。以下の機能が含まれます:

- モデル変更からのマイグレーション生成
- マイグレーション依存関係解決
- フォワード・バックワードマイグレーション実行
- スキーマイントロスペクションと差分検出

### 高度な機能

- ポリモーフィックモデルとクエリ(着手済み、未完成)
- マルチデータベース設定用データベースルーティング(着手済み、未完成)
- インストルメンテーションとプロファイリング(着手済み、未完成)
- リフレクションとメタデータ検査(着手済み、未完成)
- 宣言型ベースシステム(着手済み、未完成)
- セッション管理(着手済み、未完成)
- コネクションプーリング設定
- 二相コミットサポート
- 生成フィールド(着手済み、未完成)
- ファイルフィールド(着手済み、未完成)
- GISサポート(着手済み、未完成)
- 共通テーブル式(CTE)サポート(着手済み、未完成)
- ラテラル結合(着手済み、未完成)
- ラムダステートメントサポート(着手済み、未完成)
- 絶対URL上書き(着手済み、未完成)
- 複合シノニム(着手済み、未完成)
- Order with respect to(着手済み、未完成)

### クエリ強化

- プリフェッチ関連の最適化
- ofパラメータを伴うselect for update
- 特定フィールドに対するdistinct
- クエリヒントと最適化

### 追加バリデータ

- カスタムバリデータフレームワーク
- 非同期バリデータサポート
- フィールド間バリデーション
- モデルレベルバリデーションフック

### パフォーマンス最適化

- クエリ結果キャッシング
- コネクションプーリング
- プリペアドステートメントキャッシング
- バッチ操作の最適化
