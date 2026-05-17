// `no_client_inventory` specified twice in the same `#[routes(...)]`
// attribute must be rejected at parse time. Mirrors the existing
// `duplicate_standalone` test for the opt-out flags introduced in
// the #4453 switchover.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(no_client_inventory, no_client_inventory)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
