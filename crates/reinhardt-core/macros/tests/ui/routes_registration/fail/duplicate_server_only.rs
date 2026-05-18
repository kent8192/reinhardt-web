// `server_only` specified twice in the same `#[routes(...)]` attribute
// must be rejected at parse time. Refs #4509.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(server_only, server_only)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
