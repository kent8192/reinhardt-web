# I18N 要件ドキュメント

## 背景

削除された`reinhardt-template`クレートには約1,414行のi18nテストが存在していました。
この文書はそれらのテストから要件を抽出し、`reinhardt-pages`における将来のi18n実装の指針とします。

## 目的

`reinhardt-pages`フレームワークに国際化（i18n）機能を統合し、多言語対応のWASMアプリケーションを構築可能にする。

---

## コア要件

### 1. 基本翻訳機能

#### 1.1 単純な翻訳（`gettext`）
```rust
use reinhardt_pages::i18n::{gettext, _};

let message = gettext("welcome_message");
// または省略形
let message = _("welcome_message");
```

**要件**:
- メッセージIDから翻訳文字列を取得
- 翻訳が見つからない場合はメッセージIDをそのまま返す
- 実行時のロケール切り替えに対応

#### 1.2 複数形対応翻訳（`ngettext`）
```rust
use reinhardt_pages::i18n::ngettext;

let message = ngettext(
    "You have {count} message",
    "You have {count} messages",
    count
);
```

**要件**:
- 数値に応じて単数形/複数形を選択
- 言語ごとの複数形ルール（CLDR準拠）をサポート
  - 英語: 2形式（1, その他）
  - 日本語: 1形式（常に同じ）
  - ロシア語: 3形式（1, 2-4, その他）
  - アラビア語: 6形式
- ゼロの扱いは言語ごとに異なる

#### 1.3 コンテキスト付き翻訳（`pgettext`）
```rust
use reinhardt_pages::i18n::pgettext;

let month = pgettext("calendar", "May");  // カレンダーの5月
let modal = pgettext("permission", "May");  // 許可の「できる」
```

**要件**:
- 同じ単語でも文脈によって異なる翻訳を提供
- コンテキストIDとメッセージIDの組み合わせで管理

---

### 2. ロケール管理

#### 2.1 ロケール検出
```rust
use reinhardt_pages::i18n::Locale;

// ブラウザのAccept-Languageヘッダーから自動検出
let locale = Locale::from_browser_default();

// 明示的に指定
let locale = Locale::new("ja-JP");
```

**要件**:
- ブラウザのAccept-Languageヘッダーから自動検出
- ユーザー設定による上書き（Cookie、LocalStorage）
- フォールバックチェーン: `ja-JP` → `ja` → `en` → デフォルト

#### 2.2 動的ロケール切り替え
```rust
use reinhardt_pages::i18n::set_locale;

// ロケールを変更
set_locale(Locale::new("fr-FR"));

// リアクティブに更新
let locale_signal = use_locale();
locale_signal.set(Locale::new("de-DE"));
```

**要件**:
- 実行時のロケール切り替え
- Signalベースのリアクティブ更新
- 切り替え時に全UIコンポーネントが自動更新

#### 2.3 ロケール情報取得
```rust
let current_locale = get_locale();
let language_code = current_locale.language(); // "ja"
let country_code = current_locale.country();   // "JP"
let full_code = current_locale.code();         // "ja-JP"
```

---

### 3. 文字列補間

#### 3.1 Named Placeholders
```rust
let message = _("Hello, {name}!").format([("name", "Alice")]);
// "Hello, Alice!"

let message = _("Order #{order_id} for {customer}").format([
    ("order_id", "12345"),
    ("customer", "Bob"),
]);
// "Order #12345 for Bob"
```

**要件**:
- `{name}` 形式のプレースホルダー
- 複数のプレースホルダーをサポート
- プレースホルダーが見つからない場合は警告を出す

#### 3.2 Positional Placeholders
```rust
let message = _("Item {0} of {1}").format_positional([1, 10]);
// "Item 1 of 10"
```

**要件**:
- `{0}`, `{1}`, `{2}` 形式のプレースホルダー
- 0-indexed

#### 3.3 Reactiveな値の補間
```rust
let count = Signal::new(0);
let message = _("You have {count} items").format_signal([
    ("count", count.clone())
]);

// countが変更されるとmessageも自動更新
count.set(5); // message = "You have 5 items"
```

**要件**:
- `Signal<T>`をプレースホルダーに使用可能
- Signalの値が変更されると翻訳文字列も自動更新
- Effectと連携してリアクティブに動作

---

### 4. ローカライゼーション（フォーマット）

