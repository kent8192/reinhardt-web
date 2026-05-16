//! Shared navigation component for {{ project_name }}.

use reinhardt::pages::HtmlElement;

/// Navigation bar shared across all app pages.
///
/// Renders a simple navigation structure. Extend with real navigation links
/// once your app pages are defined.
pub fn with_nav(inner: impl Into<HtmlElement>) -> HtmlElement {
	html! {
		<>
			<nav class="navbar">
				<div class="container">
					<h1>{{ "{{ project_name }}" }}</h1>
					{/* Add navigation links here */}
				</div>
			</nav>
			{inner.into()}
		</>
	}
}
