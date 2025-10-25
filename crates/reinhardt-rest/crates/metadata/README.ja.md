# reinhardt-metadata

RhinhardtフレームワークにおけるOPTIONSリクエスト用のAPIメタデータとスキーマ生成。

## 概要

APIエンドポイントに関する包括的なメタデータを生成します。これには、利用可能なアクション、フィールド情報、バリデーションルールが含まれます。このメタデータは、ブラウザブルAPIや自動ドキュメント生成に使用されます。Django REST Frameworkのメタデータクラスにインスパイアされています。

## 実装済み ✓

### コアメタデータシステム

- **BaseMetadata Trait**: `determine_metadata`非同期メソッドを持つすべてのメタデータプロバイダーの基本トレイト
- **SimpleMetadata**: ビューとフィールド情報を返すデフォルトのメタデータ実装
  - アクション包含の設定可能性（`include_actions`）
  - POST/PUT/PATCHメソッド用の自動アクション検出
  - リクエストベースのメタデータ生成

### フィールドタイプシステム

様々なデータタイプをサポートする包括的なフィールドタイプ：

- 基本型: `Field`, `Boolean`, `String`, `Integer`, `Float`, `Decimal`
- 日付/時刻型: `Date`, `DateTime`, `Time`, `Duration`
- 特殊型: `Email`, `Url`, `Uuid`
- 選択型: `Choice`, `MultipleChoice`
- ファイル型: `File`, `Image`
- 複合型: `List`, `NestedObject`

### フィールドメタデータ

- **FieldInfo**: 詳細なフィールドメタデータ：
  - フィールドタイプと必須ステータス
  - 読み取り専用フラグ
  - 人間が読めるラベルとヘルプテキスト
  - バリデーション制約（最小/最大長、最小/最大値）
  - 選択フィールドの選択肢オプション
  - リスト型用の子フィールド
  - ネストされたオブジェクト用の子フィールド群

### ビルダーパターン

- **FieldInfoBuilder**: フィールドメタデータを構築するための流暢なAPI：
  - 型安全なフィールド設定
  - オプションの制約設定
  - 選択肢の設定
  - ネストされた構造のサポート

### メタデータレスポンス

- **MetadataResponse**: 完全なメタデータレスポンス構造
  - ビュー名と説明
  - サポートされるレンダー形式（例：`application/json`）
  - サポートされるパース形式
  - フィールド情報を持つ利用可能なアクション

### 設定

- **MetadataOptions**: メタデータ生成用の設定可能なオプション
  - ビュー名と説明
  - 許可されたHTTPメソッド
  - レンダー形式とパース形式
  - デフォルト設定のサポート

### エラーハンドリング

- **MetadataError**: 専用のエラータイプ
  - `DeterminationError`: メタデータ決定の失敗
  - `SerializerNotAvailable`: シリアライザー不在エラー

## 予定

### OpenAPI統合

- フィールドメタデータからのOpenAPI 3.0スキーマ生成
- Rust型からの自動スキーマ推論
- スキーマバリデーションとドキュメント生成

### 高度なメタデータプロバイダー

- シリアライザー対応のメタデータ生成
- モデルベースのメタデータイントロスペクション
- カスタムメタデータクラスのサポート

### 拡張フィールド機能

- 正規表現バリデーションパターン
- カスタムフィールドバリデーター
- フィールド依存関係と条件付き要件
- デフォルト値の指定

## 使用例

```rust
use reinhardt_metadata::{
    BaseMetadata, SimpleMetadata, MetadataOptions,
    FieldInfoBuilder, FieldType, ChoiceInfo
};
use std::collections::HashMap;

// メタデータプロバイダーを作成
let metadata = SimpleMetadata::new();

// メタデータオプションを設定
let options = MetadataOptions {
    name: "User List".to_string(),
    description: "List all users".to_string(),
    allowed_methods: vec!["GET".to_string(), "POST".to_string()],
    renders: vec!["application/json".to_string()],
    parses: vec!["application/json".to_string()],
};

// フィールドメタデータを構築
let mut fields = HashMap::new();
fields.insert(
    "username".to_string(),
    FieldInfoBuilder::new(FieldType::String)
        .required(true)
        .label("Username")
        .min_length(3)
        .max_length(50)
        .build()
);

fields.insert(
    "status".to_string(),
    FieldInfoBuilder::new(FieldType::Choice)
        .required(true)
        .choices(vec![
            ChoiceInfo {
                value: "active".to_string(),
                display_name: "Active".to_string(),
            },
            ChoiceInfo {
                value: "inactive".to_string(),
                display_name: "Inactive".to_string(),
            },
        ])
        .build()
);

// アクションメタデータを生成
let actions = metadata.determine_actions(&options.allowed_methods, &fields);
```

## 依存関係

- `reinhardt-apps`: コアアプリケーションタイプとリクエストハンドリング
- `reinhardt-serializers`: シリアライゼーションサポート
- `async-trait`: 非同期トレイトサポート
- `serde`: シリアライゼーションフレームワーク
- `thiserror`: エラーハンドリング
