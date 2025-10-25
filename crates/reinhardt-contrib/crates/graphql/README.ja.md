# reinhardt-graphql

GraphQL統合機能

## 概要

GraphQL API サポートを提供します。モデルからのスキーマ生成、クエリおよびミューテーションリゾルバー、認証・権限システムとの統合が含まれます。REST API の柔軟な代替手段を提供します。

## 機能

### 実装済み ✓

#### コア型システム

- **GraphQL型マーカー**: 型安全なGraphQL型定義のための `GraphQLType` および `GraphQLField` トレイト
- **エラーハンドリング**: Schema、Resolver、NotFound バリアントを持つカスタム `GraphQLError` 列挙型
- **ベースリゾルバートレイト**: 柔軟なリゾルバー実装のための汎用出力型を持つ非同期 `Resolver` トレイト

#### スキーマ & データ型

- **Userタイプ**: id、name、email、active フィールドを持つ完全なGraphQLオブジェクト実装
- **Userストレージ**: `Arc<RwLock<HashMap>>` を使用したスレッドセーフなインメモリストレージ
  - `new()`: 新しいストレージインスタンスを作成
  - `add_user()`: ストレージにユーザーを追加または更新
  - `get_user()`: IDでユーザーを取得
  - `list_users()`: 保存されているすべてのユーザーをリスト表示
- **入力タイプ**: ユーザー作成ミューテーション用の `CreateUserInput`
- **スキーマビルダー**: データコンテキストを使ってGraphQLスキーマを構築する `create_schema()` 関数

#### クエリ操作

- **Userクエリ**:
  - `user(id: ID)`: IDで単一ユーザーを取得
  - `users()`: すべてのユーザーをリスト表示
  - `hello(name: Option<String>)`: テスト用のシンプルな挨拶クエリ
- **コンテキスト統合**: クエリはGraphQLコンテキストを通じてUserStorageにアクセス

#### ミューテーション操作

- **Userミューテーション**:
  - `createUser(input: CreateUserInput)`: 自動生成されたUUIDで新しいユーザーを作成
  - `updateUserStatus(id: ID, active: bool)`: ユーザーのアクティブステータスを更新
- **状態管理**: ミューテーションはUserStorageへの変更を永続化

#### サブスクリプションシステム

- **イベントタイプ**: Created、Updated、Deleted イベントをサポートする `UserEvent` 列挙型
- **イベントブロードキャスト**: tokio broadcast チャンネルを使った `EventBroadcaster` (容量: 100)
  - `new()`: 新しいブロードキャスターインスタンスを作成
  - `broadcast()`: すべてのサブスクライバーにイベントを送信
  - `subscribe()`: イベントストリームをサブスクライブ
- **サブスクリプションルート**: フィルタリングされたサブスクリプションストリームを持つ `SubscriptionRoot`
  - `userCreated()`: ユーザー作成イベントのストリーム
  - `userUpdated()`: ユーザー更新イベントのストリーム
  - `userDeleted()`: ユーザー削除イベントのストリーム (IDのみを返す)
- **非同期ストリーム**: async-stream を使用したリアルタイムイベントフィルタリング

#### 統合

- **async-graphql統合**: 本番環境対応のGraphQLサーバーのためにasync-graphqlフレームワーク上に構築
- **型安全性**: コンパイル時保証を持つRust型システムの完全統合
- **Async/Await**: tokioランタイムによる完全な非同期サポート
- **ドキュメント**: すべての公開APIに対する例付きの包括的なdocコメント

#### gRPCトランスポート (オプション - `graphql-grpc` フィーチャー)

- **GraphQL over gRPC サービス**: GraphQL用のgRPCプロトコルを実装する `GraphQLGrpcService`
  - `execute_query()`: unary RPCを介してGraphQLクエリを実行
  - `execute_mutation()`: unary RPCを介してGraphQLミューテーションを実行
  - `execute_subscription()`: サーバーストリーミングRPCを介してGraphQLサブスクリプションを実行
