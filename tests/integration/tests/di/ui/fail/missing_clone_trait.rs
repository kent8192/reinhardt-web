use std::sync::Arc;

/// Service without Clone trait - demonstrates the `Arc<T>` Injectable requirement
struct ServiceWithoutClone {
	value: String,
}

/// This function requires T: Clone, mirroring the `Arc<T>` Injectable impl bound
fn wrap_in_arc<T: Clone>(value: T) -> Arc<T> {
	Arc::new(value)
}

fn main() {
	let service = ServiceWithoutClone {
		value: "test".to_string(),
	};
	// This fails because Arc<T> Injectable impl requires T: Clone
	let _arc = wrap_in_arc(service);
}
