mod feature {
	use reinhardt_pages::{Path, loader};

	#[loader]
	pub async fn jobs_loader(Path(job_id): Path<u64>) -> Result<u64, String> {
		Ok(job_id)
	}
}

fn main() {
	let _future = feature::jobs_loader(reinhardt_pages::Path(9));
	let _id = <feature::jobs_loader::marker as reinhardt_pages::RouteLoader>::ID;
}
