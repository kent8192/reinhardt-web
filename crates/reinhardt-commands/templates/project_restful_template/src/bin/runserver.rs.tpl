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
use config::apps::get_installed_apps;
use config::urls::url_patterns;

/// Router handler that dispatches requests to the UnifiedRouter
struct RouterHandler {
    router: Arc<UnifiedRouter>,
}

#[async_trait::async_trait]
impl Handler for RouterHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        // TODO: Implement actual routing dispatch logic
        // For now, return a simple JSON response showing the route was received
        let response_body = serde_json::json!({
            "message": "Welcome to {{ project_name }} API",
            "path": request.uri.to_string(),
            "method": format!("{:?}", request.method),
            "note": "Router integration coming soon"
        });

        Ok(Response::ok()
            .with_header("Content-Type", "application/json")
            .with_body(serde_json::to_string_pretty(&response_body)?))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load settings
    let settings = get_settings();

    // Initialize app registry with compile-time validated apps
    reinhardt::core::init_apps_checked(get_installed_apps)?;

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
    println!("{}", style("Reinhardt RESTful API Server").cyan().bold());
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
