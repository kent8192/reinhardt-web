use reinhardt_macros::url_patterns;

#[url_patterns]
fn merged() -> UnifiedRouter {
	UnifiedRouter::new().merge(UnifiedRouter::new().server(configure))
}

#[url_patterns]
fn aliased() -> UnifiedRouter {
	UnifiedRouter::new().merge({
		let router = UnifiedRouter::new();
		router.server(configure)
	})
}

#[url_patterns]
fn mounted() -> UnifiedRouter {
	UnifiedRouter::new().mount_unified("/", UnifiedRouter::new().server(configure))
}

#[url_patterns]
fn client_argument() -> UnifiedRouter {
	UnifiedRouter::new().client(|client| {
		consume(UnifiedRouter::new().server(configure));
		client
	})
}

#[url_patterns]
fn typed_local() -> UnifiedRouter {
	UnifiedRouter::new().merge({
		let router: UnifiedRouter = make_router();
		router.server(crate::native::configure)
	})
}

#[url_patterns]
fn typed_closure() -> UnifiedRouter {
	UnifiedRouter::new().merge({
		let apply = |router: UnifiedRouter| router.server(crate::native::configure);
		apply(UnifiedRouter::new())
	})
}

#[url_patterns]
fn helper_receiver() -> UnifiedRouter {
	UnifiedRouter::new().merge(make_router().server(crate::native::configure))
}

#[url_patterns]
fn nested_function() -> UnifiedRouter {
	UnifiedRouter::new().merge({
		fn apply(router: UnifiedRouter) -> UnifiedRouter {
			router.server(crate::native::configure)
		}
		apply(UnifiedRouter::new())
	})
}

#[url_patterns]
fn tuple_assignment() -> UnifiedRouter {
	UnifiedRouter::new().merge({
		let router;
		(router,) = (UnifiedRouter::new(),);
		router.server(crate::native::configure)
	})
}

#[url_patterns]
fn let_chain() -> UnifiedRouter {
	UnifiedRouter::new().merge(
		if let Some(router) = Some(UnifiedRouter::new())
			&& {
				consume(router.server(crate::native::configure));
				true
			} {
			router
		} else {
			UnifiedRouter::new()
		},
	)
}

fn main() {}
