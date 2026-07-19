//! Headless UI primitives for common asynchronous screen states.
//!
//! The public API consists of three concrete components:
//!
//! - [`crate::ui::ActionButton`] dispatches an [`Action`](crate::reactive::Action) from a
//!   semantic button and reflects its pending state.
//! - [`crate::ui::ActionResultPanel`] selects idle, pending, success, or error content
//!   from an action.
//! - [`crate::ui::ResourcePanel`] selects loading, empty, success, or error content from
//!   a [`Resource`](crate::reactive::Resource) or composed latest value.
//!
//! Shared slot and state-rendering helpers remain private to this module.

mod action_button;
mod action_result_panel;
mod resource_panel;

pub use action_button::ActionButton;
pub use action_result_panel::ActionResultPanel;
pub use resource_panel::ResourcePanel;

pub(crate) mod state;
