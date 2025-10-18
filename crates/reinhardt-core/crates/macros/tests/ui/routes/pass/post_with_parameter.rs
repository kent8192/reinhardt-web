use reinhardt_macros::post;

#[post("/users/{id}/activate")]
async fn activate_user() -> Result<(), ()> {
    Ok(())
}

fn main() {}
