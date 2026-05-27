use reinhardt_pages::page;
fn render(name: String) {
    let _ = page!(
        | name : String | { div { class : "greeting", h1 { { name } } p { { name } } } }
    )(name);
}
