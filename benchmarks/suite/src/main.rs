use std::{
	env, fs,
	path::{Path, PathBuf},
	process,
	time::{Duration, Instant},
};

const TARGETS: &[&str] = &["reinhardt", "axum", "actix-web", "loco"];

const CATEGORIES: &[Category] = &[
	Category {
		name: "runtime",
		scenarios: &[
			"hello_world",
			"json_echo",
			"path_params",
			"query_params",
			"middleware_chain",
			"dependency_injection",
			"settings_access",
		],
	},
	Category {
		name: "database",
		scenarios: &[
			"single_select",
			"list_100_rows",
			"insert_one",
			"update_one",
			"transaction",
			"n_plus_one_detection",
		],
	},
	Category {
		name: "compile_time",
		scenarios: &[
			"clean_build_minimal",
			"clean_build_full",
			"incremental_model_change",
			"incremental_route_change",
			"cargo_check",
		],
	},
	Category {
		name: "contract",
		scenarios: &[
			"introspect_small_app",
			"introspect_medium_app",
			"validate_contract",
			"generate_cloud_plan",
			"dry_run_deploy",
		],
	},
	Category {
		name: "admin",
		scenarios: &[
			"list_view_1k",
			"list_view_100k",
			"detail_view",
			"create_form",
			"search_filter",
		],
	},
];

struct Category {
	name: &'static str,
	scenarios: &'static [&'static str],
}

fn main() {
	if let Err(err) = run() {
		eprintln!("benchmark-suite: {err}");
		process::exit(1);
	}
}

fn run() -> Result<(), String> {
	let root = find_suite_root()?;
	match env::args().nth(1).as_deref() {
		None | Some("list") => {
			list_suite();
			Ok(())
		}
		Some("check") => check_suite(&root),
		Some("dry-run") => dry_run(&root),
		Some("measure") => measure_suite(&root),
		Some("-h" | "--help" | "help") => {
			print_help();
			Ok(())
		}
		Some(other) => Err(format!(
			"unknown command `{other}`; use `list`, `check`, `dry-run`, or `measure`"
		)),
	}
}

fn find_suite_root() -> Result<PathBuf, String> {
	let cwd =
		env::current_dir().map_err(|err| format!("failed to read current directory: {err}"))?;
	for candidate in [cwd.clone(), cwd.join("benchmarks")] {
		if candidate.join("suite.toml").is_file() {
			return Ok(candidate);
		}
	}

	Err(format!(
		"could not find suite.toml from `{}` or `{}`",
		cwd.display(),
		cwd.join("benchmarks").display()
	))
}

fn print_help() {
	println!("Usage: benchmark-suite [list|check|dry-run|measure]");
	println!();
	println!("Commands:");
	println!("  list     Print the category and scenario matrix");
	println!("  check    Validate suite.toml and every scenario benchmark.toml");
	println!("  dry-run  Print the declared runner, metric, and target matrix");
	println!("  measure  Measure scenario manifest coverage and validation overhead");
}

fn list_suite() {
	for category in CATEGORIES {
		println!("{}", category.name);
		for scenario in category.scenarios {
			println!("  {scenario}");
		}
	}
}

fn check_suite(root: &Path) -> Result<(), String> {
	let suite = read_file(&root.join("suite.toml"))?;
	for target in TARGETS {
		let table = format!("[targets.{target}]");
		if !suite.contains(&table) {
			return Err(format!("suite.toml is missing `{table}`"));
		}
	}

	let mut checked = 0usize;
	for category in CATEGORIES {
		let category_table = format!("[categories.{}]", category.name);
		if !suite.contains(&category_table) {
			return Err(format!("suite.toml is missing `{category_table}`"));
		}

		for scenario in category.scenarios {
			if !suite.contains(&format!("\"{scenario}\"")) {
				return Err(format!(
					"suite.toml category `{}` is missing scenario `{scenario}`",
					category.name
				));
			}
			check_scenario(root, category.name, scenario)?;
			checked += 1;
		}
	}

	println!(
		"benchmark-suite: checked {checked} scenarios across {} targets",
		TARGETS.len()
	);
	Ok(())
}

fn check_scenario(root: &Path, category: &str, scenario: &str) -> Result<(), String> {
	let path = root.join(category).join(scenario).join("benchmark.toml");
	let manifest = read_file(&path)?;

	for expected in [
		format!("category = \"{category}\""),
		format!("name = \"{scenario}\""),
	] {
		if !manifest.contains(&expected) {
			return Err(format!("{} is missing `{expected}`", path.display()));
		}
	}

	for target in TARGETS {
		if !manifest.contains(&format!("\"{target}\"")) {
			return Err(format!("{} is missing target `{target}`", path.display()));
		}
	}

	Ok(())
}

fn dry_run(root: &Path) -> Result<(), String> {
	for category in CATEGORIES {
		for scenario in category.scenarios {
			let path = root
				.join(category.name)
				.join(scenario)
				.join("benchmark.toml");
			let manifest = read_file(&path)?;
			let runner = extract_value(&manifest, "runner").unwrap_or("unknown");
			let metric = extract_value(&manifest, "metric").unwrap_or("unknown");
			let unit = extract_value(&manifest, "unit").unwrap_or("unknown");
			println!(
				"{}/{}: runner={runner} metric={metric} unit={unit} targets={}",
				category.name,
				scenario,
				TARGETS.join(",")
			);
		}
	}

	Ok(())
}

fn measure_suite(root: &Path) -> Result<(), String> {
	let start = Instant::now();
	let manifests = load_manifests(root)?;
	let load_elapsed = start.elapsed();

	let check_start = Instant::now();
	check_suite(root)?;
	let check_elapsed = check_start.elapsed();

	let manifest_bytes: usize = manifests.iter().map(String::len).sum();
	let scenario_count = scenario_count();
	let target_adapter_count = scenario_count * TARGETS.len();

	println!("benchmark-suite: scenario_count={scenario_count}");
	println!("benchmark-suite: target_count={}", TARGETS.len());
	println!("benchmark-suite: target_adapter_count={target_adapter_count}");
	println!("benchmark-suite: manifest_bytes={manifest_bytes}");
	println!(
		"benchmark-suite: manifest_load_ms={:.3}",
		elapsed_ms(load_elapsed)
	);
	println!(
		"benchmark-suite: validation_ms={:.3}",
		elapsed_ms(check_elapsed)
	);
	Ok(())
}

fn load_manifests(root: &Path) -> Result<Vec<String>, String> {
	let mut manifests = vec![read_file(&root.join("suite.toml"))?];
	for category in CATEGORIES {
		for scenario in category.scenarios {
			manifests.push(read_file(
				&root
					.join(category.name)
					.join(scenario)
					.join("benchmark.toml"),
			)?);
		}
	}
	Ok(manifests)
}

fn scenario_count() -> usize {
	CATEGORIES
		.iter()
		.map(|category| category.scenarios.len())
		.sum()
}

fn elapsed_ms(duration: Duration) -> f64 {
	duration.as_secs_f64() * 1_000.0
}

fn extract_value<'a>(manifest: &'a str, key: &str) -> Option<&'a str> {
	let prefix = format!("{key} = \"");
	manifest.lines().find_map(|line| {
		let trimmed = line.trim();
		let value = trimmed.strip_prefix(&prefix)?;
		value.strip_suffix('"')
	})
}

fn read_file(path: &Path) -> Result<String, String> {
	fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}
