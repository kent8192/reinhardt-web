# reinhardt-utils

Common utilities and helper functions

## Overview

Collection of utility functions and helpers used throughout the framework.

Includes date/time utilities, string manipulation, encoding/decoding, and other common operations.

## Features

### Implemented ✓

#### HTML Utilities (`html` module)

- **HTML Escaping/Unescaping**
  - `escape()`: HTML特殊文字のエスケープ (`<`, `>`, `&`, `"`, `'`)
  - `unescape()`: HTMLエンティティのアンエスケープ
  - `conditional_escape()`: 条件付きエスケープ（autoescapeフラグ対応）
  - `escape_attr()`: HTML属性値用エスケープ（改行・タブも処理）
- **HTML操作**
  - `strip_tags()`: HTMLタグの除去
  - `strip_spaces_between_tags()`: タグ間の空白除去
  - `truncate_html_words()`: HTMLタグを保持したまま単語数で切り詰め
  - `format_html()`: プレースホルダー置換によるHTML生成
- **安全な文字列**
  - `SafeString`: 自動エスケープをバイパスするための安全文字列型

#### Encoding Utilities (`encoding` module)

- **URL Encoding**
  - `urlencode()`: URLエンコード（スペースは`+`に変換）
  - `urldecode()`: URLデコード
- **JavaScript Escaping**
  - `escapejs()`: JavaScript文字列用エスケープ（引用符、制御文字、特殊文字対応）
- **Slugification**
  - `slugify()`: URL用スラッグ生成（小文字化、特殊文字除去、ハイフン区切り）
- **Text Processing**
  - `truncate_chars()`: 文字数で切り詰め（`...`付加）
  - `truncate_words()`: 単語数で切り詰め（`...`付加）
  - `wrap_text()`: 指定幅でテキストを折り返し
  - `force_str()`: バイト列を安全にUTF-8文字列に変換
  - `force_bytes()`: 文字列をバイト列に変換
- **Line Break Processing**
  - `linebreaks()`: 改行を`<br>`タグに変換（段落分割対応）
  - `linebreaksbr()`: 改行を`<br>`タグに変換（単純版）

#### Date/Time Formatting (`dateformat` module)

- **Django/PHP式フォーマット**
  - `format()`: フォーマット文字列による日時フォーマット
  - 対応フォーマットコード：
    - 年: `Y`（4桁）, `y`（2桁）
    - 月: `m`（ゼロ埋め）, `n`（ゼロなし）, `F`（完全名）, `M`（略称）
    - 日: `d`（ゼロ埋め）, `j`（ゼロなし）, `l`（曜日名）, `D`（曜日略称）
    - 時: `H`（24時間）, `h`（12時間）, `G`/`g`（ゼロなし版）
    - 分: `i`, 秒: `s`
    - AM/PM: `A`（大文字）, `a`（小文字）
- **ショートカット関数** (`shortcuts`サブモジュール)
  - `iso_date()`: YYYY-MM-DD形式
  - `iso_datetime()`: YYYY-MM-DD HH:MM:SS形式
  - `us_date()`: MM/DD/YYYY形式
  - `eu_date()`: DD/MM/YYYY形式
  - `full_date()`: "Monday, January 1, 2025"形式
  - `short_date()`: "Jan 1, 2025"形式
  - `time_24()`: 24時間形式時刻
  - `time_12()`: 12時間形式時刻（AM/PM付き）

#### Text Manipulation (`text` module)

- **大文字小文字変換**
  - `capfirst()`: 各単語の先頭を大文字化
  - `title()`: タイトルケース変換（全単語の先頭大文字、残り小文字）
- **数値フォーマット**
  - `intcomma()`: 整数に3桁区切りカンマ追加
  - `floatcomma()`: 浮動小数点数に3桁区切りカンマ追加
  - `ordinal()`: 序数接尾辞追加（1st, 2nd, 3rd, 4th等）
- **単数複数形**
  - `pluralize()`: カウントに基づく単数/複数形切り替え
- **パディング**
  - `ljust()`: 左詰め（右パディング）
  - `rjust()`: 右詰め（左パディング）
  - `center()`: 中央揃え（両側パディング）
- **電話番号フォーマット**
  - `phone_format()`: 10桁/11桁の電話番号を`(XXX) XXX-XXXX`形式に変換

#### Timezone Utilities (`timezone` module)

- **基本的な日時取得**
  - `now()`: 現在のUTC時刻
  - `localtime()`: 現在のローカル時刻
- **タイムゾーン変換**
  - `to_local()`: UTC→ローカルタイムゾーン変換
  - `to_utc()`: ローカル→UTC変換
  - `to_timezone()`: 指定IANA名でのタイムゾーン変換（現在はUTCのみ対応）
- **Naive/Aware変換**
  - `make_aware_utc()`: Naive日時をUTCタイムゾーン付きに変換
  - `make_aware_local()`: Naive日時をローカルタイムゾーン付きに変換
  - `is_aware()`: タイムゾーン情報の有無確認（Rustでは常に`true`）
- **パース/フォーマット**
  - `parse_datetime()`: ISO 8601形式の日時文字列をパース
  - `format_datetime()`: 日時をISO 8601形式（RFC 3339）で出力
- **タイムゾーン名取得**
  - `get_timezone_name_utc()`: UTC日時のタイムゾーン名取得
  - `get_timezone_name_local()`: ローカル日時のタイムゾーン名取得

### Planned

Currently all planned features are implemented.
