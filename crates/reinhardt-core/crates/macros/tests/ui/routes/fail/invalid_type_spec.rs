use reinhardt_macros::delete;

#[delete("/posts/{<invalid:post_id>}")]
async fn delete_post() -> Result<(), ()> {
    Ok(())
}

fn main() {}
