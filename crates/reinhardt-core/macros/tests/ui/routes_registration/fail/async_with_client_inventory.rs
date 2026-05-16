// `client_inventory` is not supported on async `#[routes]` because
// `inventory::submit!` requires a sync `const`-constructible factory.
// The macro must reject the combination at compile time rather than
// silently dropping the flag. Refs #4453 (Codex review feedback).

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(client_inventory)]
pub async fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