#### 4.1 日付・時刻フォーマット
```rust
use reinhardt_pages::i18n::format_date;

let date = DateTime::from_timestamp(1640000000, 0);

// 日本語
set_locale(Locale::new("ja-JP"));
format_date(&date, "long");    // "2021年12月20日"
format_date(&date, "short");   // "2021/12/20"
format_time(&date, "long");    // "午後6時26分40秒"

// 英語
set_locale(Locale::new("en-US"));
format_date(&date, "long");    // "December 20, 2021"
format_date(&date, "short");   // "12/20/2021"
format_time(&date, "long");    // "6:26:40 PM"
```

**要件**:
- CLDR準拠のフォーマット
- カスタムフォーマット文字列のサポート
- タイムゾーン対応

#### 4.2 数値フォーマット
```rust
use reinhardt_pages::i18n::format_number;

let value = 1234567.89;

// 日本語
set_locale(Locale::new("ja-JP"));
format_number(value);  // "1,234,567.89"

// ドイツ語（ピリオドとカンマが逆）
set_locale(Locale::new("de-DE"));
format_number(value);  // "1.234.567,89"

// アラビア語（アラビア数字）
set_locale(Locale::new("ar-SA"));
format_number(value);  // "١٬٢٣٤٬٥٦٧٫٨٩"
```

**要件**:
- 千の位区切り記号（カンマ、ピリオド、スペースなど）
- 小数点記号（ピリオド、カンマ）
- 数字の表示形式（アラビア数字、アラビア語数字など）

#### 4.3 通貨フォーマット
```rust
use reinhardt_pages::i18n::format_currency;

let amount = 1234.56;

// 日本円
set_locale(Locale::new("ja-JP"));
format_currency(amount, "JPY");  // "¥1,235"

// 米ドル
set_locale(Locale::new("en-US"));
format_currency(amount, "USD");  // "$1,234.56"

// ユーロ（フランス）
set_locale(Locale::new("fr-FR"));
format_currency(amount, "EUR");  // "1 234,56 €"
```

**要件**:
- 通貨記号の位置（前、後）
- 小数点以下の桁数（通貨ごとに異なる）
- 負の値の表示（`-$10`, `($10)`, `$-10` など）

---

## 提案する実装アーキテクチャ

### フロントエンド（WASM）

```rust
use reinhardt_pages::i18n::{I18n, Locale, _};
use reinhardt_pages::component::{Component, View};
use reinhardt_pages::reactive::Signal;

struct AppComponent {
    locale: Signal<Locale>,
}

impl Component for AppComponent {
    fn render(&self) -> View {
        View::element("div")
            .child(View::element("h1").child(_("welcome_title")))
            .child(View::element("p").child(_("welcome_message")))
            .child(self.render_language_switcher())
            .into_view()
    }

    fn name() -> &'static str {
        "AppComponent"
    }
}

impl AppComponent {
    fn render_language_switcher(&self) -> View {
        View::element("select")
            .on("change", move |event| {
                let lang = event.target_value();
                self.locale.set(Locale::new(&lang));
            })
            .child(View::element("option").attr("value", "en").child("English"))
            .child(View::element("option").attr("value", "ja").child("日本語"))
            .child(View::element("option").attr("value", "fr").child("Français"))
            .into_view()
    }
}
```

### バックエンド（翻訳データ提供）

#### 翻訳ファイル形式（JSON）
```json
{
  "locale": "ja-JP",
  "messages": {
    "welcome_title": "ようこそ",
    "welcome_message": "Reinhardtフレームワークへようこそ！",
    "items_count": {
      "one": "{count}個のアイテム",
      "other": "{count}個のアイテム"
    }
  },
  "contexts": {
    "calendar": {
      "May": "5月"
    },
    "permission": {
      "May": "できる"
    }
  }
}
```

#### ビルド時コンパイル
```rust
// build.rs
use reinhardt_i18n_build::compile_translations;

fn main() {
    compile_translations("locales/")
        .output("src/generated/translations.rs")
        .compile();
}
```

#### サーバーからのフェッチ
```rust
use reinhardt_pages::i18n::load_translations;

async fn initialize_i18n() {
    // サーバーから翻訳データをフェッチ
    let translations = fetch("/api/i18n/ja-JP").await?;
    load_translations(translations).await?;
}
```

---

## テストケース例

### 基本翻訳
```rust
#[test]
fn test_basic_translation() {
    set_locale(Locale::new("ja-JP"));
    load_translations(japanese_translations());

    assert_eq!(_("welcome"), "ようこそ");
    assert_eq!(_("goodbye"), "さようなら");
}
```

### 複数形
```rust
#[test]
fn test_plural_forms() {
    set_locale(Locale::new("en-US"));

    assert_eq!(ngettext("1 item", "{count} items", 0).format([("count", 0)]), "0 items");
    assert_eq!(ngettext("1 item", "{count} items", 1).format([("count", 1)]), "1 item");
    assert_eq!(ngettext("1 item", "{count} items", 5).format([("count", 5)]), "5 items");
}
```

