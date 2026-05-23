#[derive(Default)]
struct CardProps {
	item: Item,
	variant: Variant,
	children: Option<reinhardt_pages::component::Page>,
}

struct Item;
struct Variant;
