# reinhardt-validators

Rust向けのDjangoスタイルのデータバリデーションユーティリティ。

## 概要

Djangoのバリデーターパターンに従った、再利用可能なバリデーターの包括的なコレクションです。メールアドレス、URL、数値範囲、文字列長、カスタム正規表現パターンなど、一般的なユースケースに対して型安全なバリデーションを提供します。

## 機能

### 実装済み ✓

#### コアバリデーションフレームワーク

- **Validatorトレイト**: カスタムバリデーターを実装するための汎用バリデーションインターフェース `Validator<T>`
- **OrmValidatorトレイト**: カスタムエラーメッセージを持つORMバリデーター用の拡張トレイト
- **SettingsValidatorトレイト**: 設定値をバリデーションするための拡張トレイト
- **ValidationError**: 説明的なメッセージを持つ包括的なエラー型
- **ValidationResult<T>**: バリデーション操作のための型安全な結果型
- **Preludeモジュール**: すべてのバリデーターとエラー型の便利な再エクスポート

#### 文字列バリデーター

- **MinLengthValidator**: 最小文字列長のバリデーション
  - `String`と`&str`型の両方で動作
  - 実際の長さと期待される長さを含む詳細なエラーメッセージを提供
  - Unicode対応の長さチェック
- **MaxLengthValidator**: 最大文字列長のバリデーション
  - `String`と`&str`型の両方で動作
  - 実際の長さと期待される長さを含む詳細なエラーメッセージを提供
  - Unicode対応の長さチェック
- **RegexValidator**: 正規表現によるパターンマッチング
  - `with_message()`によるカスタムエラーメッセージのサポート
  - 完全な正規表現構文のサポート
  - `String`と`&str`型の両方で動作

#### 数値バリデーター

- **MinValueValidator**: 最小数値のバリデーション
  - `PartialOrd + Display + Clone`を実装する任意の型に対してジェネリック
  - 整数型のサポート（i8, i16, i32, i64, isize, u8, u16, u32, u64, usize）
  - 浮動小数点数のサポート（f32, f64）
  - 実際の値と期待される値を含む詳細なエラーメッセージを提供
- **MaxValueValidator**: 最大数値のバリデーション
  - `PartialOrd + Display + Clone`を実装する任意の型に対してジェネリック
  - すべての整数型と浮動小数点型をサポート
  - 実際の値と期待される値を含む詳細なエラーメッセージを提供
- **RangeValidator**: 範囲内の値のバリデーション（両端を含む）
  - `PartialOrd + Display + Clone`を実装する任意の型に対してジェネリック
  - すべての数値型をサポート
  - 値が小さすぎるか大きすぎるかを報告

#### メールバリデーター

- **EmailValidator**: RFC 5322準拠のメールバリデーション
  - 大文字小文字を区別しないバリデーション
  - ローカル部分のバリデーション（最大64文字）
    - 英数字、ドット、アンダースコア、パーセント記号、プラスとマイナス記号を許可
    - 連続するドットを防止
    - 先頭/末尾のドットを防止
  - ドメイン部分のバリデーション（最大255文字）
    - サブドメインのサポート
    - 各ラベルは最大63文字
    - TLDは最小2文字
    - ドメインラベルの先頭/末尾のハイフンを防止
  - 全体の長さ制限（最大320文字）
  - `String`と`&str`型の両方で動作

#### URLバリデーター

- **UrlValidator**: HTTP/HTTPS URLのバリデーション
  - スキームのバリデーション（http, https）
  - ポート番号のサポート（1〜5桁）
  - パスのバリデーション
  - クエリ文字列のサポート
  - フラグメント識別子のサポート
  - サブドメインのサポート
  - ドメイン名のハイフンのサポート（ラベルの先頭/末尾以外）
  - `String`と`&str`型の両方で動作

#### エラー型

- `InvalidEmail(String)`: 無効なメールアドレス形式
- `InvalidUrl(String)`: 無効なURL形式
- `TooSmall { value: String, min: String }`: 値が最小値を下回っている
- `TooLarge { value: String, max: String }`: 値が最大値を上回っている
- `TooShort { length: usize, min: usize }`: 文字列が最小長より短い
- `TooLong { length: usize, max: usize }`: 文字列が最大長より長い
- `PatternMismatch(String)`: 正規表現パターンにマッチしなかった
- `Custom(String)`: カスタムバリデーションエラー

### 予定

#### 追加バリデーター

