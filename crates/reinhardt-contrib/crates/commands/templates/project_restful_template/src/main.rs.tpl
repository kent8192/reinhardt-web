//! Main entry point for {{ project_name }}

mod config;
mod apps;

use config::settings::get_settings;
use config::apps::get_installed_apps;
use config::urls::url_patterns;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // This is a placeholder - in a real implementation, this would route through
    // the router and dispatch to the appropriate handler
    Ok(Response::new(Body::from("Hello from {{ project_name }}!")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load settings based on environment (REINHARDT_ENV)
    let settings = get_settings();

    // Initialize app registry with compile-time validated apps
    reinhardt_core::init_apps_checked(get_installed_apps)?;

    // Get URL configuration
    let router = url_patterns();

    // Configure server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    // Create service
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    // Build and run server
    let server = Server::bind(&addr).serve(make_svc);

    println!("Starting server at http://{}", addr);
    println!("Debug mode: {}", settings.debug);
    println!("Quit the server with CTRL-C");

    // Run server
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}
