//! `HiddenField<>` (empty type arg list) must be rejected with the
//! "expected at least one type argument" diagnostic (per the parser
//! helper `parse_optional_field_type_generics` in
//! reinhardt-manouche/src/parser/form.rs).

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: BadForm,
		action: "/x",
		fields: {
			question_id: HiddenField<> {},
		},
	};
}
