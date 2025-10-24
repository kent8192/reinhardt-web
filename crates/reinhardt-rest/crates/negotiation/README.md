# reinhardt-negotiation

Django REST Framework風のコンテンツネゴシエーションシステム

## Overview

クライアントの優先順位に基づいて最適なレスポンスフォーマットを決定するコンテンツネゴシエーションシステムです。Accept headerの解析、メディアタイプマッチング、レンダラー選択を行います。

## Implemented ✓

### Core Components

#### MediaType

- **MIMEタイプの表現**: `type/subtype` 形式のメディアタイプ
- **パラメータサポート**: `charset=utf-8` などのパラメータを保持
- **品質値 (q値)**: Accept headerの優先度を示す `q` パラメータ
- **パース機能**: 文字列からMediaTypeオブジェクトへの変換
- **ワイルドカードマッチング**: `*/*`, `application/*` などのワイルドカード対応
- **優先度計算**: より具体的なメディアタイプに高い優先度を付与
- **完全文字列表現**: パラメータを含む完全な文字列生成

#### AcceptHeader

- **Accept headerパーサー**: HTTP Accept headerの解析
- **品質値によるソート**: q値の高い順に自動ソート
- **最適マッチ検索**: 利用可能なメディアタイプから最適なものを選択
- **空のAccept header対応**: Accept headerがない場合の処理

#### ContentNegotiator

- **レンダラー選択**: Accept headerに基づく最適なレンダラーの選択
- **デフォルトメディアタイプ**: 設定可能なデフォルト値 (初期値: `application/json`)
- **ネゴシエーション**: クライアントの要求と利用可能なフォーマットのマッチング
- **フォーマットパラメータ対応**: `?format=json` 形式のクエリパラメータ
- **レンダラーフィルタリング**: フォーマット名によるレンダラーの絞り込み
- **パラメータ付きAccept header**: `application/json; indent=8` などの詳細指定に対応
- **ワイルドカード処理**: `*/*` の場合は最初のレンダラーを使用

### Django REST Framework互換機能

- **BaseContentNegotiation trait**: DRFの `BaseContentNegotiation` に相当する抽象インターフェース
- **select_renderer メソッド**: レンダラー選択ロジック
- **select_parser メソッド**: パーサー選択ロジック (基本実装)
- **NegotiationError**: ネゴシエーション失敗時のエラー型

### 特徴

- **型安全性**: Rustの型システムを活用した安全な実装
- **ゼロコスト抽象化**: パフォーマンスを犠牲にしない設計
- **詳細なドキュメント**: すべての公開APIにdoctestを含むドキュメント
- **包括的なテスト**: DRFの動作を再現する統合テスト

## Planned

### 今後の拡張予定

- **カスタムネゴシエーション戦略**: プラグイン可能なネゴシエーションロジック
- **Content-Type検出**: リクエストボディのContent-Type自動検出
- **言語ネゴシエーション**: Accept-Language headerのサポート
- **エンコーディングネゴシエーション**: Accept-Encoding headerのサポート
- **キャッシュ最適化**: ネゴシエーション結果のキャッシング
- **より詳細なエラー情報**: ネゴシエーション失敗時の詳細なフィードバック

## Usage Examples

### Basic Content Negotiation

```rust
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let negotiator = ContentNegotiator::new();
let available = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

// Accept headerに基づくネゴシエーション
let result = negotiator.negotiate("text/html, application/json", &available);
assert_eq!(result.subtype, "html"); // 最初にマッチした html が選択される
```

### Format Parameter Selection

```rust
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let negotiator = ContentNegotiator::new();
let available = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

// ?format=json のようなクエリパラメータによる選択
let result = negotiator.select_by_format("json", &available);
assert_eq!(result.unwrap().subtype, "json");
```

### Renderer Selection

```rust
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let negotiator = ContentNegotiator::new();
let renderers = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

let result = negotiator.select_renderer(
    Some("application/json"),
    &renderers
);
assert!(result.is_ok());
let (media_type, media_type_str) = result.unwrap();
assert_eq!(media_type.subtype, "json");
```

## License

このクレートは reinhardt プロジェクトの一部であり、Apache License 2.0 または MIT License のデュアルライセンスです。
