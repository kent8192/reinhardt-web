use reinhardt_pages::FormValues;

#[derive(Clone, PartialEq, FormValues)]
enum BadFormValues {
	Choice(String),
}

fn main() {}
