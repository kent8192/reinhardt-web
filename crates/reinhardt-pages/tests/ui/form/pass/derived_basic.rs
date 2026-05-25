//! Derived block basic usage - computed values from form fields

use reinhardt_pages::form;

fn main() {
	// Form with derived computed values
	// Note: Each derived item computes a value from form fields.
	// Currently, derived items cannot depend on other derived items due to
	// type inference limitations with `impl Trait` return types.
	let _tweet_form = form! {
		name: TweetForm,
		action: "/api/tweets",

		fields: {
			content: CharField {
				required,
				bind: true,
			}
		}

		derived: {
			char_count: |form| form.content().get().len(),
		}

	};
}
