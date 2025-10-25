# reinhardt-serializers

Django REST Frameworkに触発された、Rust用の型安全なデータシリアライゼーションとバリデーション。

## 概要

Rustの型と様々な形式(JSON、XMLなど)間の変換を行うシリアライザーを提供し、組み込みのバリデーション機能をサポートします。自動的なモデルシリアライゼーション、データベース制約用のバリデーター、そしてORMとのシームレスな統合による型安全なデータ変換を含みます。

## 機能

### 実装済み ✓

#### コアシリアライゼーション

- **`Serializer`トレイト**: データのシリアライゼーションとデシリアライゼーション用の汎用トレイト
  - `serialize()`: Rustの型を出力形式に変換
  - `deserialize()`: 出力形式をRustの型にパース
  - `SerializerError`: シリアライゼーション失敗時の型安全なエラーハンドリング

- **`JsonSerializer<T>`**: JSON シリアライゼーション実装
  - 効率的なJSON処理のために`serde_json`を使用
  - `Serialize`と`Deserialize`を実装した任意の型をサポート
  - Rustの型とJSON文字列間の自動変換

- **`Deserializer`トレイト**: デシリアライゼーション専用インターフェース
  - 読み取り専用のデシリアライゼーション操作用の独立したトレイト
  - より柔軟なデータパース処理パイプラインを実現

#### モデルシリアライゼーション

- **`ModelSerializer<M>`**: ORMモデルの自動シリアライゼーション
  - Djangoスタイルのモデル定義からの自動フィールドマッピング
  - `validate()`メソッドによる組み込みバリデーションサポート
  - `reinhardt-orm::Model`トレイトとのシームレスな統合
  - データベースモデルのJSON シリアライゼーション/デシリアライゼーション
  - カスタムビジネスロジック用の拡張可能なバリデーションシステム
  - **Meta設定**: フィールドの包含/除外、読み取り専用/書き込み専用フィールド
  - **フィールドイントロスペクション**: モデルフィールドと型の自動検出
  - **ネストされたシリアライザーサポート**: 関連オブジェクトの設定とシリアライゼーション
  - **バリデーター統合**: 組み込みのデータベース制約バリデーション

#### Meta設定

- **`MetaConfig`**: Django REST FrameworkスタイルのMetaオプション
  - `fields`: 特定のフィールドを明示的に含める
  - `exclude`: 特定のフィールドを除外
  - `read_only_fields`: フィールドを読み取り専用としてマーク
  - `write_only_fields`: フィールドを書き込み専用としてマーク(例: パスワード)
  - チェーン可能なメソッドによるビルダーパターン
  - 包括的なドキュメントテスト(4テスト)とユニットテスト(8テスト)

#### フィールドイントロスペクション

- **`FieldIntrospector`**: 自動フィールド検出と型推論
  - `FieldInfo`(名前、型、オプショナル、コレクション、主キー)でフィールドを登録
  - フィールドのクエリ: `field_names()`, `required_fields()`, `optional_fields()`, `primary_key_field()`
  - 一般的なRust型用の`TypeMapper`による型マッピング
  - ModelSerializerとの統合による自動フィールド検出
  - 包括的なユニットテスト(14テスト)と統合テスト(10テスト)

- **`FieldInfo`**: 豊富なフィールドメタデータ
  - フィールド名、型名、オプショナル性、コレクションステータス
  - 主キーの識別
  - 設定用のビルダーパターン

#### ネストされたシリアライゼーション

- **`NestedSerializerConfig`**: ネストされたオブジェクトのシリアライゼーション設定
  - フィールドごとの深さ制御
  - 読み取り専用 vs 書き込み可能なネストされたフィールド
  - 作成/更新権限(`allow_create`, `allow_update`)
  - 柔軟なネストされたフィールド設定
  - 包括的なユニットテスト(11テスト)

- **`NestedFieldConfig`**: 個別のネストされたフィールド設定
  - `depth()`: ネスト深さの設定(デフォルト: 1)
  - `read_only()`: 読み取り専用としてマーク
  - `writable()`: 作成/更新操作を有効化
  - `allow_create()`, `allow_update()`: きめ細かい権限設定

