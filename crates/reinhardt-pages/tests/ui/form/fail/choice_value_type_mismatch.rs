//! `ChoiceField<i64>` semantics: the choices store becomes
//! `Signal<Vec<(i64, String)>>` (Task 8). This test exists as a contract
//! marker — at HEAD the validator does not yet introspect `choice_value`
//! against `T`, so compile-time enforcement comes from downstream type
//! mismatches when populating the store. This file may produce a
//! complex diagnostic; the .stderr is the snapshot, not the contract.

use reinhardt_pages::form;

struct Choice {
	#[allow(dead_code)]
	name: String,
}

fn main() {
	let _ = form! {
		name: BadForm,
		action: "/x",

		fields: {
			c: ChoiceField<i64> {
				required,
				choices_from: "choices",
				choice_value: "name",
				choice_label: "name",
			},
		},
	};

	let _choice = Choice { name: ::std::string::String::new() };
}
