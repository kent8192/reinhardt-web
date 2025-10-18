use reinhardt_macros::permission_required;

#[permission_required("my_app.add_blog_post")]
async fn add_post() -> Result<(), ()> {
    Ok(())
}

fn main() {}
