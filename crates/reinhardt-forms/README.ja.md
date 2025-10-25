# reinhardt-forms

Rust向けのDjangoにインスパイアされたフォームハンドリングとバリデーション

## 概要

`reinhardt-forms`は、HTMLフォームのハンドリング、バリデーション、レンダリングのための包括的なフォームシステムを提供します。Djangoのフォームフレームワークにインスパイアされており、モデルからの自動フォーム生成と、広範なバリデーション機能を備えた手動フォーム定義の両方を提供します。

## 機能ステータス

### コアフォームシステム

#### 実装済み ✓

- **フォームベース (`Form`)**: バインディング、バリデーション、レンダリングを備えた完全なフォームデータ構造
  - 初期データとフィールドプレフィックスサポート付きのフォーム作成
  - データバインディングとバリデーションライフサイクル
  - フォームレベルおよびフィールドレベルのバリデーション用カスタムクリーン関数
  - 複数のレンダリング形式: `as_table()`, `as_p()`, `as_ul()`
  - フィールドアクセスと操作 (追加、削除、取得)
  - 初期データと変更検出
  - エラーハンドリングと報告

- **BoundField**: レンダリングのためにフォームデータにバインドされたフィールド
  - フィールドデータとエラーバインディング
  - 適切なエスケープ付きHTML レンダリング
  - ウィジェット統合
  - ラベルとヘルプテキストのサポート

- **CSRF保護 (`CsrfToken`)**: 基本的なCSRFトークン実装
  - トークンの生成と保存
  - 隠し入力のレンダリング
  - `enable_csrf()`によるフォーム統合

- **メディア管理 (`Media`)**: CSSとJavaScriptアセット管理
  - メディア定義構造
  - ウィジェットメディア統合 (`MediaDefiningWidget`トレイト経由)

### フィールドタイプ

#### 実装済み ✓

**基本フィールド:**

- `CharField`: 最小/最大長、空白削除、ヌル文字バリデーション付きテキスト入力
- `IntegerField`: 最小/最大値制約、文字列パース付き整数入力
- `BooleanField`: 柔軟な型強制を持つブール値/チェックボックス入力
- `EmailField`: 正規表現と長さ制約によるメールバリデーション

**高度なフィールド:**

- `FloatField`: 最小/最大制約付き浮動小数点数バリデーション
- `DecimalField`: スケールと精度を持つ正確な10進数処理
- `DateField`: 複数フォーマットサポートとロケール処理付き日付入力
- `TimeField`: フォーマットパース付き時刻入力
- `DateTimeField`: 日付と時刻の組み合わせバリデーション
- `URLField`: スキームと最大長チェック付きURLバリデーション
- `JSONField`: JSONデータのバリデーションとパース
- `FileField`: サイズバリデーション付きファイルアップロード処理
- `ImageField`: 寸法チェック付き画像ファイルバリデーション
- `ChoiceField`: 事前定義された選択肢からの選択
- `MultipleChoiceField`: 複数選択サポート
- `RegexField`: カスタム正規表現によるパターンベースのバリデーション
- `SlugField`: URLスラッグバリデーション
- `GenericIPAddressField`: IPv4/IPv6アドレスバリデーション
- `UUIDField`: UUID形式バリデーション
- `DurationField`: 時間間隔のパース
- `ComboField`: 複数フィールドバリデーションの組み合わせ
- `MultiValueField`: 複合フィールド処理 (分割フィールドのベース)
- `SplitDateTimeField`: 個別の日付と時刻入力

**モデル関連フィールド:**

- `ModelChoiceField`: クエリセットサポート付き外部キー選択
- `ModelMultipleChoiceField`: 多対多選択

### モデル統合

#### 実装済み ✓

- **ModelForm (`ModelForm<T>`)**: モデルからの自動フォーム生成
  - モデル統合のための`FormModel`トレイト
  - モデルメタデータからのフィールド型推論
  - フィールドの包含/除外設定
  - カスタムフィールドオーバーライドサポート
  - フォームデータからのモデルインスタンスの生成
  - バリデーション付き保存機能

- **ModelFormBuilder**: ModelForm設定のための流暢なAPI
  - フィールド選択 (include/exclude)
  - ウィジェットカスタマイズ
  - ラベルカスタマイズ
  - ヘルプテキストカスタマイズ

- **ModelFormConfig**: ModelFormの動作のための設定構造
  - フィールドマッピング設定
  - バリデーションルール
  - 保存動作のカスタマイズ

### フォームセット

#### 実装済み ✓

- **FormSet**: 複数のフォームを一緒に管理
  - フォームコレクション管理
  - 複数フォーム間のバリデーション
  - 追加フォーム生成
  - 最小/最大フォーム数制約
  - 削除と順序付けサポート
  - 管理フォーム処理
  - 非フォームエラートラッキング

- **ModelFormSet**: モデルインスタンス用のフォームセット
  - クエリセット統合
  - インスタンスの作成、更新、削除
  - インラインフォームセットサポート
  - `ModelFormSetConfig`による設定
  - `ModelFormSetBuilder`によるビルダーパターンAPI

### 高度な機能

#### 実装済み ✓

- **フォームウィザード (`FormWizard`)**: 複数ステップのフォームフロー
  - ステップ定義と管理 (`WizardStep`)
  - 条件付きステップ利用可能性
  - ステップ間のセッションデータ保存
  - ステップナビゲーション (次へ、前へ、ジャンプ)
  - 最終データのコンパイル
  - 進捗トラッキング

