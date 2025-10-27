//! URL configuration for example-rest-api project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt::prelude::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use std::sync::Arc;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
async fn list_users() -> Json<Vec<User>> {
    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
    ];

    Json(users)
}

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder()
        .build();

    // Add API endpoint
    router.add_function_route("/api/users", Method::GET, list_users);

    Arc::new(router)
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub fn url_patterns() -> () {
    ()
}
