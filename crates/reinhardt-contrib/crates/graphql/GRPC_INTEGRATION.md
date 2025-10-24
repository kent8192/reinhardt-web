# GraphQL facade over gRPC

reinhardt-graphql は、gRPC サービスを GraphQL API として公開する機能を提供します。

## Feature Flags

```toml
[dependencies]
reinhardt-graphql = { version = "0.1.0", features = ["full"] }
# または個別に有効化
reinhardt-graphql = { version = "0.1.0", features = ["graphql-grpc", "subscription"] }
```

### Available Features

- `graphql-grpc`: Query/Mutation の gRPC 統合
- `subscription`: Subscription の gRPC 統合（Rust 2024 対応）
- `full`: 全機能有効化

## 使用方法

### 1. Protobuf 型と GraphQL 型の自動変換

`#[derive(GrpcGraphQLConvert)]` を使用すると、Protobuf 型と GraphQL 型の間の変換が自動生成されます:

```rust
use reinhardt_graphql::GrpcGraphQLConvert;
use async_graphql::SimpleObject;

#[derive(GrpcGraphQLConvert, SimpleObject)]
#[graphql(rename_all = "camelCase")]
struct User {
    id: String,
    name: String,
    email: Option<String>,
}

// 自動生成される:
// - From<proto::User> for User
// - From<User> for proto::User
```

### 2. Query/Mutation の gRPC 統合

`GrpcServiceAdapter` trait を実装してリゾルバーを作成:

```rust
use reinhardt_graphql::GrpcServiceAdapter;
use async_trait::async_trait;

struct UserServiceAdapter {
    grpc_client: proto::UserServiceClient<tonic::transport::Channel>,
}

#[async_trait]
impl GrpcServiceAdapter for UserServiceAdapter {
    type Input = String; // ユーザーID
    type Output = User;  // GraphQL User 型
    type Error = anyhow::Error;

    async fn call(&self, user_id: Self::Input) -> Result<Self::Output, Self::Error> {
        let request = proto::GetUserRequest { id: user_id };
        let response = self.grpc_client.get_user(request).await?;
        Ok(response.into_inner().into()) // proto → GraphQL 変換
    }
}

// GraphQL リゾルバー
struct Query;

#[Object]
impl Query {
    async fn user(&self, ctx: &Context<'_>, id: String) -> Result<User> {
        let adapter = ctx.data::<UserServiceAdapter>()?;
        adapter.call(id).await.map_err(|e| e.into())
    }
}
```

### 3. Subscription の gRPC 統合

`#[derive(GrpcSubscription)]` を使用すると、gRPC Server Streaming を GraphQL Subscription に自動マッピングします:

```rust
use reinhardt_graphql::GrpcSubscription;

#[derive(GrpcSubscription)]
#[grpc(service = "UserEventsServiceClient", method = "subscribe_user_events")]
#[graphql(filter = "event_type == Created")]
struct UserCreatedSubscription;

// 自動生成される GraphQL Subscription:
// subscription {
//   userCreated {
//     id
//     name
//     email
//   }
// }
```

**Rust 2024 対応:** このマクロは、Rust 2024 の lifetime キャプチャ問題を解決するため、`Box::pin` と明示的な lifetime アノテーションを使用します。

### 4. 手動実装（高度なユースケース）

より細かい制御が必要な場合は、`GrpcSubscriptionAdapter` を手動実装:

```rust
use reinhardt_graphql::GrpcSubscriptionAdapter;

struct UserEventsAdapter;

impl GrpcSubscriptionAdapter for UserEventsAdapter {
    type Proto = proto::UserEvent;
    type GraphQL = User;
    type Error = anyhow::Error;

    fn map_event(&self, proto: Self::Proto) -> Option<Self::GraphQL> {
        // イベントタイプでフィルタ
        if proto.event_type == proto::EventType::Created as i32 {
            proto.user.map(|u| u.into())
        } else {
            None
        }
    }
}

#[Subscription]
impl Subscription {
    async fn user_created<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = User> + 'ctx {
        use tokio_stream::StreamExt;

        let client = ctx.data::<proto::UserEventsServiceClient<_>>().unwrap();
        let adapter = UserEventsAdapter;

        let stream = client
            .subscribe_user_events(proto::SubscribeRequest::default())
            .await
            .unwrap()
            .into_inner();

        // Rust 2024 対応: Box::pin でラップ
        Box::pin(stream.filter_map(move |result| async move {
            match result {
                Ok(proto_event) => adapter.map_event(proto_event),
                Err(_) => None,
            }
        }))
    }
}
```

## アーキテクチャ

```
┌─────────────────┐
│  GraphQL Client │
└────────┬────────┘
         │ GraphQL Query/Mutation/Subscription
         ↓
┌─────────────────────────────────────┐
│  reinhardt-graphql                  │
│  ┌──────────────────────────────┐  │
│  │ GraphQL Schema & Resolvers   │  │
│  └────────┬─────────────────────┘  │
│           │                         │
│  ┌────────↓─────────────────────┐  │
│  │ GrpcServiceAdapter           │  │
│  │ GrpcSubscriptionAdapter      │  │
│  └────────┬─────────────────────┘  │
└───────────┼─────────────────────────┘
            │ gRPC (tonic)
            ↓
┌─────────────────────────────────────┐
│  reinhardt-grpc                     │
│  ┌──────────────────────────────┐  │
│  │ gRPC Services (User Service) │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

## Rust 2024 Subscription 問題の解決

async-graphql 7.0 は Rust 2024 の新しい lifetime キャプチャルールと互換性がありません。この問題は、gRPC Server Streaming を使用することで解決されます:

**従来の async-graphql Subscription（動作しない）:**

```rust
// Rust 2024 でコンパイルエラー
async fn user_created<'ctx>(&self, ctx: &Context<'ctx>)
    -> impl Stream<Item = User> + 'ctx
{
    async_stream::stream! {
        // lifetime キャプチャ問題
    }
}
```

**gRPC ベースの Subscription（動作する）:**

```rust
// Rust 2024 対応
async fn user_created<'ctx>(&self, ctx: &Context<'ctx>)
    -> impl Stream<Item = User> + 'ctx
{
    let stream = grpc_client.subscribe().await?.into_inner();
    Box::pin(stream.filter_map(/* ... */)) // ✅ OK
}
```

## パフォーマンス

- **Direct GraphQL**: ~3-4 µs/query
- **GraphQL over gRPC**: ~4-5 µs/query
- **オーバーヘッド**: 5-21% (+0.2-0.8 µs)

詳細は [PERFORMANCE.md](PERFORMANCE.md) を参照してください。

## サンプルコード

完全な実装例は `tests/` ディレクトリを参照:

- `tests/grpc_services/` - gRPC サービスの実装例
- `tests/proto/` - Protobuf 定義例

## ライセンス

MIT OR Apache-2.0
