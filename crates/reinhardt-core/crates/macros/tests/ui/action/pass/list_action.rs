use reinhardt_macros::action;

struct Request;
struct Response;

struct MyViewSet;

impl MyViewSet {
	#[action(methods = "GET", detail = false)]
	async fn list_action(&self, _req: Request) -> Result<Response, ()> {
		Ok(Response)
	}
}

fn main() {}
