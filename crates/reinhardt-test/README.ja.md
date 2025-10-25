# reinhardt-test

Reinhardtフレームワーク用のテストユーティリティとテストクライアント

## 概要

Django REST Frameworkのテストユーティリティにインスパイアされた包括的なテストユーティリティ。このクレートは、テストリクエストを作成するためのAPIClient、テストケースの基底クラス、データベースフィクスチャ、モックユーティリティ、インフラストラクチャテスト用のTestContainers統合など、再利用可能なテストツールを提供します。

実データベースまたはテストデータベースを使用した単体テストと統合テストの両方をサポートします。

## 機能

### 実装済み ✓

#### APIテストクライアント

- **APIClient**: 認証サポート付きHTTPテストクライアント
  - HTTPメソッド: GET、POST、PUT、PATCH、DELETE、HEAD、OPTIONS
  - 認証: 強制認証、Basic認証、ログイン/ログアウト
  - リクエストカスタマイズ: ヘッダー、クッキー、ベースURL設定
  - 柔軟なシリアライゼーション: JSONおよびフォームエンコードデータのサポート
- **APIRequestFactory**: テストリクエストを作成するためのファクトリ
  - すべてのHTTPメソッドのリクエストビルダー
  - JSONおよびフォームデータのシリアライゼーション
  - ヘッダーとクエリパラメータの管理
  - 強制認証のサポート

#### レスポンステスト

- **TestResponse**: アサーションヘルパー付きレスポンスラッパー
  - ステータスコードアサーション: `assert_ok()`、`assert_created()`、`assert_not_found()`など
  - ステータス範囲チェック: `assert_success()`、`assert_client_error()`、`assert_server_error()`
  - ボディの解析: JSONデシリアライゼーション、テキスト抽出
  - ヘッダーアクセスとコンテンツタイプチェック

#### テストケース基底クラス

- **APITestCase**: 共通のセットアップ/ティアダウンを持つ基底テストケース
  - 事前設定されたAPIClientインスタンス
  - セットアップとティアダウンのライフサイクルフック
  - オプションのTestContainersデータベース統合
- **テストマクロ**: テストケース定義のための便利なマクロ
  - `test_case!`: 標準テストケース定義
  - `authenticated_test_case!`: 事前認証済みテストケース
  - `test_case_with_db!`: データベース対応テストケース（`testcontainers`フィーチャーが必要）

#### フィクスチャとファクトリ

- **FixtureLoader**: JSONベースのテストデータローダー
  - JSON文字列からフィクスチャを読み込み
  - 型安全なデシリアライゼーション
  - フィクスチャの存在確認とリスト表示
- **Factoryトレイト**: テストデータ生成
  - `Factory<T>`トレイトによるテストオブジェクトの作成
  - `FactoryBuilder`: シンプルなファクトリ実装
  - バッチデータ生成のサポート

#### モックとスパイユーティリティ

- **MockFunction**: 設定可能な戻り値を持つ関数呼び出し追跡
  - 戻り値のキューイングとデフォルト値
  - 呼び出し回数と引数の追跡
  - 条件付きアサーション: `was_called()`、`was_called_with()`
- **Spy**: オプションのラップされたオブジェクトを持つメソッド呼び出し追跡
  - タイムスタンプ付き呼び出し記録
  - 引数の検証
  - リセットと検査機能

#### メッセージテスト（Djangoスタイル）

- **メッセージアサーション**: メッセージフレームワーク統合のテスト
  - `assert_message_count()`: メッセージ数の検証
  - `assert_message_exists()`: 特定のメッセージの確認
  - `assert_message_level()`: メッセージレベルの検証
  - `assert_message_tags()`: メッセージタグの確認
  - `assert_messages()`: 順序付き・順序なしメッセージ検証
- **MessagesTestMixin**: メッセージテストユーティリティ用のテストミックスイン
  - より見やすいテスト出力のためのスタックトレースフィルタリング
  - タグベースのメッセージアサーション

#### JSONアサーション

- **JSONフィールドアサーション**:
  - `assert_json_field_eq()`: フィールド値の等価性
  - `assert_json_has_field()`: フィールドの存在
  - `assert_json_missing_field()`: フィールドの不在
- **JSON配列アサーション**:
  - `assert_json_array_len()`: 配列長の検証
  - `assert_json_array_empty()` / `assert_json_array_not_empty()`: 空状態のチェック
  - `assert_json_array_contains()`: 要素の存在
- **JSONパターンマッチング**:
  - `assert_json_matches()`: 複雑な構造のサブセットマッチング

#### HTTPアサーション

- **ステータスコードアサーション**:
  - `assert_status_eq()`: 正確なステータスコードマッチング
  - `assert_status_success()`: 2xx範囲の検証
  - `assert_status_client_error()`: 4xx範囲の検証
  - `assert_status_server_error()`: 5xx範囲の検証
  - `assert_status_redirect()`: 3xx範囲の検証
  - `assert_status_error()`: 4xxまたは5xxの検証
- **コンテンツアサーション**:
  - `assert_contains()`: テキスト部分文字列の存在
  - `assert_not_contains()`: テキスト部分文字列の不在

#### デバッグツールバー

- **DebugToolbar**: リクエスト/レスポンスデバッグユーティリティ
  - タイミング情報の追跡（合計時間、SQL時間、キャッシュヒット/ミス）
  - 実行時間とスタックトレース付きSQLクエリ記録
  - さまざまなエントリタイプ（キーバリュー、テーブル、コード、テキスト）を持つカスタムデバッグパネル
  - デバッグ出力のHTMLレンダリング
  - デバッグの有効化/無効化サポート

#### TestContainers統合（オプション、`testcontainers`フィーチャーが必要）

- **データベースコンテナ**:
  - `PostgresContainer`: カスタム認証情報を持つPostgreSQLテストコンテナ
  - `MySqlContainer`: カスタム認証情報を持つMySQLテストコンテナ
  - `RedisContainer`: Redisテストコンテナ
- **TestDatabaseトレイト**: データベースコンテナの共通インターフェース
  - 接続URL生成
  - データベースタイプの識別
  - 準備状態の確認
- **ヘルパー関数**:
  - `with_postgres()`: PostgreSQLコンテナを使用してテストを実行
  - `with_mysql()`: MySQLコンテナを使用してテストを実行
  - `with_redis()`: Redisコンテナを使用してテストを実行

### 予定

現在、予定されている機能はすべて実装済みです。
