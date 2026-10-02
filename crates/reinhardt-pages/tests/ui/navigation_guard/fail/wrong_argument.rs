// Rejected macro items leave their supporting imports unused.
#![allow(unused_imports)]

use reinhardt_pages::{NavigationDecision, NavigationGuardError, navigation_guard};

#[navigation_guard]
async fn require_authenticated(_context: String) -> Result<NavigationDecision, NavigationGuardError> {
	Ok(NavigationDecision::Allow)
}

fn main() {}
