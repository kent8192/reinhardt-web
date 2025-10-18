use reinhardt_macros::action;

struct ViewSet;

impl ViewSet {
    #[action(methods = "POST", detail = true, url_path = "invalid path")]
    async fn invalid_path(&self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
