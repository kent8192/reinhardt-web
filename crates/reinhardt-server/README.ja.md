# reinhardt-server

ReinhardtフレームワークのHTTPサーバー実装

## 概要

`reinhardt-server`は、Hyperをベースに構築された、WebSocketとGraphQLサポートを備えたReinhardtアプリケーション向けの高性能HTTPサーバー実装を提供します。このクレートは、サーバー関連機能を統合する親クレートとして機能します。

## 機能

このクレートは`server`サブクレートから機能を再エクスポートしています：

- **コアHTTPサーバー**: 高性能HTTP/1.1サーバー
  - Tokioランタイムによる非同期リクエスト処理
  - Handlerトレイトによるカスタムハンドラーサポート
  - 効率的なTCP接続管理
  - 自動リクエスト/レスポンス変換
  - 組み込みエラーハンドリング

- **WebSocketサポート** (feature = "websocket"): WebSocketサーバー実装
  - tokio-tungstenitベースのWebSocketサーバー
  - カスタムメッセージハンドラーサポート
  - 接続ライフサイクルフック（on_connect、on_disconnect）
  - テキストおよびバイナリメッセージ処理
  - 自動接続管理

- **GraphQLサポート** (feature = "graphql"): GraphQLエンドポイント統合
  - async-graphql統合
  - QueryとMutationルート用のスキーマビルダー
  - GraphQLクエリのPOSTリクエスト処理
  - JSONレスポンスシリアライゼーション
  - GraphQLエラーのエラーハンドリング