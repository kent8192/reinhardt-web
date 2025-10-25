# reinhardt-templates-macros

Reinhardtのコンパイル時テンプレートパス検証のための手続き型マクロ。

## 概要

このクレートは、コンパイル時にテンプレートパスを検証するマクロを提供します。これにより、実行時の前にテンプレートパスが正しい構文とセキュリティ制約に従っていることを確認できます。

## 機能

- **コンパイル時検証**: コンパイル中にテンプレートパスのエラーを検出
- **パストラバーサル保護**: `../` 親ディレクトリ参照を防止
- **クロスプラットフォーム安全性**: Unix形式のパスを強制(バックスラッシュなし)
- **拡張子検証**: 有効なファイル拡張子(.html、.txtなど)を保証
- **明確なエラーメッセージ**: 例付きの有用なコンパイル時エラーメッセージ

## 使い方

```rust
use reinhardt_templates_macros::template;

// 有効なテンプレートパス
let path = template!("emails/welcome.html");
let path = template!("admin/users/list.html");
let path = template!("base.html");
```

## 検証ルール

`template!` マクロは以下のルールを適用します：

1. **相対パスのみ**: 先頭のスラッシュ(`/`)なし
2. **親ディレクトリ参照なし**: パス内に `..` なし
3. **Unix形式のパス**: バックスラッシュ(`\`)なし
4. **有効な拡張子**: 許可されたファイル拡張子のみ
5. **二重スラッシュなし**: 連続する `/` 文字なし
6. **有効な文字**: 英数字、ハイフン、アンダースコア、ドット、スラッシュのみ

### 許可される拡張子

- `.html`, `.htm` - HTMLテンプレート
- `.txt` - テキストテンプレート
- `.xml` - XMLテンプレート
- `.json` - JSONテンプレート
- `.css`, `.js` - スタイル/スクリプトテンプレート
- `.md` - Markdownテンプレート
- `.svg` - SVGテンプレート
- `.jinja`, `.j2`, `.tpl`, `.template` - テンプレートファイル

## 例

### 有効なパス

```rust
use reinhardt_templates_macros::template;

// シンプルなパス
template!("index.html");
template!("base.html");

// ネストされたパス
template!("emails/welcome.html");
template!("admin/users/list.html");

// 異なる拡張子
template!("config.json");
template!("styles.css");
template!("template.jinja");

// ハイフンとアンダースコア付き
template!("user-profile.html");
template!("user_details.html");
```

### 無効なパス(コンパイル時エラー)

```rust
use reinhardt_templates_macros::template;

// エラー: 親ディレクトリ参照
template!("../etc/passwd");

// エラー: バックスラッシュは許可されていません
template!("path\\to\\file.html");

// エラー: 絶対パス
template!("/etc/passwd");

// エラー: 無効な拡張子
template!("file.exe");

// エラー: 二重スラッシュ
template!("templates//index.html");

// エラー: 空のパス
template!("");
```

## セキュリティ

このマクロは一般的なセキュリティ脆弱性の防止に役立ちます：

- **パストラバーサル攻撃**: `..` 参照を拒否することによる防止
- **クロスプラットフォームの問題**: Unix形式のパスを強制することによる防止
- **無効なリソース**: ファイル拡張子を検証することによる防止

## 統合

このクレートは `reinhardt-templates` と連携するように設計されており、パス検証のために単独で使用することもできます。

```toml
[dependencies]
reinhardt-templates-macros = "0.1.0"
```

## ライセンス

以下のいずれかのライセンスの下で利用可能です：

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好きな方を選択してください。
