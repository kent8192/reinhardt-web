# reinhardt-grpc

gRPC foundation crate for the Reinhardt framework

## Overview

This crate provides the foundation for gRPC functionality in the Reinhardt
framework. It includes only framework-level common types and adapter traits,
with domain-specific implementations left to users.

## Installation

Add `reinhardt` to your `Cargo.toml`:

<!-- reinhardt-version-sync:3 -->
```toml
[dependencies]
reinhardt = { version = "0.4.0-alpha.22", features = ["grpc"] }

# Or use a preset:
# reinhardt = { version = "0.4.0-alpha.22", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.4.0-alpha.22", features = ["full"] }      # All features
```

Then import gRPC features:

```rust
use reinhardt::grpc::{GrpcServiceAdapter, GrpcSubscriptionAdapter};
use reinhardt::grpc::proto::common::{Empty, Timestamp, PageInfo};
```

**Note:** gRPC features are included in the `standard` and `full` feature presets.

## Features

- Common Protobuf types (Empty, Timestamp, Error, PageInfo, BatchResult)
- Adapter traits for gRPC service integration
- Error handling utilities
- **Dependency injection support (optional, with `di` feature)**

### Mutual TLS and service authorization

Enable `grpc-tls` on the facade (or `tls` on this crate). Import protocol types
through `reinhardt::grpc::tonic`; a separate Tonic dependency is unnecessary for
transport configuration.

TLS configuration preserves an application-installed Rustls crypto provider.
If none is installed, it selects AWS-LC before configuring either server or
client TLS. This also supports dependency graphs enabling both Rustls AWS-LC
and Ring features. Applications requiring a different provider must install it
before their first gRPC TLS configuration.

```rust,no_run
use reinhardt::grpc::{GrpcClientTlsConfig, GrpcServerTlsConfig, PeerIdentity, PeerIdentityInterceptor, tonic};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let ca = std::fs::read("ca.pem")?;
let server_tls = GrpcServerTlsConfig::new(
    std::fs::read("server.pem")?, std::fs::read("server-key.pem")?, &ca,
);
let mut server = server_tls.configure(tonic::transport::Server::builder())?;
let policy = PeerIdentityInterceptor::new(|peer: &PeerIdentity| {
    if peer.uri_sans() == ["spiffe://example.test/service/marketplace"] {
        Ok(())
    } else {
        Err(tonic::Status::permission_denied("service is not authorized"))
    }
});
// Pass `policy` to your generated service's `with_interceptor` constructor,
// then add that service to `server` and serve its dedicated internal listener.
let channel = GrpcClientTlsConfig::new(
    ca, std::fs::read("client.pem")?, std::fs::read("client-key.pem")?, "auth.internal",
).endpoint("https://127.0.0.1:50051")?
 .connect_timeout(std::time::Duration::from_secs(5))
 .connect().await?;
# Ok(())
# }
```

The server requires certificates chaining to its configured client CA bundle.
The client verifies its explicit server CA and expected DNS name and refuses
plaintext endpoints. Authorization reads URI SANs from the verified client leaf
certificate, never request headers or common names. Attach a separate policy to
each generated service; handlers receive the authorized `PeerIdentity` in request
extensions. Custom async and per-method authorization belongs in handlers or
application middleware. Issuance, rotation, and trust-domain policy remain with
the application. The existing `runserver` listener does not automatically use
these settings: use the explicit configured transport for an mTLS listener.

### 1. Common Protobuf Types

Generic types provided by the framework:

```protobuf
// Empty - Empty response
message Empty {}

// Timestamp - Timestamp representation
message Timestamp {
  int64 seconds = 1;
  int32 nanos = 2;
}

// Error - Error information
message Error {
  string code = 1;
  string message = 2;
  map<string, string> metadata = 3;
}

// PageInfo - Pagination information
message PageInfo {
  int32 page = 1;
  int32 per_page = 2;
  int32 total = 3;
  bool has_next = 4;
  bool has_prev = 5;
}

// BatchResult - Batch operation result
message BatchResult {
  int32 success_count = 1;
  int32 failure_count = 2;
  repeated Error errors = 3;
}
```

### 2. Adapter Traits

Traits for integrating gRPC services with other framework components (such as
GraphQL):

