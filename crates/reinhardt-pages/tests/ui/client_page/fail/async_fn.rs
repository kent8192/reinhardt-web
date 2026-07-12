use reinhardt_pages::client_page;

#[client_page]
pub async fn home_page() -> reinhardt_pages::Page {
	reinhardt_pages::Page::empty()
}

fn main() {}
