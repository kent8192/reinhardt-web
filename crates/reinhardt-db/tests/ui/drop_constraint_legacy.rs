use reinhardt_db::migrations::Operation;

fn main() {
	let _operation = Operation::DropConstraint {
		table: "jobs".to_string(),
		constraint_name: "jobs_status_check".to_string(),
	};
}
