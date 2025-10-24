# reinhardt-shortcuts

ReinhardtフレームワークのDjango風ショートカット関数

## 概要

Djangoの`django.shortcuts`モジュールに着想を得た、一般的なHTTP操作のための便利なショートカット関数です。これらの関数は、レスポンスの作成、リダイレクト、自動404エラー処理を備えたデータベースクエリの処理のための、シンプルで直感的なAPIを提供します。

## 機能

- リダイレクトショートカット
  - `redirect()`: 一時的リダイレクト（HTTP 302）
  - `redirect_permanent()`: 永続的リダイレクト（HTTP 301）

- レスポンスレンダリングショートカット
  - `render_json()`: データをJSONとしてレンダリング
  - `render_json_pretty()`: 整形されたJSONをレンダリング
  - `render_html()`: HTMLコンテンツをレンダリング
  - `render_text()`: プレーンテキストをレンダリング

- データベースショートカット（404エラー処理）
  - `get_or_404_response()`: 単一オブジェクトを取得、見つからない場合は404
  - `get_list_or_404_response()`: オブジェクトリストを取得、空の場合は404
  - `exists_or_404_response()`: レコードが存在するか確認、存在しない場合は404

- ORM統合（`database`フィーチャーが必要）
  - `get_object_or_404()`: データベースから単一オブジェクトを直接取得
  - `get_list_or_404()`: データベースからリストを直接取得

- テンプレート統合（`templates`フィーチャーが必要）
  - `render_template()`: コンテキストを使用したテンプレートレンダリング
  - `render_to_response()`: カスタムレスポンス設定を伴う高度なテンプレートレンダリング
  - 基本的な変数置換（`{{ variable }}`構文）
  - ファイルシステムからのテンプレート読み込み

## エラー型

- `GetError`
  - `NotFound`: データベースでオブジェクトが見つからない
  - `MultipleObjectsReturned`: 1つが期待されるところで複数のオブジェクトが返された
  - `DatabaseError(String)`: データベース操作エラー