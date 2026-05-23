#[derive(bon::Builder)]
struct CardProps {
    item: Item,
    #[builder(default)]
    variant: Variant,
    #[builder(default)]
    children: Option<reinhardt_pages::component::Page>,
}
struct Item;
struct Variant;
