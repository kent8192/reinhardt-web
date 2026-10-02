#![deny(private_interfaces)]

use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

mod endpoint {
	use super::*;

	#[derive(Clone, Serialize, Deserialize)]
	struct PrivateRequest {
		id: u32,
	}

	#[derive(Serialize, Deserialize)]
	struct PrivateResponse {
		id: u32,
	}

	#[server_fn]
	#[allow(private_interfaces)]
	pub async fn accepts_private_types(
		request: PrivateRequest,
	) -> Result<PrivateResponse, reinhardt_pages::server_fn::ServerFnError> {
		Ok(PrivateResponse { id: request.id })
	}
}

fn main() {}
