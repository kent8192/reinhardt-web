# reinhardt-exception

例外処理とエラータイプ

## 概要

Djangoに触発された例外階層を備えた包括的なエラー処理を提供します。HTTP例外、バリデーションエラー、データベースエラー、詳細なエラーメッセージを持つカスタムエラータイプを含みます。

## 機能

### 実装済み ✓

- **Djangoスタイルの例外階層** - カテゴリ化されたエラータイプを持つ包括的な`Error`列挙型
- **HTTPステータスコード例外** - `Http`、`Authentication` (401)、`Authorization` (403)、`NotFound` (404)、`Internal` (500)など
- **バリデーションエラー処理** - フィールドレベルのエラーサポート付きの`Validation`バリアント
- **データベース例外タイプ** - DB関連エラー用の`Database`バリアント
- **カスタムエラータイプ** - `ImproperlyConfigured`、`BodyAlreadyConsumed`、`ParseError`など
- **エラーシリアライゼーション** - すべてのエラーは`Display`を実装し、`status_code()`メソッドを介してHTTPレスポンスに変換可能
- **thiserror統合** - 派生エラー実装のための`thiserror`との完全な統合
- **anyhow統合** - 互換性のために任意の`anyhow::Error`をラップする`Other`バリアント
- **エラー分類** - カテゴリ分類のための`ErrorKind`列挙型
- **標準変換** - `serde_json::Error`、`std::io::Error`、`http::Error`のための`From`実装

### 予定

現在、すべての予定された機能が実装されています。