- **`SerializationContext`**: 循環参照と深さの管理
  - 訪問したオブジェクトを追跡して無限ループを防止
  - 最大深さの強制
  - コンテキストを考慮したトラバーサルメソッド
  - 包括的なユニットテスト(15テスト)と統合テスト(17テスト)

- **`RecursiveError`**: ネストされたシリアライゼーションのエラーハンドリング
  - `MaxDepthExceeded`: ネストが深すぎる
  - `CircularReference`: 循環依存を検出
  - `SerializationError`: 一般的なシリアライゼーション失敗

#### バリデーター設定

- **`ValidatorConfig<M>`**: ModelSerializerのバリデーター管理
  - `UniqueValidator`と`UniqueTogetherValidator`を登録
  - 登録されたバリデーターのクエリ
  - 型安全なバリデーター管理
  - 包括的なユニットテスト(4テスト)と統合テスト(17テスト)

#### データベースバリデーター

- **`UniqueValidator<M>`**: データベースのフィールド一意性を強制
  - PostgreSQLデータベースに対する非同期バリデーション
  - 更新操作をサポート(現在のインスタンスを一意性チェックから除外)
  - カスタマイズ可能なフィールド名とエラーメッセージ
  - データベースレベルの一意性検証
  - カスタムエラーメッセージ用の`with_message()`によるビルダーパターン
  - クローン可能でデバッグ可能
  - 包括的なユニットテスト(4テスト)

- **`UniqueTogetherValidator<M>`**: フィールドの一意な組み合わせを保証
  - 複数フィールドの一意性制約
  - 非同期PostgreSQLバリデーション
  - 更新操作のサポート
  - `with_message()`によるカスタマイズ可能なエラーメッセージ
  - 柔軟なフィールドの組み合わせ
  - クローン可能でデバッグ可能
  - 包括的なユニットテスト(4テスト)

#### エラーハンドリング

- **`SerializerError`**: すべてのシリアライゼーション操作用の包括的なエラー型
  - `Validation(ValidatorError)`: 詳細なコンテキストを持つバリデーションエラー
  - `Serde { message }`: シリアライゼーション/デシリアライゼーションエラー
  - `Other { message }`: 一般的なエラー
  - ヘルパーコンストラクター: `unique_violation()`, `unique_together_violation()`, `required_field()`, `database_error()`
  - エラー検査用の`is_validation_error()`, `as_validator_error()`
  - 包括的なエラーハンドリングテスト(22テスト)

- **`ValidatorError`**: 詳細なバリデーションエラー情報
  - `UniqueViolation`: 単一フィールドの一意性違反
  - `UniqueTogetherViolation`: 複数フィールドの一意性違反
  - `RequiredField`: 必須フィールドの欠落
  - `FieldValidation`: フィールド制約違反(正規表現、範囲など)
  - `DatabaseError`: データベース操作エラー
  - `Custom`: 一般的なバリデーションエラー
  - フィールド名、値、制約を含む豊富なエラーコンテキスト
  - メソッド: `message()`, `field_names()`, `is_database_error()`, `is_uniqueness_violation()`

#### コンテンツネゴシエーション(再エクスポート)

- **`ContentNegotiator`**: クライアントリクエストに基づいて適切なレスポンス形式を選択
- **`MediaType`**: メディアタイプ文字列のパースと比較

#### レンダラー(`reinhardt-renderers`から再エクスポート)

- **`JSONRenderer`**: データをJSONとしてレンダリング
- **`XMLRenderer`**: データをXMLとしてレンダリング
- **`BrowsableAPIRenderer`**: API探索用のインタラクティブなHTMLインターフェース

#### パーサー(`reinhardt-parsers`から再エクスポート)

