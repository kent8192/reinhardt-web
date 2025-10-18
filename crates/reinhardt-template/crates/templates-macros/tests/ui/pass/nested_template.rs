use reinhardt_templates_macros::template;

fn main() {
    let _ = template!("emails/welcome.html");
    let _ = template!("blog/post_detail.html");
    let _ = template!("admin/user-list.html");
    let _ = template!("apps/blog/templates/post.html");
}
