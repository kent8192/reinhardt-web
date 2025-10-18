//! Content negotiation for Reinhardt
//!
//! This crate provides DRF-style content negotiation for selecting
//! the appropriate renderer and parser based on Accept headers.

pub mod accept;
pub mod media_type;
pub mod negotiator;

pub use media_type::MediaType;
pub use negotiator::{
    BaseContentNegotiation, BaseNegotiator, ContentNegotiator, NegotiationError, RendererInfo,
};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::media_type::*;
    pub use crate::negotiator::*;
}