```rust
use reinhardt::grpc::{GrpcServiceAdapter, GrpcSubscriptionAdapter};

/// Adapter for Query/Mutation
#[async_trait]
pub trait GrpcServiceAdapter: Send + Sync {
    type Input;
    type Output;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

/// Adapter for Subscription
pub trait GrpcSubscriptionAdapter: Send + Sync {
    type Proto;
    type GraphQL;
    type Error: std::error::Error + Send + Sync + 'static;

    fn map_event(&self, proto: Self::Proto) -> Option<Self::GraphQL>;
}
```

### 3. Error Handling

gRPC error types and conversions:

```rust
use reinhardt::grpc::{GrpcError, GrpcResult};

pub enum GrpcError {
    Connection(String),
    Service(String),
    NotFound(String),
    InvalidArgument(String),
    Internal(String),
}
```

## Usage

### Registering Generated Services

Configure generated Tonic services before registering them with `GrpcRouter`:

```rust,ignore
use reinhardt_grpc::GrpcRouter;

pub fn grpc_services() -> GrpcRouter {
    GrpcRouter::new().service(ChatServiceServer::new(ChatService::default()))
}
```

### Using Your Own .proto Files

1. Create a `proto/` directory in your project

```
my-app/
├── proto/
│   ├── user.proto
│   └── product.proto
├── src/
│   └── main.rs
└── Cargo.toml
```

2. Compile .proto files in `build.rs`

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

3. Add dependencies to `Cargo.toml`

<!-- reinhardt-version-sync -->
```toml
[dependencies]
reinhardt-grpc = "0.4.0-alpha.22"
tonic = "0.12"
prost = "0.13"

[build-dependencies]
tonic-build = "0.12"
protox = "0.7"
```

4. Use generated code

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

// Use common types from reinhardt-grpc
use reinhardt::grpc::proto::common::{Empty, Timestamp, PageInfo};
```

### Dependency Injection

Facade consumers can enable `grpc` alongside a preset that includes DI:

<!-- reinhardt-version-sync -->
```toml
[dependencies]
reinhardt = { version = "0.4.0-alpha.22", package = "reinhardt-web", default-features = false, features = ["minimal", "grpc"] }
```

Direct `reinhardt-grpc` consumers can instead enable this crate's `di`
feature explicitly and depend on `reinhardt-di` for DI types:

<!-- reinhardt-version-sync:2 -->
```toml
[dependencies]
reinhardt-grpc = { version = "0.4.0-alpha.22", features = ["di"] }
reinhardt-di = "0.4.0-alpha.22"
```

The basic example below uses the facade configuration. Direct consumers should
use the following imports instead:

```rust
use reinhardt_di::InjectionContext;
use reinhardt_grpc::{GrpcRequestExt, grpc_handler};
```

#### Basic Usage

```rust
use reinhardt::di::InjectionContext;
use reinhardt::grpc::{GrpcRequestExt, grpc_handler};
use tonic::{Request, Response, Status};
use std::sync::Arc;

pub struct UserServiceImpl {
    injection_context: Arc<InjectionContext>,
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn get_user(&self, mut request: Request<GetUserRequest>)
        -> Result<Response<User>, Status>
    {
        // Set DI context in request extensions
        request.extensions_mut().insert(self.injection_context.clone());

        // Call handler with DI support
        self.get_user_impl(request).await
    }
}

impl UserServiceImpl {
    #[grpc_handler]
    async fn get_user_impl(
        &self,
        request: Request<GetUserRequest>,
        #[inject] db: DatabaseConnection,  // Auto-injected
    ) -> Result<Response<User>, Status> {
        let user_id = request.into_inner().id;
        let user = db.fetch_user(user_id).await?;
        Ok(Response::new(user))
    }
}
```

#### Cache Control

Control dependency caching with the `cache` parameter:

```rust
#[grpc_handler]
async fn handler(
    &self,
    request: Request<Req>,
    #[inject] cached_db: DatabaseConnection,          // Cached (default)
    #[inject(cache = false)] fresh_db: DatabaseConnection,  // Not cached
) -> Result<Response<Resp>, Status> {
    // ...
}
```

Injected parameters may use normal Rust binding patterns, including mutable
bindings and newtype destructuring. The generated wrapper preserves the pattern
on the handler implementation while forwarding the resolved dependency value.

```rust,ignore
#[inject] mut db: DatabaseConnection
#[inject] Wrapper(mut value): Wrapper<Data>
```

Mutability applies only to the handler function's internal binding; it does not
change resolver ownership or caching.

### Integration with GraphQL

When using with the `reinhardt-graphql` crate, refer to the
[reinhardt-graphql documentation](../reinhardt-graphql/README.md).

## License

Licensed under the BSD 3-Clause License.