- **`JSONParser`**: JSONリクエストボディのパース
- **`FormParser`**: フォームエンコードされたデータのパース
- **`MultiPartParser`**: multipart/form-dataの処理(ファイルアップロード)
- **`FileUploadParser``: 直接的なファイルアップロード処理
- **`ParseError`**: パース失敗用のエラー型

#### フィールドタイプ

- **`FieldError`**: フィールドバリデーション失敗用の包括的なエラー型
  - すべてのバリデーションシナリオをカバーする14のエラーバリアント
  - ユーザーフレンドリーなエラーメッセージのDisplayイミュレーション
- **`CharField`**: 長さバリデーション付き文字列フィールド
  - `min_length()`, `max_length()`, `required()`, `allow_blank()`によるビルダーパターン
  - デフォルト値のサポート
  - 包括的なドキュメントテスト(7テスト)とユニットテスト(3テスト)
- **`IntegerField`**: 範囲バリデーション付き整数フィールド
  - `min_value()`, `max_value()`, `required()`, `allow_null()`によるビルダーパターン
  - i64値のサポート
  - 包括的なドキュメントテスト(6テスト)とユニットテスト(3テスト)
- **`FloatField`**: 範囲バリデーション付き浮動小数点フィールド
  - `min_value()`, `max_value()`, `required()`, `allow_null()`によるビルダーパターン
  - f64値のサポート
  - 包括的なドキュメントテスト(6テスト)とユニットテスト(1テスト)
- **`BooleanField`**: ブール値フィールド処理
  - `required()`, `allow_null()`, `default()`によるビルダーパターン
  - 常に有効なバリデーション(ブール値は無効になり得ない)
  - 包括的なドキュメントテスト(3テスト)とユニットテスト(1テスト)
- **`EmailField`**: メールフォーマットバリデーション
  - `required()`, `allow_blank()`, `allow_null()`によるビルダーパターン
  - 基本的なRFC準拠のメールバリデーション(@記号、ドット付きドメイン)
  - 包括的なドキュメントテスト(4テスト)とユニットテスト(2テスト)
- **`URLField`**: URLフォーマットバリデーション
  - `required()`, `allow_blank()`, `allow_null()`によるビルダーパターン
  - HTTP/HTTPSプロトコルバリデーション
  - 包括的なドキュメントテスト(4テスト)とユニットテスト(2テスト)
- **`ChoiceField`**: 列挙値バリデーション
  - `required()`, `allow_blank()`, `allow_null()`によるビルダーパターン
  - 設定可能な有効な選択肢のリスト
  - 包括的なドキュメントテスト(3テスト)とユニットテスト(2テスト)

#### 高度なシリアライゼーション

- **`SerializerMethodField`**: カスタム読み取り専用フィールドの計算
  - シリアライザー用のメソッドベースの計算フィールド
  - `.method_name()`によるカスタムメソッド名
  - メソッド値用のHashMapベースのコンテキスト
  - 読み取り専用フィールドのサポート(常に`read_only: true`)
  - 例: `first_name` + `last_name`から計算される`full_name`フィールド
  - 包括的なドキュメントテスト(2テスト)とユニットテスト(7テスト)

- **`MethodFieldProvider`トレイト**: メソッドフィールド付きシリアライザーのサポート
  - `compute_method_fields()`: すべてのメソッドフィールド値を生成
  - `compute_method()`: 単一のメソッドフィールド値を生成
  - シリアライザーコンテキストとの統合

- **`MethodFieldRegistry`**: 複数のメソッドフィールドの管理
  - `.register()`でメソッドフィールドを登録
  - `.get()`と`.contains()`でフィールドを取得
  - `.all()`ですべてのフィールドにアクセス

#### バリデーションシステム

- **`ValidationError`**: 構造化されたバリデーションエラーメッセージ
  - `FieldError`: フィールド名とメッセージを持つ単一フィールドバリデーションエラー
  - `MultipleErrors`: 複数のバリデーションエラーのコレクション
  - `ObjectError`: オブジェクトレベルのバリデーションエラー
  - ヘルパーメソッド: `field_error()`, `object_error()`, `multiple()`
  - エラーハンドリングのためのthiserror統合

- **`FieldValidator`トレイト**: フィールドレベルのバリデーション
  - `validate()`: 個々のフィールド値のバリデーション
  - カスタムバリデーター(EmailValidator、AgeValidatorなど)によって実装
  - JSON Valueベースのバリデーション

- **`ObjectValidator`トレイト**: オブジェクトレベルのバリデーション
  - `validate()`: 複数フィールドを持つオブジェクト全体のバリデーション
  - クロスフィールドバリデーションのサポート
  - 例: パスワード確認の一致

- **`FieldLevelValidation`トレイト**: シリアライザーのフィールドレベルバリデーション
  - `validate_field()`: 名前で特定のフィールドをバリデーション
  - `get_field_validators()`: 登録されたすべてのフィールドバリデーターを取得
  - Djangoスタイルの`validate_<field>()`パターンのサポート

- **`ObjectLevelValidation`トレイト**: シリアライザーのオブジェクトレベルバリデーション
  - `validate()`: シリアライズされたオブジェクト全体のバリデーション
  - すべてのフィールドバリデーションが通過した後に呼び出される

- **`validate_fields()`ヘルパー**: データオブジェクト内のすべてのフィールドをバリデーション
  - フィールドバリデーターのHashMapを受け取る
  - 単一のエラーまたはMultipleErrorsを返す
  - 包括的なドキュメントテスト(3テスト)とユニットテスト(13テスト)

### 高度なリレーション

#### ハイパーリンクモデルシリアライザー

- **HyperlinkedModelSerializer<M>**: Django REST Frameworkスタイルのハイパーリンクシリアライゼーション
- **UrlReverserトレイト**: リソースの自動URL生成
- **ビュー名マッピング**: ビュー名に基づいたURL生成
- **カスタムURLフィールド**: 設定可能なURLフィールド名

```rust
use reinhardt_serializers::HyperlinkedModelSerializer;

