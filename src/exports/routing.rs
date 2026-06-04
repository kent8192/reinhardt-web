//! Routing, URL, and client-router re-exports.

#[cfg(all(
	any(feature = "api", feature = "standard", feature = "api-only"),
	native
))]
pub use reinhardt_views::viewsets::{
	Action, ActionType, CreateMixin, DestroyMixin, GenericViewSet, ListMixin, ModelViewSet,
	ReadOnlyModelViewSet, RetrieveMixin, UpdateMixin, ViewSet,
};

#[cfg(native)]
pub use reinhardt_urls::routers::{
	DefaultRouter, PathMatcher, PathPattern, Route, Router, RouterFactory, ServerRouter,
	UrlPatternsRegistration, clear_router, get_router, is_router_registered, register_router,
	register_router_arc,
};

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "client-router"
))]
pub use reinhardt_urls::routers::{
	ClientRouterRegistration, collect_client_router_from_inventory, iter_registered_client_routers,
};

// On wasm with `client-router`, `ServerRouter` is the no-op builder (issue
// #4569). Re-export it at the crate root so `reinhardt::ServerRouter` resolves
// uniformly with native, enabling cross-target route-delegate signatures such
// as `fn(ServerRouter) -> ServerRouter`.
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "client-router"
))]
pub use reinhardt_urls::routers::ServerRouter;

#[cfg(feature = "client-router")]
pub use reinhardt_urls::routers::{
	ClientPathPattern, ClientRoute, ClientRouteMatch, ClientRouter, FromPath, HistoryState,
	MergeError, NavigationType, ParamContext, SingleFromPath, UnifiedRouter,
};

#[cfg(feature = "client-router")]
pub use reinhardt_urls::routers::Path as ClientPath;

pub use reinhardt_urls::routers::ClientUrlResolver;
#[cfg(native)]
pub use reinhardt_urls::routers::resolver::UrlResolver;
#[cfg(native)]
pub use reinhardt_urls::routers::resolver::WebSocketUrlResolver;

#[cfg(native)]
pub use reinhardt_urls::routers::{UrlReverser, include_routes as include, path, re_path, reverse};

// WebSocket types (native only)
#[cfg(all(feature = "websockets-pages", native))]
pub use reinhardt_websockets::integration::pages::PagesAuthenticator;
#[cfg(all(feature = "websockets", native))]
pub use reinhardt_websockets::room::{BroadcastResult, Room, RoomError, RoomManager, RoomResult};
#[cfg(all(feature = "websockets", native))]
pub use reinhardt_websockets::{
	ConsumerContext, Message, WebSocketConnection, WebSocketConsumer, WebSocketError,
	WebSocketResult,
};
#[cfg(all(feature = "websockets", native))]
pub use reinhardt_websockets::{
	RouteError, RouteResult, WebSocketRoute, WebSocketRouter, clear_websocket_router,
	get_websocket_router, register_websocket_router, reverse_websocket_url,
};
