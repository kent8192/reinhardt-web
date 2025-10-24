# Reinhardt Dispatch

Reinhardtフレームワークの HTTPリクエストディスパッチおよびハンドラーシステム

## 概要

`reinhardt-dispatch`は、Djangoの`django.core.handlers`と`django.dispatch`に相当するコアリクエストハンドリング機能を提供します。ミドルウェア実行、シグナル発行、例外処理を含む完全なリクエストライフサイクルをオーケストレーションします。

## 機能

- リクエストライフサイクル管理
- ミドルウェアチェイン（リクエスト/レスポンス処理のための組み合わせ可能なミドルウェア）
- シグナル統合（`request_started`、`request_finished`ライフサイクルシグナルを発行）
- 例外処理（エラーを適切なHTTPレスポンスに変換）
- 非同期サポート（Tokioによる完全なasync/awaitサポート）

## コンポーネント

- **BaseHandler**: コアリクエストハンドラー
- **MiddlewareChain**: 複数のミドルウェアコンポーネントを処理パイプラインに組み立て
- **Dispatcher**: ハンドラーとフレームワークの残りの部分を調整する高レベルディスパッチャー
- **ExceptionHandler**: カスタム例外処理のためのトレイト
- **DefaultExceptionHandler**: 標準エラーレスポンス