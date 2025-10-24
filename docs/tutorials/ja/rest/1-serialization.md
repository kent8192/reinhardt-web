# チュートリアル 1: シリアライゼーション

## はじめに

このチュートリアルでは、シンプルなコードスニペットWeb APIの作成について説明します。Reinhardtのシリアライゼーションシステムがどのように機能するかを理解していただきます。

簡単な概要だけが必要な場合は、[クイックスタート](quickstart.md)ドキュメントを参照してください。

## プロジェクトのセットアップ

新しいプロジェクトを作成しましょう。

```bash
cargo new tutorial
cd tutorial
```

`Cargo.toml`にReinhardtの依存関係を追加します:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["standard"] }
# または、最小限の場合: reinhardt = { version = "0.1.0", default-features = false, features = ["minimal", "api"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
```

> **機能フラグについて:**
>
> - `standard`: ORM、シリアライザ、ViewSet、認証、ページネーションを含む標準的な機能セット
> - `minimal`: 基本的なルーティングとパラメータ抽出のみ（マイクロサービス向け）
> - `api`: REST API機能を含む
>
> 詳細は[Feature Flags Guide](../../../FEATURE_FLAGS.md)を参照してください。

## モデルの作成

コードスニペットを保存するための`Snippet`構造体を作成します。`src/main.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub created: DateTime<Utc>,
    pub title: String,
    pub code: String,
    pub linenos: bool,
    pub language: String,
    pub style: String,
}

impl Snippet {
    pub fn new(code: String) -> Self {
        Self {
            id: None,
            created: Utc::now(),
            title: String::new(),
            code,
            linenos: false,
            language: "python".to_string(),
            style: "friendly".to_string(),
        }
    }
}
```

## 基本的なシリアライゼーション

Reinhardtでは、`serde`を使用してデータをシリアライズ/デシリアライズします:

```rust
fn main() {
    // スニペットを作成
    let snippet = Snippet::new("print('hello, world')".to_string());

    // JSONにシリアライズ
    let json = serde_json::to_string_pretty(&snippet).unwrap();
    println!("Serialized:\n{}", json);

    // JSONからデシリアライズ
    let deserialized: Snippet = serde_json::from_str(&json).unwrap();
    println!("\nDeserialized: {:?}", deserialized);
}
```

出力:

```json
{
  "id": null,
  "created": "2025-10-08T10:30:00Z",
  "title": "",
  "code": "print('hello, world')",
  "linenos": false,
  "language": "python",
  "style": "friendly"
}
```

## シリアライザの作成

データ転送用の専用シリアライザ構造体を定義できます:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnippetSerializer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default)]
    pub title: String,
    pub code: String,
    #[serde(default)]
    pub linenos: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_style")]
    pub style: String,
}

fn default_language() -> String {
    "python".to_string()
}

fn default_style() -> String {
    "friendly".to_string()
}

impl SnippetSerializer {
    pub fn from_snippet(snippet: &Snippet) -> Self {
        Self {
            id: snippet.id,
            title: snippet.title.clone(),
            code: snippet.code.clone(),
            linenos: snippet.linenos,
            language: snippet.language.clone(),
            style: snippet.style.clone(),
        }
    }

    pub fn to_snippet(&self) -> Snippet {
        Snippet {
            id: self.id,
            created: Utc::now(),
            title: self.title.clone(),
            code: self.code.clone(),
            linenos: self.linenos,
            language: self.language.clone(),
            style: self.style.clone(),
        }
    }
}
```

## シリアライゼーションとバリデーション

`Serializer`トレイトを実装することで、以下の3つの機能を統合できます:

- **シリアライゼーション**: データをバイト列に変換（`serialize`メソッド）
- **デシリアライゼーション**: バイト列からデータに変換（`deserialize`メソッド）
- **バリデーション**: データの妥当性検証（`validate`メソッド）

