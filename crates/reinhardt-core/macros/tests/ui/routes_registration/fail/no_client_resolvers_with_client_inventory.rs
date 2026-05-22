// `no_client_resolvers` and `client_inventory` are mutually exclusive.
// Refs #4509.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(no_client_resolvers, client_inventory)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
