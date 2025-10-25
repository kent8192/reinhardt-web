# reinhardt-grpc

Reinhardtフレームワーク向けのgRPC基盤クレート

## 概要

このクレートは、ReinhardtフレームワークにgRPC機能を提供するための基盤を提供します。フレームワークレベルの共通型とアダプタートレイトのみを含み、ドメイン固有の実装はユーザーが行います。

## 機能

### 1. 共通Protobuf型

フレームワークが提供する汎用的な型:

```protobuf
// Empty - 空のレスポンス
message Empty {}

// Timestamp - タイムスタンプ表現
message Timestamp {
  int64 seconds = 1;
  int32 nanos = 2;
}

// Error - エラー情報
message Error {
  string code = 1;
  string message = 2;
  map<string, string> metadata = 3;
}

// PageInfo - ページネーション情報
message PageInfo {
  int32 page = 1;
  int32 per_page = 2;
  int32 total = 3;
  bool has_next = 4;
  bool has_prev = 5;
}

// BatchResult - バッチ操作結果
message BatchResult {
  int32 success_count = 1;
  int32 failure_count = 2;
  repeated Error errors = 3;
}
```

### 2. アダプタートレイト

gRPCサービスを他のフレームワークコンポーネント(GraphQLなど)に統合するためのトレイト:

```rust
use reinhardt_grpc::{GrpcServiceAdapter, GrpcSubscriptionAdapter};

/// Query/Mutation用アダプター
#[async_trait]
pub trait GrpcServiceAdapter: Send + Sync {
    type Input;
    type Output;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

/// Subscription用アダプター
pub trait GrpcSubscriptionAdapter: Send + Sync {
    type Proto;
    type GraphQL;
    type Error: std::error::Error + Send + Sync + 'static;

    fn map_event(&self, proto: Self::Proto) -> Option<Self::GraphQL>;
}
```

### 3. エラー処理

gRPCエラー型と変換:

```rust
use reinhardt_grpc::{GrpcError, GrpcResult};

pub enum GrpcError {
    Connection(String),
    Service(String),
    NotFound(String),
    InvalidArgument(String),
    Internal(String),
}
```

## 使用方法

### 独自の.protoファイルを使用する

1. プロジェクトに`proto/`ディレクトリを作成

```
my-app/
├── proto/
│   ├── user.proto
│   └── product.proto
├── src/
│   └── main.rs
└── Cargo.toml
```

2. `build.rs`で.protoファイルをコンパイル

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_descriptors = protox::compile(
        &["proto/user.proto", "proto/product.proto"],
        &["proto"],
    )?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_fds(file_descriptors)?;

    Ok(())
}
```

3. `Cargo.toml`に依存関係を追加

```toml
[dependencies]
reinhardt-grpc = "0.1.0"
tonic = "0.12"
prost = "0.13"

[build-dependencies]
tonic-build = "0.12"
protox = "0.7"
```

4. 生成されたコードを使用

```rust
// src/lib.rs
pub mod proto {
    pub mod user {
        tonic::include_proto!("myapp.user");
    }
    pub mod product {
        tonic::include_proto!("myapp.product");
    }
}

// reinhardt-grpcの共通型を使用
use reinhardt_grpc::proto::common::{Empty, Timestamp, PageInfo};
```

### GraphQLとの統合

`reinhardt-graphql`クレートと組み合わせて使用する場合は、[reinhardt-graphqlのドキュメント](../reinhardt-contrib/crates/graphql/README.md)を参照してください。

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
