+++
title = "Request Admission"
weight = 70
+++

# Request Admission

Guide to admitting requests before the server reads their bodies, both in deployment and under `runserver`.

## Table of Contents

- [Why Admit Before the Body](#why-admit-before-the-body)
- [Writing an Admission Policy](#writing-an-admission-policy)
- [Applying a Policy to a Server](#applying-a-policy-to-a-server)
- [Applying a Policy Under runserver](#applying-a-policy-under-runserver)
- [Sharing the Policy with Application Code](#sharing-the-policy-with-application-code)

---

## Why Admit Before the Body

The native HTTP server collects the whole request body before it calls the router. Router middleware therefore cannot cap the body size, limit how long a client takes to send the body, or reject a request under overload before the body arrives. Without an admission policy, the server accepts bodies up to its default size limit and waits for them without a timeout.

An admission policy reads only the request head: the method, URI, headers, and peer address. It runs before the server reads the body. For each request, the policy either rejects the request with a response or admits it with:

- A **body budget**: the maximum number of body bytes the server collects for this request.
- A **receive timeout**: the maximum time the server waits for the whole body.
- An **admission guard**: any `Send` value, such as a semaphore permit, that the server holds until the handler finishes or the request is cancelled.

The server answers `413 Payload Too Large` when the body exceeds the budget and `408 Request Timeout` when the body does not arrive in time. A rejection response from the policy goes to the client unchanged: the router, server middleware, and exception handler do not see it.

---

## Writing an Admission Policy

Implement `RequestAdmission`:

```rust
use async_trait::async_trait;
use hyper::StatusCode;
use reinhardt::http::Response;
use reinhardt::server::{AdmittedRequest, RequestAdmission, RequestHead};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub struct Protection {
    permits: Arc<Semaphore>,
}

impl Protection {
    pub fn new(concurrency: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(concurrency)),
        }
    }
}

#[async_trait]
impl RequestAdmission for Protection {
    async fn admit(&self, head: RequestHead<'_>) -> Result<AdmittedRequest, Response> {
        let Ok(permit) = self.permits.clone().try_acquire_owned() else {
            return Err(Response::new(StatusCode::SERVICE_UNAVAILABLE));
        };
        let body_budget = if head.uri.path().starts_with("/uploads/") {
            6 * 1024 * 1024
        } else {
            1024 * 1024
        };
        Ok(AdmittedRequest::new(body_budget, Duration::from_secs(30), permit))
    }
}
```

The server calls `admit` for every request, so keep it fast: try to acquire a resource instead of waiting for it.

---

## Applying a Policy to a Server

Pass the policy to `HttpServer::with_request_admission`. `Http2Server` has the same method.

```rust
use reinhardt::server::HttpServer;
use std::sync::Arc;

let server = HttpServer::new(router)
    .with_request_admission(Arc::new(Protection::new(64)));
```

---

## Applying a Policy Under runserver

`runserver` builds its own `HttpServer`, so a project supplies the policy through a runserver hook. Implement `RunserverHook::request_admission`:

```rust
use async_trait::async_trait;
use reinhardt::commands::{RunserverContext, RunserverHook};
use reinhardt::server::RequestAdmission;
use std::error::Error;
use std::sync::Arc;

#[reinhardt::hook(on = runserver)]
struct AdmissionHook;

#[async_trait]
impl RunserverHook for AdmissionHook {
    async fn request_admission(
        &self,
        _ctx: &RunserverContext,
    ) -> Result<Option<Arc<dyn RequestAdmission>>, Box<dyn Error + Send + Sync>> {
        Ok(Some(Arc::new(Protection::new(64))))
    }
}
```

`runserver` applies these rules:

- It calls `request_admission` on every hook after every hook's `on_server_start` has completed, and before it builds the HTTP server. At startup, it prints a `[VERBOSE]` line that names the hook that supplied the policy.
- At most one hook may return a policy. Hook discovery order is not stable across builds, so when several hooks return one, startup fails with an error that names each of them.
- A hook that returns `Err` stops startup.
- The policy admits every request on the HTTP listener, including the `--with-pages` static and SPA responses, OpenAPI routes, and WebSocket handshakes. To treat development-only routes differently, branch on `head.uri` in `admit`. The policy does not apply to the gRPC listener.
- With autoreload, only the `--noreload` child process calls the hook, so every rebuild starts with a fresh policy.

---

## Sharing the Policy with Application Code

Application code may need the same policy state, for example to apply per-user limits against the same permits. Register one instance in DI during `on_server_start`, and return that instance from `request_admission`:

```rust
#[async_trait]
impl RunserverHook for AdmissionHook {
    async fn on_server_start(
        &self,
        ctx: &RunserverContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        ctx.di_context.singleton_scope().set(Protection::new(64));
        Ok(())
    }

    async fn request_admission(
        &self,
        ctx: &RunserverContext,
    ) -> Result<Option<Arc<dyn RequestAdmission>>, Box<dyn Error + Send + Sync>> {
        let protection = ctx
            .di_context
            .singleton_scope()
            .get::<Protection>()
            .ok_or("Protection is not registered")?;
        Ok(Some(protection))
    }
}
```

The server inserts a `PreBodyAdmitted` marker into the extensions of every request that a policy admitted. If the same handlers can also run behind a server without a policy, check for the marker and do the admission work only when it is absent:

```rust
use reinhardt::server::PreBodyAdmitted;

if request.extensions.get::<PreBodyAdmitted>().is_none() {
    // No policy admitted this request; acquire the permit here instead.
}
```
