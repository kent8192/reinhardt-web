use reinhardt_macros::put;

#[put("/api/users/{<int:user_id>}/profile")]
async fn update_profile() -> Result<(), ()> {
    Ok(())
}

fn main() {}