- **ウィジェットシステム**: フォームフィールドのHTML レンダリング
  - ベース`Widget`トレイト
  - `WidgetType`列挙型
  - 組み込みウィジェット:
    - テキスト入力 (text, password, email, number)
    - 日付/時刻入力 (date, time, datetime)
    - Textarea
    - Select (単一および複数)
    - チェックボックスとラジオ入力
    - ファイル入力
    - 隠し入力
    - 分割日時
  - カスタム属性サポート
  - selectウィジェットの選択肢レンダリング

### バリデーション

#### 実装済み ✓

- **フィールドバリデーション**: 個別フィールドのクリーニングとバリデーション
  - 必須フィールドチェック
  - 型変換と型強制
  - 長さ制約 (CharField)
  - 値範囲制約 (IntegerField, FloatField, DecimalField)
  - フォーマットバリデーション (EmailField, URLField, DateField など)
  - パターンマッチング (RegexField)
  - カスタムバリデーター

- **フォームバリデーション**: 複数フィールドバリデーション
  - カスタムクリーンメソッド (`add_clean_function`)
  - フィールド固有のクリーンメソッド (`add_field_clean_function`)
  - フィールド間バリデーション
  - エラー集約
  - 非フィールドエラー

- **エラーハンドリング**: 包括的なエラー報告
  - `FieldError`タイプ (Required, Invalid, Validation)
  - `FormError`タイプ (Field, Validation)
  - カスタムエラーメッセージ
  - エラーメッセージの国際化サポート

#### 実装済み ✓

- **セキュリティ機能**:
  - レート制限統合 (`RateLimiter`)
  - ハニーポットフィールド (`HoneypotField`)
  - フォームセキュリティミドルウェア (`FormSecurityMiddleware`)

- **ファイルハンドリング**:
  - `Drop`実装による一時ファイルのクリーンアップ
  - サイズと拡張子バリデーション付きファイルアップロードハンドラー
  - メモリベースのファイルアップロード
  - 自動削除付きディスクベースの一時ファイル

- **国際化** (部分的):
  - ロケール対応の日付/時刻フォーマット (`localize`サポート付き`DateField`)
  - 数値フォーマットのローカライゼーション (`thousands_separator`付き`DecimalField`)
  - フィールドごとのロケール設定

- **フォームテンプレート** (部分的):
  - Bootstrap 5統合 (`BootstrapRenderer`)
  - Tailwind CSS統合 (`TailwindRenderer`)
  - テキスト入力、セレクト、チェックボックス用のCSSフレームワークレンダラー

### 予定機能

- **高度なCSRF保護**:
  - 暗号化トークン生成
  - トークンローテーション
  - Same-siteクッキーサポート
  - オリジンバリデーション

- **ファイルハンドリングの強化**:
  - チャンクアップロードサポート
  - 進捗トラッキング
  - ファイルバリデーションルールエンジン

- **セキュリティ機能**:
  - 高度なXSS保護
  - 入力サニタイゼーションルール

- **追加フィールドタイプ**:
  - 強度バリデーション付き`PasswordField`
  - カラーピッカー用`ColorField`
  - 数値範囲用`RangeField`
  - 配列データ用`ArrayField`
  - 空間データ用`GeometryField`

- **国際化**:
  - 多言語エラーメッセージ
  - RTL言語サポート
  - 完全なi18nメッセージカタログ

- **フォームテンプレート**:
  - テンプレートベースのレンダリングエンジン
  - カスタムフォームレイアウト
  - アクセシブルなフォームマークアップ生成 (ARIA属性)
  - 追加のCSSフレームワークサポート

- **高度なウィジェット**:
  - リッチテキストエディター
  - カレンダー付き日付ピッカー
  - オートコンプリート入力
  - 検索付きマルチセレクト
  - ファイルドラッグアンドドロップ

- **テストユーティリティ**:
  - フォームテストヘルパー
  - モックデータ生成
  - バリデーションテストフィクスチャ
  - 統合テストサポート

## 使用例

### 基本的なフォーム

```rust
use reinhardt_forms::{Form, CharField, IntegerField, FormField};
use std::collections::HashMap;
use serde_json::json;

let mut form = Form::new();
form.add_field(Box::new(CharField::new("name".to_string())));
form.add_field(Box::new(IntegerField::new("age".to_string())));

let mut data = HashMap::new();
data.insert("name".to_string(), json!("John"));
data.insert("age".to_string(), json!(30));

form.bind(data);
assert!(form.is_valid());
```

### ModelForm

```rust
use reinhardt_forms::{ModelForm, ModelFormBuilder};

let form = ModelFormBuilder::<User>::new()
    .include_fields(vec!["name", "email"])
    .build();
```

### カスタムバリデーション

```rust
use reinhardt_forms::{Form, FormError};

let mut form = Form::new();
form.add_clean_function(|data| {
    if data.get("password") != data.get("confirm_password") {
        Err(FormError::Validation("Passwords do not match".to_string()))
    } else {
        Ok(())
    }
});
```

## アーキテクチャ

- **フィールド層**: バリデーションロジックを持つ個別のフィールドタイプ
- **フォーム層**: フォーム構造、バインディング、バリデーション
- **ウィジェット層**: HTML レンダリングとブラウザインタラクション
- **モデル層**: ORM統合と自動フォーム生成
- **フォームセット層**: 複数フォーム管理
- **ウィザード層**: 複数ステップフォームフロー

## デザイン哲学

このクレートは、Djangoのフォーム哲学に従います:

- 宣言的なフィールド定義
- バリデーションロジックの分離
- 自動HTML レンダリング
- モデル統合
- 拡張可能でカスタマイズ可能

## ライセンス

Apache License, Version 2.0またはMIT licenseのいずれかの条件の下でライセンスされています。
