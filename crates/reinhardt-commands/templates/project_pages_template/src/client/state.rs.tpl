//! Global client-side state management

use std::cell::RefCell;

thread_local! {
	static APP_STATE: RefCell<AppState> = RefCell::new(AppState::default());
}

#[derive(Debug, Default)]
pub struct AppState {
	// Add your global state fields here
	// Example:
	// pub user: Option<UserInfo>,
	// pub loading: bool,
}

/// Initialize the global app state
pub fn init_app_state() {
	APP_STATE.with(|state| {
		*state.borrow_mut() = AppState::default();
	});
}

/// Access the global app state
pub fn with_app_state<F, R>(f: F) -> R
where
	F: FnOnce(&AppState) -> R,
{
	APP_STATE.with(|state| f(&state.borrow()))
}

/// Mutate the global app state
pub fn with_app_state_mut<F, R>(f: F) -> R
where
	F: FnOnce(&mut AppState) -> R,
{
	APP_STATE.with(|state| f(&mut state.borrow_mut()))
}
