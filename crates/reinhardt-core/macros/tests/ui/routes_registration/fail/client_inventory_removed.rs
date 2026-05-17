// The `#[routes(client_inventory)]` opt-in flag was removed in the
// #4453 opt-out switchover. The macro must reject the legacy flag at
// parse time with a migration message pointing at the new
// `no_client_inventory` opt-out spelling — silently dropping the flag
// would surprise users who expect the old gating to still apply.

use reinhardt_macros::routes;

struct UnifiedRouter;

#[routes(client_inventory)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter
}

fn main() {}
