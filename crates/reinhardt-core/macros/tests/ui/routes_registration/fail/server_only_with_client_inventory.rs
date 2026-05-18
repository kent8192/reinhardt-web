// `server_only` and `client_inventory` are mutually exclusive:
// `client_inventory` registers the WASM `ClientRouter` surface that
// `server_only` (which sets `no_client_resolvers + no_ws_resolvers`)
// disables. Combining them must fail at parse time. Refs #4509.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(server_only, client_inventory)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