let serializer = HyperlinkedModelSerializer::<User>::new("user-detail");
// 次のようなURLを生成: {"url": "/api/users/123/", "username": "alice"}
```

#### ネストされたシリアライザー

- **NestedSerializer<M, R>**: ネストされたオブジェクトのシリアライゼーションを処理
- **リレーションシップフィールド**: 関連モデルをインラインでシリアライズ
- **深さ制御**: ネスト深さの設定
- **双方向リレーション**: 親子関係のサポート

```rust
use reinhardt_serializers::NestedSerializer;

let serializer = NestedSerializer::<Post, User>::new("author", 2);
// シリアライズ結果: {"title": "Post", "author": {"id": 1, "username": "alice"}}
```

#### リレーションフィールド

- **PrimaryKeyRelatedField<T>**: 主キーを使用してリレーションを表現
- **SlugRelatedField<T>**: slugフィールドを使用してリレーションを表現
- **StringRelatedField<T>**: 関連オブジェクトの読み取り専用文字列表現
- **柔軟な表現**: APIに最適な表現を選択

```rust
use reinhardt_serializers::{PrimaryKeyRelatedField, SlugRelatedField};

// 主キーリレーション: {"author": 123}
let pk_field = PrimaryKeyRelatedField::<User>::new();

// slugリレーション: {"author": "alice-smith"}
let slug_field = SlugRelatedField::<User>::new("slug");
```

### ORM統合

#### QuerySet統合

- **`SerializerSaveMixin`トレイト**: シリアライザー用のDjangoスタイルの保存インターフェース
- **`SaveContext`**: 保存操作用のトランザクション対応コンテキスト
- **Managerセーション**: 自動ORM作成/更新操作

```rust
use reinhardt_serializers::{SerializerSaveMixin, SaveContext};
use reinhardt_orm::{Model, Manager};

// 新しいインスタンスを作成
let context = SaveContext::new();
let user = serializer.save(context).await?;

// 既存のインスタンスを更新
let context = SaveContext::with_instance(existing_user);
let updated_user = serializer.update(validated_data, existing_user).await?;
```

#### トランザクション管理

- **`TransactionHelper`**: RAIIベースのトランザクション管理
- **自動ロールバック**: エラー時のドロップベースのクリーンアップ
- **セーブポイントサポート**: ネストされたトランザクション処理

```rust
use reinhardt_serializers::TransactionHelper;

// トランザクションで操作をラップ
TransactionHelper::with_transaction(|| async {
    // ここでのすべてのデータベース操作はアトミック
    let user = manager.create(user_data).await?;
    let profile = manager.create(profile_data).await?;
    Ok((user, profile))
}).await?;