### ロケールフォールバック
```rust
#[test]
fn test_locale_fallback() {
    set_locale(Locale::new("ja-JP"));

    // ja-JPに存在しない場合はjaにフォールバック
    assert_eq!(_("only_in_ja"), "日本語のみ");

    // jaにも存在しない場合はenにフォールバック
    assert_eq!(_("only_in_en"), "English only");

    // どこにも存在しない場合はメッセージIDを返す
    assert_eq!(_("missing_key"), "missing_key");
}
```

### リアクティブ更新
```rust
#[test]
fn test_reactive_locale_change() {
    let locale = Signal::new(Locale::new("en-US"));
    set_locale_signal(locale.clone());

    let message = use_translation("welcome");
    assert_eq!(message.get(), "Welcome");

    locale.set(Locale::new("ja-JP"));
    assert_eq!(message.get(), "ようこそ");
}
```

---

## 実装の優先順位

### Phase 1: 基礎実装（必須）
1. ✅ 基本的な`gettext`関数
2. ✅ ロケール管理（設定、取得）
3. ✅ 翻訳データのロード（JSON）
4. ✅ 文字列補間（named placeholders）

### Phase 2: 高度な翻訳（重要）
1. ⬜ 複数形対応（`ngettext`）
2. ⬜ コンテキスト付き翻訳（`pgettext`）
3. ⬜ ロケールフォールバック
4. ⬜ Signalベースのリアクティブ翻訳

### Phase 3: ローカライゼーション（推奨）
1. ⬜ 日付・時刻フォーマット（CLDR準拠）
2. ⬜ 数値フォーマット
3. ⬜ 通貨フォーマット

### Phase 4: 開発者体験（オプション）
1. ⬜ マクロベースのAPI（`t!("key")`）
2. ⬜ コンポーネントベースのAPI（`<Trans key="..." />`）
3. ⬜ ビルド時の翻訳キー検証
4. ⬜ 翻訳漏れの警告

---

## 参考実装

### Django i18n
- `gettext()`, `ngettext()`, `pgettext()`
- `{% trans %}`, `{% blocktrans %}` テンプレートタグ
- `.po`/`.mo`ファイルフォーマット

### Fluent（Mozilla）
- メッセージ構文: `hello = Hello, {$name}!`
- 複数形: `emails = { $count -> [one] {$count} email *[other] {$count} emails }`
- バンドルベースのローディング

### react-i18next
- `useTranslation()` フック
- `<Trans>` コンポーネント
- 名前空間のサポート

---

## セキュリティ考慮事項

### 1. XSS対策
翻訳文字列はユーザー入力ではないが、プレースホルダーの値はエスケープが必要：

```rust
let username = user_input; // 信頼できない入力
let message = _("Hello, {name}!").format([("name", escape_html(username))]);
```

### 2. インジェクション攻撃
翻訳キーにユーザー入力を使用しない：

```rust
// ❌ 危険
let key = format!("message.{}", user_input);
let message = _(key);

// ✅ 安全
match user_input {
    "greeting" => _("message.greeting"),
    "farewell" => _("message.farewell"),
    _ => _("message.default"),
}
```

---

## パフォーマンス考慮事項

### 1. 遅延ロード
```rust
// 初期ロードは最小限に
load_translations(Locale::new("en-US"), ["common", "auth"]).await;

// 必要になったら追加ロード
load_translations_namespace(Locale::new("en-US"), "admin").await;
```

### 2. キャッシュ
```rust
// 翻訳結果をメモ化
let message = Memo::new(move || {
    let locale = use_locale();
    _("expensive_translation")
});
```

### 3. ビルド時最適化
```rust
// 未使用の翻訳を削除
#[cfg(feature = "optimize-translations")]
fn tree_shake_translations() {
    // ...
}
```

---

## 今後の拡張

### 1. RTL言語サポート
- アラビア語、ヘブライ語などの右から左への言語
- `dir="rtl"` 属性の自動設定

### 2. 翻訳管理ツール
- Webベースの翻訳エディタ
- 翻訳進捗の可視化
- 翻訳者向けコメント

### 3. 機械翻訳統合
- Google Translate API
- DeepL API
- 翻訳候補の自動生成

---

## まとめ

このドキュメントは`reinhardt-pages`におけるi18n実装の指針となります。実装時にはこのドキュメントを参照し、段階的に機能を追加していくことを推奨します。
