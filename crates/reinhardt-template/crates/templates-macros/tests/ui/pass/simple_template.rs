use reinhardt_templates_macros::template;

fn main() {
    let _ = template!("base.html");
    let _ = template!("index.html");
    let _ = template!("welcome.txt");
    let _ = template!("readme.md");
}