```rust
use reinhardt_serializers::{Serializer, ValidationError, ValidationResult};

impl Serializer<Snippet> for SnippetSerializer {
    fn serialize(&self, instance: &Snippet) -> Result<Vec<u8>, String> {
        serde_json::to_vec(instance)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    fn deserialize(&self, data: &[u8]) -> Result<Snippet, String> {
        serde_json::from_slice(data)
            .map_err(|e| format!("Deserialization error: {}", e))
    }

    fn validate(&self, instance: &Snippet) -> ValidationResult {
        let mut errors = Vec::new();

        // codeフィールドは必須
        if instance.code.is_empty() {
            errors.push(ValidationError::new("code", "This field is required"));
        }

        // titleは100文字以下
        if instance.title.len() > 100 {
            errors.push(ValidationError::new("title", "Title is too long (max 100 characters)"));
        }

        // 言語は有効な選択肢から選択
        let valid_languages = vec!["python", "rust", "javascript"];
        if !valid_languages.contains(&instance.language.as_str()) {
            errors.push(ValidationError::new("language", "Invalid language choice"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

## バリデーションの使用

```rust
fn main() {
    let serializer = SnippetSerializer {
        id: None,
        title: "Test".to_string(),
        code: "print('test')".to_string(),
        linenos: false,
        language: "python".to_string(),
        style: "friendly".to_string(),
    };

    let snippet = serializer.to_snippet();

    // バリデーション実行
    match Serializer::validate(&serializer, &snippet) {
        Ok(_) => println!("✓ Validation passed"),
        Err(errors) => {
            println!("✗ Validation errors:");
            for error in errors {
                println!("  - {}: {}", error.field, error.message);
            }
        }
    }
}
```

## 複数のスニペットをシリアライズ

リストをシリアライズする例:

```rust
fn main() {
    let snippets = vec![
        Snippet::new("print('hello')".to_string()),
        Snippet::new("fn main() {}".to_string()),
        Snippet::new("console.log('hi')".to_string()),
    ];

    // JSONにシリアライズ
    let json = serde_json::to_string_pretty(&snippets).unwrap();
    println!("{}", json);

    // シリアライザを使用
    let serializers: Vec<SnippetSerializer> = snippets
        .iter()
        .map(|s| SnippetSerializer::from_snippet(s))
        .collect();

    let serialized_data = serde_json::to_string_pretty(&serializers).unwrap();
    println!("{}", serialized_data);
}
```

## フィールドレベルのバリデーション

個別のフィールドに対するバリデーション:

```rust
use reinhardt_serializers::fields::CharField;

fn validate_title(title: &str) -> Result<(), String> {
    let field = CharField::new()
        .min_length(1)
        .max_length(100)
        .required(true);

    field.validate(&title.to_string())
        .map_err(|e| e.message)
}

fn main() {
    let title = "My Snippet";
    match validate_title(title) {
        Ok(_) => println!("✓ Title is valid"),
        Err(e) => println!("✗ Title validation error: {}", e),
    }

    let too_long = "x".repeat(150);
    match validate_title(&too_long) {
        Ok(_) => println!("✓ Title is valid"),
        Err(e) => println!("✗ Title validation error: {}", e),
    }
}
```

## 完全な例

```rust
use chrono::{DateTime, Utc};
use reinhardt_serializers::{Serializer, ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub created: DateTime<Utc>,
    pub title: String,
    pub code: String,
    pub linenos: bool,
    pub language: String,
    pub style: String,
}

impl Snippet {
    pub fn new(code: String) -> Self {
        Self {
            id: None,
            created: Utc::now(),
            title: String::new(),
            code,
            linenos: false,
            language: "python".to_string(),
            style: "friendly".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnippetSerializer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub title: String,
    pub code: String,
    pub linenos: bool,
    pub language: String,
    pub style: String,
}

impl SnippetSerializer {
    pub fn from_snippet(snippet: &Snippet) -> Self {
        Self {
            id: snippet.id,
            title: snippet.title.clone(),
            code: snippet.code.clone(),
            linenos: snippet.linenos,
            language: snippet.language.clone(),
            style: snippet.style.clone(),
        }
    }

    pub fn to_snippet(&self) -> Snippet {
        Snippet {
            id: self.id,
            created: Utc::now(),
            title: self.title.clone(),
            code: self.code.clone(),
            linenos: self.linenos,
            language: self.language.clone(),
            style: self.style.clone(),
        }
    }
}

impl Serializer<Snippet> for SnippetSerializer {
    fn serialize(&self, instance: &Snippet) -> Result<Vec<u8>, String> {
        serde_json::to_vec(instance).map_err(|e| format!("Serialization error: {}", e))
    }

    fn deserialize(&self, data: &[u8]) -> Result<Snippet, String> {
        serde_json::from_slice(data).map_err(|e| format!("Deserialization error: {}", e))
    }

    fn validate(&self, instance: &Snippet) -> ValidationResult {
        let mut errors = Vec::new();

        if instance.code.is_empty() {
            errors.push(ValidationError::new("code", "This field is required"));
        }

        if instance.title.len() > 100 {
            errors.push(ValidationError::new(
                "title",
                "Title is too long (max 100 characters)",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn main() {
    // スニペットを作成
    let snippet = Snippet::new("print('hello, world')".to_string());

    // JSONにシリアライズ
    let json = serde_json::to_string_pretty(&snippet).unwrap();
    println!("Serialized:\n{}\n", json);

    // シリアライザを使用したバリデーション
    let serializer = SnippetSerializer::from_snippet(&snippet);
    match Serializer::validate(&serializer, &snippet) {
        Ok(_) => println!("✓ Validation passed\n"),
        Err(errors) => {
            println!("✗ Validation errors:");
            for error in errors {
                println!("  - {}: {}", error.field, error.message);
            }
        }
    }
}
```

## まとめ

このチュートリアルで学んだこと:

1. Rustの構造体を使用したデータモデルの定義
2. `serde`を使用したシリアライゼーションとデシリアライゼーション
3. `Serializer`トレイトを使用したカスタムバリデーション
4. フィールドレベルのバリデーション
5. シリアライザ構造体を使用したデータ変換

次のチュートリアル: [チュートリアル 2: リクエストとレスポンス](2-requests-and-responses.md)
