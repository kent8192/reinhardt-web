//! Page components for the Tier 4 fixture.
//!
//! All four pages share `layout_shell`, a persistent `<aside>` sidebar
//! plus per-route content swap inside `<main>`, mirroring Tier 3 so the
//! link-interceptor parent-walk path is exercised the same way. Only
//! the `route.name()` lookup behaviour distinguishes Tier 4 from
//! Tier 3.
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use super::router::with_router;
fn nav_link(href: &'static str, label: &'static str, current: &str) -> PageElement {
    let class = if current == href { "active" } else { "" };
    PageElement::new("a").attr("href", href).attr("class", class).child(label)
}
fn layout_shell(content_id: &'static str, content_label: &'static str) -> Page {
    let current = with_router(|r| r.current_path().get());
    PageElement::new("div")
        .attr("id", "shell")
        .child(
            PageElement::new("aside")
                .attr("id", "sidebar")
                .child(
                    PageElement::new("ul")
                        .child(
                            PageElement::new("li").child(nav_link("/", "Home", &current)),
                        )
                        .child(
                            PageElement::new("li")
                                .child(nav_link("/clusters", "Clusters", &current)),
                        )
                        .child(
                            PageElement::new("li")
                                .child(nav_link("/deployments", "Deployments", &current)),
                        )
                        .child(
                            PageElement::new("li")
                                .child(nav_link("/login", "Login", &current)),
                        ),
                ),
        )
        .child(
            PageElement::new("main")
                .attr("id", "content")
                .child(
                    PageElement::new("section")
                        .attr("id", content_id)
                        .child(PageElement::new("h1").child(content_label)),
                ),
        )
        .into_page()
}
pub fn home_page() -> Page {
    layout_shell("route-home", "HOME VIEW")
}
pub fn clusters_page() -> Page {
    layout_shell("route-clusters", "CLUSTERS VIEW")
}
pub fn deployments_page() -> Page {
    layout_shell("route-deployments", "DEPLOYMENTS VIEW")
}
pub fn login_page() -> Page {
    layout_shell("route-login", "LOGIN VIEW")
}
