// `client_inventory` specified twice in the same `#[routes(...)]`
// attribute must be rejected at parse time. Refs #4453.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(client_inventory, client_inventory)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
