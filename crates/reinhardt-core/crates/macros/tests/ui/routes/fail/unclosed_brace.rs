use reinhardt_macros::get;

#[get("/users/{id")]
async fn get_user() -> Result<(), ()> {
    Ok(())
}

fn main() {}
