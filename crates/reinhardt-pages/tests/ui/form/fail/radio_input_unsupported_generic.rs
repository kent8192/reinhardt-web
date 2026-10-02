use reinhardt_pages::form;

type CustomValue = String;

fn main() {
	let _form = form! {
		name: UnsupportedGenericRadio,
		fields: {
			answer: ChoiceField<String> { widget: RadioInput }
			token: HiddenField<CustomValue> {
				initial: String::from("token")
			}
		}
	};
}
