// `standalone` specified twice in the same `#[routes(...)]` attribute
// must be rejected at parse time. Refs #4453.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(standalone, standalone)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