// セーブポイントを使用したネストされたトランザクション
TransactionHelper::savepoint(depth, || async {
    // 自動セーブポイントを使用したネストされた操作
    manager.update(instance).await
}).await?;
```

#### ネストされた保存コンテキスト

- **`NestedSaveContext`**: 深さを考慮したトランザクション管理
- **自動スコープ選択**: 深さに基づいてトランザクション vs セーブポイント
- **階層的操作**: 深くネストされたシリアライザーのサポート

```rust
use reinhardt_serializers::NestedSaveContext;

let context = NestedSaveContext::new(depth);

// 深さに応じて自動的にトランザクション(depth=0)またはセーブポイント(depth>0)を使用
context.with_scope(|| async {
    // ネストされたシリアライザーの保存操作
    nested_serializer.save(data).await
}).await?;
```

#### 多対多リレーションシップ管理

- **`ManyToManyManager`**: 中間テーブル操作
- **一括操作**: 効率的なバッチ挿入/削除
- **セット操作**: すべてのリレーションシップをアトミックに置換

```rust
use reinhardt_serializers::ManyToManyManager;

let m2m_manager = ManyToManyManager::<User, Tag>::new(
    "user_tags",      // 中間テーブル
    "user_id",        // ソースFK
    "tag_id"          // ターゲットFK
);

// 複数のリレーションシップを追加
m2m_manager.add_bulk(&user_id, vec![tag1_id, tag2_id, tag3_id]).await?;

// 特定のリレーションシップを削除
m2m_manager.remove_bulk(&user_id, vec![tag1_id]).await?;

// すべてのリレーションシップをアトミックに置換
m2m_manager.set(&user_id, vec![tag4_id, tag5_id]).await?;

// すべてのリレーションシップをクリア
m2m_manager.clear(&user_id).await?;
```

#### リレーションフィールドのデータベースルックアップ

- **`PrimaryKeyRelatedFieldORM`**: データベースバックの主キーリレーション
- **`SlugRelatedFieldORM`**: データベースバックのslugフィールドリレーション
- **バッチクエリ最適化**: 複数ルックアップ用のIN句
- **カスタムQuerySetフィルター**: 追加のフィルタリング制約

```rust
use reinhardt_serializers::{PrimaryKeyRelatedFieldORM, SlugRelatedFieldORM};
use reinhardt_orm::{Filter, FilterOperator, FilterValue};

// データベースバリデーション付き主キーリレーション
let pk_field = PrimaryKeyRelatedFieldORM::<User>::new();

// データベース内の存在をバリデーション
pk_field.validate_exists(&user_id).await?;

// 単一インスタンスを取得
let user = pk_field.get_instance(&user_id).await?;

// バッチ取得(N+1クエリを防止)
let users = pk_field.get_instances(vec![id1, id2, id3]).await?;

// カスタムフィルター付きslugフィールドリレーション
let slug_field = SlugRelatedFieldORM::<User>::new("username")
    .with_queryset_filter(Filter::new(
        "is_active",
        FilterOperator::Eq,
        FilterValue::Bool(true)
    ));

// slugの存在をバリデーション
slug_field.validate_exists(&slug_value).await?;

// slugで取得
let user = slug_field.get_instance(&slug_value).await?;

// slugでバッチ取得
let users = slug_field.get_instances(vec!["alice", "bob", "charlie"]).await?;
```

#### パフォーマンス最適化

- **`IntrospectionCache`**: フィールドメタデータをキャッシュして繰り返しイントロスペクションを回避
- **`QueryCache`**: TTLベースのクエリ結果キャッシング
- **`BatchValidator`**: 複数のデータベースチェックを単一のクエリに結合
- **`PerformanceMetrics`**: シリアライゼーションとバリデーションのパフォーマンスを追跡

```rust
use reinhardt_serializers::{IntrospectionCache, QueryCache, BatchValidator, PerformanceMetrics};
use std::time::Duration;

