//! WebSocket re-exports.

#[cfg(feature = "websockets-pages")]
pub use reinhardt_websockets::integration::pages::PagesAuthenticator;

pub use reinhardt_websockets::room::{BroadcastResult, Room, RoomError, RoomManager, RoomResult};

pub use reinhardt_websockets::{
	ConsumerContext, Message, WebSocketConnection, WebSocketConsumer, WebSocketError,
	WebSocketResult,
};

pub use reinhardt_websockets::{
	RouteError, RouteResult, WebSocketRoute, WebSocketRouter, clear_websocket_router,
	get_websocket_router, register_websocket_router, reverse_websocket_url,
};
