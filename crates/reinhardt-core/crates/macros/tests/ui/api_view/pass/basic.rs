use reinhardt_macros::api_view;

struct Request;
struct Response;

#[api_view(methods = "GET")]
async fn handler(_req: Request) -> Result<Response, ()> {
    Ok(Response)
}

fn main() {}
