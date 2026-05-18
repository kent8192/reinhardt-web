// `no_ws_resolvers` specified twice in the same `#[routes(...)]` attribute
// must be rejected at parse time. Refs #4509.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(no_ws_resolvers, no_ws_resolvers)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
