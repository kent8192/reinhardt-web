use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper_util::rt::TokioIo;
use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use crate::shutdown::ShutdownCoordinator;

/// HTTP Server with middleware support
pub struct HttpServer {
    pub handler: Arc<dyn Handler>,
    pub(crate) middlewares: Vec<Arc<dyn reinhardt_types::Middleware>>,
}

impl HttpServer {
    /// Create a new server with the given handler
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use reinhardt_server::HttpServer;
    /// use reinhardt_types::Handler;
    /// use reinhardt_http::{Request, Response};
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for MyHandler {
    ///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
    ///         Ok(Response::ok().with_body("Hello"))
    ///     }
    /// }
    ///
    /// let handler = Arc::new(MyHandler);
    /// let server = HttpServer::new(handler);
    /// ```
    pub fn new(handler: Arc<dyn Handler>) -> Self {
        Self {
            handler,
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the server using builder pattern
    ///
    /// Middlewares are executed in the order they are added.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use reinhardt_server::HttpServer;
    /// use reinhardt_types::{Handler, Middleware};
    /// use reinhardt_http::{Request, Response};
    ///
    /// struct MyHandler;
    /// struct MyMiddleware;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for MyHandler {
    ///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
    ///         Ok(Response::ok())
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Middleware for MyMiddleware {
    ///     async fn process(&self, request: Request, next: Arc<dyn Handler>) -> reinhardt_exception::Result<Response> {
    ///         next.handle(request).await
    ///     }
    /// }
    ///
    /// let handler = Arc::new(MyHandler);
    /// let middleware = Arc::new(MyMiddleware);
    /// let server = HttpServer::new(handler)
    ///     .with_middleware(middleware);
    /// ```
    pub fn with_middleware(mut self, middleware: Arc<dyn reinhardt_types::Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Build the final handler with middleware chain
    ///
    /// This creates a MiddlewareChain that wraps the handler with all configured middlewares.
    fn build_handler(&self) -> Arc<dyn Handler> {
        if self.middlewares.is_empty() {
            return self.handler.clone();
        }

        let mut chain = reinhardt_types::MiddlewareChain::new(self.handler.clone());
        for middleware in &self.middlewares {
            chain.add_middleware(middleware.clone());
        }

        Arc::new(chain)
    }
    /// Start the server and listen on the given address
    ///
    /// This method starts the server and begins accepting connections.
    /// It runs indefinitely until an error occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use reinhardt_server::HttpServer;
    /// use reinhardt_types::Handler;
    /// use reinhardt_http::{Request, Response};
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for MyHandler {
    ///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
    ///         Ok(Response::ok())
    ///     }
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = Arc::new(MyHandler);
    /// let server = HttpServer::new(handler);
    /// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    /// server.listen(addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn listen(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("Server listening on http://{}", addr);

        // Build the handler with middleware chain
        let handler = self.build_handler();

        loop {
            let (stream, socket_addr) = listener.accept().await?;
            let handler = handler.clone();

            tokio::task::spawn(async move {
                if let Err(err) = Self::handle_connection(stream, socket_addr, handler).await {
                    eprintln!("Error handling connection: {:?}", err);
                }
            });
        }
    }

    /// Start the server with graceful shutdown support
    ///
    /// This method starts the server and listens for shutdown signals.
    /// When a shutdown signal is received, it stops accepting new connections
    /// and waits for existing connections to complete.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use std::time::Duration;
    /// use reinhardt_server::{HttpServer, ShutdownCoordinator};
    /// use reinhardt_types::Handler;
    /// use reinhardt_http::{Request, Response};
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for MyHandler {
    ///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
    ///         Ok(Response::ok())
    ///     }
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = Arc::new(MyHandler);
    /// let server = HttpServer::new(handler);
    /// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    /// let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
    /// server.listen_with_shutdown(addr, coordinator).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn listen_with_shutdown(
        self,
        addr: SocketAddr,
        coordinator: ShutdownCoordinator,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("Server listening on http://{}", addr);

        // Build the handler with middleware chain
        let handler = self.build_handler();

        let mut shutdown_rx = coordinator.subscribe();

        loop {
            tokio::select! {
                // Accept new connection
                result = listener.accept() => {
                    let (stream, socket_addr) = result?;
                    let handler = handler.clone();
                    let mut conn_shutdown = coordinator.subscribe();

                    tokio::task::spawn(async move {
                        // Handle connection with shutdown support
                        tokio::select! {
                            result = Self::handle_connection(stream, socket_addr, handler) => {
                                if let Err(err) = result {
                                    eprintln!("Error handling connection: {:?}", err);
                                }
                            }
                            _ = conn_shutdown.recv() => {
                                // Connection interrupted by shutdown
                            }
                        }
                    });
                }
                // Shutdown signal received
                _ = shutdown_rx.recv() => {
                    println!("Shutdown signal received, stopping server...");
                    break;
                }
            }
        }

        // Notify that server has stopped accepting connections
        coordinator.notify_shutdown_complete();

        Ok(())
    }
    /// Handle a single TCP connection by processing HTTP requests
    ///
    /// This is an internal method used by the server to process individual connections.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use tokio::net::TcpStream;
    /// use reinhardt_server::HttpServer;
    /// use reinhardt_types::Handler;
    /// use reinhardt_http::{Request, Response};
    ///
    /// struct MyHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for MyHandler {
    ///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
    ///         Ok(Response::ok())
    ///     }
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = Arc::new(MyHandler);
    /// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    /// let stream = TcpStream::connect(addr).await?;
    /// let socket_addr = stream.peer_addr()?;
    /// HttpServer::handle_connection(stream, socket_addr, handler).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn handle_connection(
        stream: TcpStream,
        socket_addr: SocketAddr,
        handler: Arc<dyn Handler>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let io = TokioIo::new(stream);
        let service = RequestService {
            handler,
            remote_addr: socket_addr,
        };

        http1::Builder::new().serve_connection(io, service).await?;

        Ok(())
    }
}

/// Service implementation for hyper
struct RequestService {
    handler: Arc<dyn Handler>,
    remote_addr: SocketAddr,
}

impl Service<hyper::Request<Incoming>> for RequestService {
    type Response = hyper::Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn call(&self, req: hyper::Request<Incoming>) -> Self::Future {
        let handler = self.handler.clone();
        let remote_addr = self.remote_addr;

        Box::pin(async move {
            // Extract request parts
            let (parts, body) = req.into_parts();

            // Read body
            let body_bytes = body.collect().await?.to_bytes();

            // Create reinhardt Request
            let mut request = Request::new(
                parts.method,
                parts.uri,
                parts.version,
                parts.headers,
                Bytes::from(body_bytes),
            );
            request.remote_addr = Some(remote_addr);

            // Handle request
            let response = handler
                .handle(request)
                .await
                .unwrap_or_else(|_| Response::internal_server_error());

            // Convert to hyper response
            let mut hyper_response = hyper::Response::builder().status(response.status);

            // Add headers
            for (key, value) in response.headers.iter() {
                hyper_response = hyper_response.header(key, value);
            }

            Ok(hyper_response.body(Full::new(response.body))?)
        })
    }
}
/// Helper function to create and run a server
///
/// This is a convenience function that creates an `HttpServer` and starts listening.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use std::net::SocketAddr;
/// use reinhardt_server::serve;
/// use reinhardt_types::Handler;
/// use reinhardt_http::{Request, Response};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for MyHandler {
///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
///         Ok(Response::ok().with_body("Hello, World!"))
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let handler = Arc::new(MyHandler);
/// let addr: SocketAddr = "127.0.0.1:3000".parse()?;
/// serve(addr, handler).await?;
/// # Ok(())
/// # }
/// ```
pub async fn serve(
    addr: SocketAddr,
    handler: Arc<dyn Handler>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new(handler);
    server.listen(addr).await
}

/// Helper function to create and run a server with graceful shutdown
///
/// This function sets up a server with shutdown signal handling and graceful shutdown support.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use std::net::SocketAddr;
/// use std::time::Duration;
/// use reinhardt_server::{serve_with_shutdown, shutdown_signal, ShutdownCoordinator};
/// use reinhardt_types::Handler;
/// use reinhardt_http::{Request, Response};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for MyHandler {
///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
///         Ok(Response::ok().with_body("Hello, World!"))
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let handler = Arc::new(MyHandler);
/// let addr: SocketAddr = "127.0.0.1:3000".parse()?;
/// let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
///
/// tokio::select! {
///     result = serve_with_shutdown(addr, handler, coordinator.clone()) => {
///         result?;
///     }
///     _ = shutdown_signal() => {
///         coordinator.shutdown();
///         coordinator.wait_for_shutdown().await;
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub async fn serve_with_shutdown(
    addr: SocketAddr,
    handler: Arc<dyn Handler>,
    coordinator: ShutdownCoordinator,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new(handler);
    server.listen_with_shutdown(addr, coordinator).await
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait::async_trait]
    impl Handler for TestHandler {
        async fn handle(&self, _request: Request) -> reinhardt_exception::Result<Response> {
            Ok(Response::ok().with_body("Hello, World!"))
        }
    }

    #[tokio::test]
    async fn test_http_server_creation() {
        let _server = HttpServer::new(Arc::new(TestHandler));
        // Just verify server can be created without panicking
    }

    #[tokio::test]
    async fn test_http_server_with_middleware() {
        use reinhardt_types::Middleware;

        struct TestMiddleware {
            prefix: String,
        }

        #[async_trait::async_trait]
        impl Middleware for TestMiddleware {
            async fn process(
                &self,
                request: Request,
                next: Arc<dyn Handler>,
            ) -> reinhardt_exception::Result<Response> {
                let response = next.handle(request).await?;
                let current_body = String::from_utf8(response.body.to_vec()).unwrap_or_default();
                let new_body = format!("{}{}", self.prefix, current_body);
                Ok(Response::ok().with_body(new_body))
            }
        }

        let middleware = Arc::new(TestMiddleware {
            prefix: "Middleware: ".to_string(),
        });

        let server = HttpServer::new(Arc::new(TestHandler))
            .with_middleware(middleware);

        // Verify middleware is added
        assert_eq!(server.middlewares.len(), 1);
    }

    #[tokio::test]
    async fn test_http_server_multiple_middlewares() {
        use reinhardt_types::Middleware;

        struct PrefixMiddleware {
            prefix: String,
        }

        #[async_trait::async_trait]
        impl Middleware for PrefixMiddleware {
            async fn process(
                &self,
                request: Request,
                next: Arc<dyn Handler>,
            ) -> reinhardt_exception::Result<Response> {
                let response = next.handle(request).await?;
                let current_body = String::from_utf8(response.body.to_vec()).unwrap_or_default();
                let new_body = format!("{}{}", self.prefix, current_body);
                Ok(Response::ok().with_body(new_body))
            }
        }

        let mw1 = Arc::new(PrefixMiddleware {
            prefix: "MW1:".to_string(),
        });
        let mw2 = Arc::new(PrefixMiddleware {
            prefix: "MW2:".to_string(),
        });

        let server = HttpServer::new(Arc::new(TestHandler))
            .with_middleware(mw1)
            .with_middleware(mw2);

        assert_eq!(server.middlewares.len(), 2);
    }

    #[tokio::test]
    async fn test_middleware_chain_execution() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Middleware;

        struct PrefixMiddleware {
            prefix: String,
        }

        #[async_trait::async_trait]
        impl Middleware for PrefixMiddleware {
            async fn process(
                &self,
                request: Request,
                next: Arc<dyn Handler>,
            ) -> reinhardt_exception::Result<Response> {
                let response = next.handle(request).await?;
                let current_body = String::from_utf8(response.body.to_vec()).unwrap_or_default();
                let new_body = format!("{}{}", self.prefix, current_body);
                Ok(Response::ok().with_body(new_body))
            }
        }

        let mw1 = Arc::new(PrefixMiddleware {
            prefix: "First:".to_string(),
        });
        let mw2 = Arc::new(PrefixMiddleware {
            prefix: "Second:".to_string(),
        });

        let server = HttpServer::new(Arc::new(TestHandler))
            .with_middleware(mw1)
            .with_middleware(mw2);

        // Build the handler with middleware chain
        let handler = server.build_handler();

        // Create a test request
        let request = Request::new(
            Method::GET,
            "/".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Execute the handler
        let response = handler.handle(request).await.unwrap();
        let body = String::from_utf8(response.body.to_vec()).unwrap();

        // Middlewares should be applied in order: First -> Second -> Handler
        assert_eq!(body, "First:Second:Hello, World!");
    }
}
