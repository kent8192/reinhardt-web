use reinhardt_macros::action;

struct Request;
struct Response;

struct MyViewSet;

impl MyViewSet {
    #[action(methods = "POST", detail = true)]
    async fn activate(&self, _req: Request, pk: i64) -> Result<Response, ()> {
        let _ = pk;
        Ok(Response)
    }
}

fn main() {}
