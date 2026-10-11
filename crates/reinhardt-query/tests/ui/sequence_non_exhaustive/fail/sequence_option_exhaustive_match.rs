// External crates must not match `SequenceOption` exhaustively: every current
// variant is listed, but the missing wildcard arm is still rejected.
use reinhardt_query::types::sequence::SequenceOption;

fn describe(option: &SequenceOption) -> &'static str {
	match option {
		SequenceOption::Restart(_) => "restart",
		SequenceOption::IncrementBy(_) => "increment by",
		SequenceOption::MinValue(_) => "minvalue",
		SequenceOption::NoMinValue => "no minvalue",
		SequenceOption::MaxValue(_) => "maxvalue",
		SequenceOption::NoMaxValue => "no maxvalue",
		SequenceOption::Cache(_) => "cache",
		SequenceOption::Cycle => "cycle",
		SequenceOption::NoCycle => "no cycle",
		SequenceOption::OwnedBy(_) => "owned by",
		SequenceOption::AsType(_) => "as type",
		SequenceOption::StartWith(_) => "start with",
	}
}

fn main() {
	let _ = describe(&SequenceOption::Cycle);
}
