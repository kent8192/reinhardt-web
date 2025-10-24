//! Show all URL patterns registered in the application
//!
//! This binary displays all routes registered in the UnifiedRouter,
//! similar to Django's `./manage.py show_urls` command.

use console::style;
use reinhardt::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", style("API URL Configuration").cyan().bold());
    println!();

    // Check if router is registered
    if !reinhardt::is_router_registered() {
        eprintln!("{}", style("Error: Router not registered").red().bold());
        eprintln!("Make sure your application has called reinhardt::register_router()");
        std::process::exit(1);
    }

    // Get the global router
    let router = reinhardt::get_router()
        .expect("Router should be registered");

    // Get all routes
    let routes = router.get_all_routes();

    if routes.is_empty() {
        println!("{}", style("No routes registered").yellow());
        return Ok(());
    }

    println!("{}", style(format!("Found {} route(s):", routes.len())).green());
    println!();

    // Group routes by namespace
    let mut by_namespace: std::collections::HashMap<Option<String>, Vec<_>> =
        std::collections::HashMap::new();

    for route in routes {
        by_namespace.entry(route.2.clone()).or_default().push(route);
    }

    // Sort namespaces
    let mut namespaces: Vec<_> = by_namespace.keys().cloned().collect();
    namespaces.sort_by(|a, b| {
        match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    });

    // Display routes by namespace
    for namespace in namespaces {
        if let Some(ns) = &namespace {
            println!("{}", style(format!("Namespace: {}", ns)).magenta().bold());
        } else {
            println!("{}", style("Root namespace").magenta().bold());
        }

        let mut routes_in_ns = by_namespace.get(&namespace).unwrap().clone();
        routes_in_ns.sort_by(|a, b| a.0.cmp(&b.0));

        for (path, name, _namespace, methods) in routes_in_ns {
            let methods_str = if methods.is_empty() {
                style("ANY".to_string()).dim()
            } else {
                let method_names: Vec<_> = methods.iter()
                    .map(|m| format!("{:?}", m))
                    .collect();
                style(method_names.join(", ")).green()
            };

            let name_str = if let Some(n) = name {
                format!(" ({})", style(&n).yellow())
            } else {
                String::new()
            };

            println!("  {} {} {}", methods_str, style(&path).cyan(), name_str);
        }

        println!();
    }

    Ok(())
}