- **Protocol Buffers**: `reinhardt-grpc` クレートに完全なproto定義
  - `GraphQLRequest`: query、variables、operation_name
  - `GraphQLResponse`: data、errors、extensions
  - `SubscriptionEvent`: id、event_type、payload、timestamp
- **リクエスト/レスポンス変換**: gRPCとasync-graphql型間の自動変換
- **エラーハンドリング**: 完全なエラー情報の伝播 (message、locations、path、extensions)
- **パフォーマンス**: 直接実行と比較して最小限のオーバーヘッド (5-21%、または0.2-0.8 µs)
- **ネットワーク通信**: tonic経由の完全なTCP/HTTP2サポート
- **ストリーミング**: リアルタイムサブスクリプション用の効率的なサーバーサイドストリーミング

### 予定

現在、すべての予定機能が実装されています。

## インストール

```toml
[dependencies]
# 基本的なGraphQLサポート
reinhardt-graphql = "0.1.0"

# gRPCトランスポート付き
reinhardt-graphql = { version = "0.1.0", features = ["graphql-grpc"] }

# すべての機能
reinhardt-graphql = { version = "0.1.0", features = ["full"] }
```

## 使用例

### 基本的なGraphQL使用例

```rust
use async_graphql::{EmptySubscription, Schema};
use reinhardt_graphql::schema::{Mutation, Query, UserStorage};

#[tokio::main]
async fn main() {
    let storage = UserStorage::new();
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(storage)
        .finish();

    let query = r#"{ hello(name: "World") }"#;
    let result = schema.execute(query).await;
    println!("{}", result.data);
}
```

### GraphQL over gRPC サーバー

```rust
use async_graphql::{EmptySubscription, Schema};
use reinhardt_graphql::grpc_service::GraphQLGrpcService;
use reinhardt_graphql::schema::{Mutation, Query, UserStorage};
use reinhardt_grpc::proto::graphql::graph_ql_service_server::GraphQlServiceServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = UserStorage::new();
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(storage)
        .finish();

    let service = GraphQLGrpcService::new(schema);
    let grpc_service = GraphQlServiceServer::new(service);

    Server::builder()
        .add_service(grpc_service)
        .serve("127.0.0.1:50051".parse()?)
        .await?;

    Ok(())
}
```

### GraphQL over gRPC クライアント

```rust
use reinhardt_grpc::proto::graphql::{
    graph_ql_service_client::GraphQlServiceClient,
    GraphQlRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GraphQlServiceClient::connect("http://127.0.0.1:50051").await?;

    let request = tonic::Request::new(GraphQlRequest {
        query: r#"{ hello(name: "gRPC") }"#.to_string(),
        variables: None,
        operation_name: None,
    });

    let response = client.execute_query(request).await?;
    println!("{:?}", response.into_inner());

    Ok(())
}
```

### 使用例の実行

```bash
# gRPCサーバーを起動
cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_server

# 別のターミナルでクライアントを実行
cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_client
```

## テスト

```bash
# すべてのテスト
cargo test --package reinhardt-graphql --features graphql-grpc

# 統合テスト
cargo test --package reinhardt-graphql --features graphql-grpc --test grpc_integration_tests

# サブスクリプションストリーミングテスト
cargo test --package reinhardt-graphql --features graphql-grpc --test grpc_subscription_tests

# 実際のネットワークを使用したE2Eテスト
cargo test --package reinhardt-graphql --features graphql-grpc --test grpc_e2e_tests

# パフォーマンスベンチマーク
cargo bench --package reinhardt-graphql --features graphql-grpc
```

## パフォーマンス

詳細なベンチマークについては [PERFORMANCE.md](PERFORMANCE.md) を参照してください。

**概要:**

- 直接GraphQL: クエリあたり約3-4 µs
- gRPC GraphQL: クエリあたり約4-5 µs
- オーバーヘッド: gRPCシリアライゼーションで5-21% (+0.2-0.8 µs)
- どちらのアプローチも実際のアプリケーションで高いパフォーマンスを発揮
