//! Development server for {{ project_name }}
//!
//! This binary starts the development server with the registered UnifiedRouter.
//! Similar to Django's `./manage.py runserver` command.

use console::style;
use reinhardt::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;

mod config;
mod apps;

use config::settings::get_settings;
use config::urls::url_patterns;

/// Router handler that dispatches requests to the UnifiedRouter
struct RouterHandler {
    router: Arc<UnifiedRouter>,
}

#[async_trait::async_trait]
impl Handler for RouterHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        // TODO: Implement actual routing dispatch logic
        // For now, return a simple HTML response
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{{ project_name }}</title>
    <style>
        body {{ font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }}
        h1 {{ color: #2c3e50; }}
        .info {{ background: #ecf0f1; padding: 15px; border-radius: 5px; }}
    </style>
</head>
<body>
    <h1>Welcome to {{ project_name }}!</h1>
    <div class="info">
        <p><strong>Path:</strong> {}</p>
        <p><strong>Method:</strong> {:?}</p>
        <p><em>Router integration coming soon</em></p>
    </div>
</body>
</html>"#,
            request.uri, request.method
        );

        Ok(Response::ok()
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(html))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load settings
    let settings = get_settings();

    // Initialize app registry
    reinhardt::core::init_apps(settings.installed_apps.clone())?;

    // Get URL configuration and register globally
    let router = url_patterns();
    reinhardt::register_router(router.clone());

    // Parse server address from args or use default
    let args: Vec<String> = std::env::args().collect();
    let addr_str = args.get(1)
        .map(|s| s.as_str())
        .unwrap_or("127.0.0.1:8000");

    let addr: SocketAddr = addr_str.parse()
        .map_err(|e| format!("Invalid address '{}': {}", addr_str, e))?;

    // Create handler with router
    let handler = Arc::new(RouterHandler { router });
    let server = reinhardt::server::HttpServer::new(handler);

    // Print startup messages
    println!();
    println!("{}", style("Reinhardt Development Server").cyan().bold());
    println!("{}", style("─".repeat(50)).dim());
    println!();
    println!("  {} http://{}", style("✓").green(), style(&addr).cyan().bold());
    println!("  {} {}", style("Environment:").dim(), style(&settings.debug).yellow());
    println!();
    println!("{}", style("Quit the server with CTRL+C").dim());
    println!();

    // Start server
    server.listen(addr).await?;

    Ok(())
}
