// Rejected macro items leave their supporting imports unused.
#![allow(unused_imports)]

use reinhardt_pages::{CancellationToken, loader};

#[loader]
async fn duplicate(
	CancellationToken(first): CancellationToken,
	CancellationToken(second): CancellationToken,
) -> Result<(), String> {
	let _ = (first, second);
	Ok(())
}

fn main() {}
