// External crates must not match `OwnedBy` exhaustively: every current variant
// is listed, but the missing wildcard arm is still rejected.
use reinhardt_query::types::sequence::OwnedBy;

fn describe(owner: &OwnedBy) -> &'static str {
	match owner {
		OwnedBy::Column { .. } => "column",
		OwnedBy::None => "none",
		OwnedBy::SchemaColumn { .. } => "schema column",
	}
}

fn main() {
	let _ = describe(&OwnedBy::None);
}
