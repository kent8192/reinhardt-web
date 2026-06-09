use std::path::PathBuf;

fn main() {
	let paths = std::env::args_os()
		.skip(1)
		.map(PathBuf::from)
		.collect::<Vec<_>>();
	let Some(html) = reinhardt_commands::__hot_reload_test_api::render_static_page_patch(&paths)
	else {
		std::process::exit(2);
	};
	println!("{}", html.len());
}
