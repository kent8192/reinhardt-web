use reinhardt_macros::get;

#[get("/users")]
async fn list_users() -> Result<(), ()> {
    Ok(())
}

fn main() {}