// フィールドイントロスペクション結果のキャッシュ
let cache = IntrospectionCache::new();
if let Some(fields) = cache.get("User") {
    // キャッシュされたフィールドを使用
} else {
    let fields = introspect_fields();
    cache.set("User".to_string(), fields);
}

// TTL付きクエリ結果キャッシング
let query_cache = QueryCache::new(Duration::from_secs(300));
query_cache.set("user:123".to_string(), user_data);

// バッチバリデーション
let mut validator = BatchValidator::new();
validator.add_unique_check("users", "email", "alice@example.com");
validator.add_unique_check("users", "username", "alice");
let failures = validator.execute().await?;

// パフォーマンス追跡
let metrics = PerformanceMetrics::new();
metrics.record_serialization(50); // 50ms
let stats = metrics.get_stats();
println!("平均: {}ms", stats.avg_serialization_ms);
```

**注意**: ORM統合機能には`django-compat`フィーチャーフラグが有効である必要があります。このフラグがない場合、スタブ実装は適切なエラーを返します。

### 予定

#### 追加のフィールドタイプ

- `DateField`, `DateTimeField`: chronoとの統合による日付と時刻の処理

#### 高度なシリアライゼーション

- `WritableNestedSerializer`: ネストされたオブジェクトへの更新をサポート
- `ListSerializer`: オブジェクトのコレクションのシリアライズ

#### 追加のレンダラー

- `YAMLRenderer`: データをYAMLとしてレンダリング
- `CSVRenderer`: データをCSVとしてレンダリング(リストエンドポイント用)
- `OpenAPIRenderer`: OpenAPI/Swagger仕様の生成

#### Metaオプション

- フィールドの包含/除外
- 読み取り専用/書き込み専用フィールド
- カスタムフィールドマッピング
- ネストされたシリアライゼーションの深さ制御

## 使用例

### 基本的なJSONシリアライゼーション

```rust
use reinhardt_serializers::{JsonSerializer, Serializer};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: i64,
}

let serializer = JsonSerializer::<User>::new();
let user = User { name: "Alice".to_string(), age: 30 };

