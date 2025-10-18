use reinhardt_macros::action;

struct ViewSet;

impl ViewSet {
    #[action(methods = "POST", detail = "not_bool")]
    async fn invalid_detail(&self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
