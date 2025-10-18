use reinhardt_macros::api_view;

struct Request;
struct Response;

// Should default to GET
#[api_view]
async fn handler(_req: Request) -> Result<Response, ()> {
    Ok(Response)
}

fn main() {}