// JSONにシリアライズ
let json = serializer.serialize(&user).unwrap();
assert_eq!(json, r#"{"name":"Alice","age":30}"#);

// JSONからデシリアライズ
let parsed = serializer.deserialize(&json).unwrap();
assert_eq!(parsed.name, "Alice");
```

### バリデーション付きModelSerializer

```rust
use reinhardt_serializers::{ModelSerializer, Serializer};
use reinhardt_orm::Model;

// Modelトレイトを実装したUserモデルを持っていると仮定
let serializer = ModelSerializer::<User>::new();

let user = User {
    id: Some(1),
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
};

// シリアライゼーション前にバリデーション
assert!(serializer.validate(&user).is_ok());

// JSONにシリアライズ
let json = serializer.serialize(&user).unwrap();
```

### 一意フィールドバリデーション

```rust
use reinhardt_serializers::UniqueValidator;
use sqlx::PgPool;

let pool: PgPool = /* データベース接続 */;
let validator = UniqueValidator::<User>::new("email");

// メールが一意であることをバリデーション(新規ユーザー用)
validator.validate(&pool, "alice@example.com", None).await?;

// 更新用にバリデーション(現在のユーザーのIDを除外)
validator.validate(&pool, "alice@example.com", Some(&user_id)).await?;
```

### 一意組み合わせバリデーション

```rust
use reinhardt_serializers::UniqueTogetherValidator;
use std::collections::HashMap;

let validator = UniqueTogetherValidator::<User>::new(vec!["first_name", "last_name"]);

let mut values = HashMap::new();
values.insert("first_name".to_string(), "Alice".to_string());
values.insert("last_name".to_string(), "Smith".to_string());

validator.validate(&pool, &values, None).await?;
```

### 計算フィールド用のSerializerMethodField

```rust
use reinhardt_serializers::{SerializerMethodField, MethodFieldProvider, MethodFieldRegistry};
use serde_json::{json, Value};
use std::collections::HashMap;

struct UserSerializer {
    method_fields: MethodFieldRegistry,
}

impl UserSerializer {
    fn new() -> Self {
        let mut method_fields = MethodFieldRegistry::new();
        method_fields.register("full_name", SerializerMethodField::new("full_name"));
        Self { method_fields }
    }
}

impl MethodFieldProvider for UserSerializer {
    fn compute_method_fields(&self, instance: &Value) -> HashMap<String, Value> {
        let mut context = HashMap::new();

        if let Some(obj) = instance.as_object() {
            if let (Some(first), Some(last)) = (
                obj.get("first_name").and_then(|v| v.as_str()),
                obj.get("last_name").and_then(|v| v.as_str()),
            ) {
                let full_name = format!("{} {}", first, last);
                context.insert("full_name".to_string(), json!(full_name));
            }
        }

        context
    }

    fn compute_method(&self, method_name: &str, instance: &Value) -> Option<Value> {
        let context = self.compute_method_fields(instance);
        context.get(method_name).cloned()
    }
}

// 使用方法
let serializer = UserSerializer::new();
let user_data = json!({
    "first_name": "Alice",
    "last_name": "Johnson"
});

let context = serializer.compute_method_fields(&user_data);
assert_eq!(context.get("full_name").unwrap(), &json!("Alice Johnson"));
```

### フィールドレベルバリデーション

```rust
use reinhardt_serializers::{FieldValidator, ValidationResult, ValidationError, validate_fields};
use serde_json::{json, Value};
use std::collections::HashMap;

struct EmailValidator;

impl FieldValidator for EmailValidator {
    fn validate(&self, value: &Value) -> ValidationResult {
        if let Some(email) = value.as_str() {
            if email.contains('@') && email.contains('.') {
                Ok(())
            } else {
                Err(ValidationError::field_error("email", "Invalid email format"))
            }
        } else {
            Err(ValidationError::field_error("email", "Must be a string"))
        }
    }
}

// バリデーターを登録
let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
validators.insert("email".to_string(), Box::new(EmailValidator));

// データをバリデーション
let mut data = HashMap::new();
data.insert("email".to_string(), json!("user@example.com"));

let result = validate_fields(&data, &validators);
assert!(result.is_ok());
```

### オブジェクトレベルバリデーション

```rust
use reinhardt_serializers::{ObjectValidator, ValidationResult, ValidationError};
use serde_json::{json, Value};
use std::collections::HashMap;

struct PasswordMatchValidator;

impl ObjectValidator for PasswordMatchValidator {
    fn validate(&self, data: &HashMap<String, Value>) -> ValidationResult {
        let password = data.get("password").and_then(|v| v.as_str());
        let confirm = data.get("password_confirm").and_then(|v| v.as_str());

        if password == confirm {
            Ok(())
        } else {
            Err(ValidationError::object_error("Passwords do not match"))
        }
    }
}

// バリデーション
let validator = PasswordMatchValidator;
let mut data = HashMap::new();
data.insert("password".to_string(), json!("secret123"));
data.insert("password_confirm".to_string(), json!("secret123"));

assert!(validator.validate(&data).is_ok());
```

### コンテンツネゴシエーション

```rust
use reinhardt_serializers::{ContentNegotiator, JSONRenderer, XMLRenderer};

let negotiator = ContentNegotiator::new();
negotiator.register(Box::new(JSONRenderer::new()));
negotiator.register(Box::new(XMLRenderer::new()));

// Acceptヘッダーに基づいてレンダラーを選択
let renderer = negotiator.select("application/json")?;
```

## 依存関係

- `reinhardt-orm`: ModelSerializerのORM統合
- `reinhardt-parsers`: リクエストボディのパース
- `reinhardt-renderers`: レスポンスのレンダリング
- `reinhardt-negotiation`: コンテンツタイプのネゴシエーション
- `serde`, `serde_json`: シリアライゼーション基盤
- `sqlx`: バリデーター用のデータベース操作
- `chrono`: 日付と時刻の処理
- `thiserror`: バリデーションとメソッドフィールド用のエラー型定義
- `async-trait`: 非同期トレイトのサポート

## ライセンス

以下のいずれかのライセンスの下でライセンスされています:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

お好みに応じて選択できます。
