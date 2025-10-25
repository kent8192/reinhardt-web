# reinhardt-graphql-macros

ReinhardtフレームワークのGraphQL-gRPC統合用Deriveマクロ

## 概要

`reinhardt-graphql-macros`は、ReinhardtフレームワークにおけるgRPCとGraphQLの統合を簡素化するための手続き型マクロを提供します。これらのマクロは、ボイラープレートを削減するために、変換コードとサブスクリプション実装を自動生成します。

## 機能

### 実装済み ✓

- **GrpcGraphQLConvert** - ProtobufとGraphQL型間の自動型変換
  - `From<proto::T> for T`と`From<T> for proto::T`を導出
  - `#[graphql(rename_all = "camelCase")]`によるフィールドリネーム
  - `#[graphql(skip_if = "...")]`による条件付きフィールド含有
  - `#[proto(...)]`属性によるカスタムprotobuf型マッピング

- **GrpcSubscription** - gRPCストリームからの自動GraphQLサブスクリプション
  - gRPCストリーミングメソッドをGraphQLサブスクリプションにマッピング
  - `#[grpc(service = "...", method = "...")]`によるサービスとメソッドの指定
  - `#[graphql(filter = "...")]`によるオプショナルフィルタリング
  - Rust 2024ライフタイム互換性

### 予定

- 追加の変換戦略
- 強化されたエラーハンドリング
- より多くのカスタマイズオプション
- 双方向ストリーミングサポート

## 使い方

### 型変換

```rust
use reinhardt_graphql_macros::GrpcGraphQLConvert;

#[derive(GrpcGraphQLConvert)]
#[graphql(rename_all = "camelCase")]
struct User {
    id: String,
    name: String,
    #[graphql(skip_if = "Option::is_none")]
    email: Option<String>,
}
```

これにより以下が生成されます:

- `From<proto::User> for User`
- `From<User> for proto::User`

### gRPCサブスクリプション

```rust
use reinhardt_graphql_macros::GrpcSubscription;

#[derive(GrpcSubscription)]
#[grpc(service = "UserEventsServiceClient", method = "subscribe_user_events")]
#[graphql(filter = "event_type == Created")]
struct UserCreatedSubscription;
```

これにより、以下を行うGraphQLサブスクリプション実装が自動生成されます:

- gRPCサービスに接続
- 指定されたメソッドにサブスクライブ
- 受信イベントにフィルタを適用
- gRPCメッセージをGraphQL型に変換

## 属性

### GrpcGraphQLConvert属性

- `#[graphql(rename_all = "...")]` - すべてのフィールドをリネーム (camelCase, snake_case, PascalCase)
- `#[graphql(skip_if = "...")]` - 述語がtrueの場合フィールドをスキップ
- `#[proto(type = "...")]` - カスタムprotobuf型を指定
- `#[proto(rename = "...")]` - protobufでのフィールド名をリネーム

### GrpcSubscription属性

- `#[grpc(service = "...")]` - gRPCサービスクライアント名
- `#[grpc(method = "...")]` - gRPCメソッド名
- `#[graphql(filter = "...")]` - イベントのフィルタ式

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
