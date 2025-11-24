use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
	let router = UnifiedRouter::new()
		.mount("/polls/", crate::apps::polls::urls::url_patterns());

	Arc::new(router)
}