- **SlugValidator**: URL安全なスラグのバリデーション
- **UUIDValidator**: UUID形式のバリデーション（v1-v5）
- **IPAddressValidator**: IPv4/IPv6アドレスのバリデーション
- **DateValidator**: 日付形式のバリデーション
- **TimeValidator**: 時刻形式のバリデーション
- **DateTimeValidator**: 日時形式のバリデーション
- **JSONValidator**: JSON構造とスキーマのバリデーション
- **FileExtensionValidator**: ファイル拡張子のバリデーション
- **FileSizeValidator**: ファイルサイズのバリデーション
- **ImageDimensionValidator**: 画像の幅/高さのバリデーション
- **ColorValidator**: カラーコードのバリデーション（hex、rgb、rgbaなど）
- **PhoneNumberValidator**: 電話番号のバリデーション（E.164形式）
- **CreditCardValidator**: クレジットカード番号のバリデーション（Luhnアルゴリズム）
- **PostalCodeValidator**: 郵便番号のバリデーション（国別）

#### 拡張機能

- **バリデーター合成**: AND/ORロジックで複数のバリデーターを組み合わせる
- **条件付きバリデーション**: 条件に基づいてバリデーターを適用
- **非同期バリデーター**: 非同期バリデーション操作のサポート
- **カスタムエラーメッセージ**: バリデーターごとのカスタムエラーメッセージテンプレート
- **国際化（i18n）**: 多言語エラーメッセージ
- **シリアライゼーションサポート**: 保存のためのバリデーターのシリアライズ/デシリアライズ
- **スキーマバリデーション**: JSON Schemaおよびその他のスキーマ形式のサポート

#### パフォーマンス最適化

- **遅延正規表現コンパイル**: 必要な時のみ正規表現パターンをコンパイル
- **バリデーターキャッシング**: 再利用のためにコンパイル済みバリデーターをキャッシュ
- **並列バリデーション**: 独立したバリデーターを同時実行

## 使用例

### 基本的な文字列バリデーション

```rust
use reinhardt_validators::{MinLengthValidator, MaxLengthValidator, Validator};

let min_validator = MinLengthValidator::new(5);
let max_validator = MaxLengthValidator::new(10);

assert!(min_validator.validate("hello").is_ok());
assert!(min_validator.validate("hi").is_err());

assert!(max_validator.validate("hello").is_ok());
assert!(max_validator.validate("hello world").is_err());
```

### 数値範囲バリデーション

```rust
use reinhardt_validators::{RangeValidator, Validator};

let validator = RangeValidator::new(10, 20);
assert!(validator.validate(&15).is_ok());
assert!(validator.validate(&5).is_err());
assert!(validator.validate(&25).is_err());
```

### メールバリデーション

```rust
use reinhardt_validators::{EmailValidator, Validator};

let validator = EmailValidator::new();
assert!(validator.validate("user@example.com").is_ok());
assert!(validator.validate("invalid@").is_err());
```

### URLバリデーション

```rust
use reinhardt_validators::{UrlValidator, Validator};

let validator = UrlValidator::new();
assert!(validator.validate("http://example.com").is_ok());
assert!(validator.validate("https://example.com:8080/path?query=value#section").is_ok());
assert!(validator.validate("not-a-url").is_err());
```

### 正規表現パターンバリデーション

```rust
use reinhardt_validators::{RegexValidator, Validator};

let validator = RegexValidator::new(r"^\d{3}-\d{4}$")
    .unwrap()
    .with_message("Phone number must be in format XXX-XXXX");

assert!(validator.validate("123-4567").is_ok());
assert!(validator.validate("invalid").is_err());
```

### 複数のバリデーターの組み合わせ

```rust
use reinhardt_validators::{MinLengthValidator, MaxLengthValidator, Validator};

fn validate_username(username: &str) -> Result<(), String> {
    let min_validator = MinLengthValidator::new(3);
    let max_validator = MaxLengthValidator::new(20);

    min_validator.validate(username).map_err(|e| e.to_string())?;
    max_validator.validate(username).map_err(|e| e.to_string())?;

    Ok(())
}

assert!(validate_username("john").is_ok());
assert!(validate_username("jo").is_err());
assert!(validate_username("verylongusernamethatexceedslimit").is_err());
```

### Preludeの使用

```rust
use reinhardt_validators::prelude::*;

let email = EmailValidator::new();
let url = UrlValidator::new();
let range = RangeValidator::new(0, 100);

assert!(email.validate("test@example.com").is_ok());
assert!(url.validate("http://example.com").is_ok());
assert!(range.validate(&50).is_ok());
```

## テスト

すべてのバリデーターには、Djangoのバリデーターテストに基づいた包括的なテストスイートが含まれています。テストの実行：

```bash
cargo test
```

## ライセンス

以下のいずれかのライセンスの下でライセンスされています：

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

お好みの方を選択してください。
