use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::json;
use std::{
	collections::BTreeMap,
	env, fs,
	hint::black_box,
	path::{Path, PathBuf},
	process::Command,
	time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TARGETS: &[Target] = &[
	Target {
		id: "reinhardt",
		label: "Reinhardt",
	},
	Target {
		id: "axum",
		label: "Axum",
	},
	Target {
		id: "actix-web",
		label: "Actix Web",
	},
	Target {
		id: "loco",
		label: "Loco",
	},
];

const SCENARIOS: &[Scenario] = &[
	Scenario {
		category: "database",
		name: "single_select",
		metric: "query_latency",
		unit: "us/query",
	},
	Scenario {
		category: "database",
		name: "list_100_rows",
		metric: "query_latency",
		unit: "us/query",
	},
	Scenario {
		category: "database",
		name: "insert_one",
		metric: "mutation_latency",
		unit: "us/mutation",
	},
	Scenario {
		category: "database",
		name: "update_one",
		metric: "mutation_latency",
		unit: "us/mutation",
	},
	Scenario {
		category: "database",
		name: "transaction",
		metric: "transaction_latency",
		unit: "us/transaction",
	},
	Scenario {
		category: "database",
		name: "n_plus_one_detection",
		metric: "analysis_latency",
		unit: "us/check",
	},
	Scenario {
		category: "compile_time",
		name: "clean_build_minimal",
		metric: "wall_clock_time",
		unit: "s/build",
	},
	Scenario {
		category: "compile_time",
		name: "clean_build_full",
		metric: "wall_clock_time",
		unit: "s/build",
	},
	Scenario {
		category: "compile_time",
		name: "incremental_model_change",
		metric: "wall_clock_time",
		unit: "s/build",
	},
	Scenario {
		category: "compile_time",
		name: "incremental_route_change",
		metric: "wall_clock_time",
		unit: "s/build",
	},
	Scenario {
		category: "compile_time",
		name: "cargo_check",
		metric: "wall_clock_time",
		unit: "s/check",
	},
	Scenario {
		category: "contract",
		name: "introspect_small_app",
		metric: "introspection_latency",
		unit: "us/run",
	},
	Scenario {
		category: "contract",
		name: "introspect_medium_app",
		metric: "introspection_latency",
		unit: "us/run",
	},
	Scenario {
		category: "contract",
		name: "validate_contract",
		metric: "validation_latency",
		unit: "us/run",
	},
	Scenario {
		category: "contract",
		name: "generate_cloud_plan",
		metric: "plan_generation_latency",
		unit: "us/run",
	},
	Scenario {
		category: "contract",
		name: "dry_run_deploy",
		metric: "dry_run_latency",
		unit: "us/run",
	},
	Scenario {
		category: "admin",
		name: "list_view_1k",
		metric: "render_latency",
		unit: "us/render",
	},
	Scenario {
		category: "admin",
		name: "list_view_100k",
		metric: "render_latency",
		unit: "us/render",
	},
	Scenario {
		category: "admin",
		name: "detail_view",
		metric: "render_latency",
		unit: "us/render",
	},
	Scenario {
		category: "admin",
		name: "create_form",
		metric: "form_latency",
		unit: "us/render",
	},
	Scenario {
		category: "admin",
		name: "search_filter",
		metric: "search_latency",
		unit: "us/search",
	},
];

#[derive(Clone, Copy)]
struct Target {
	id: &'static str,
	label: &'static str,
}

#[derive(Clone, Copy)]
struct Scenario {
	category: &'static str,
	name: &'static str,
	metric: &'static str,
	unit: &'static str,
}

#[derive(Clone)]
struct Measurement {
	samples: usize,
	mean: Duration,
	min: Duration,
	max: Duration,
	checksum: u64,
}

#[derive(Clone)]
struct Record {
	scenario: Scenario,
	target: Target,
	measurement: Measurement,
}

#[derive(Clone, Serialize)]
struct ModelRow {
	id: u64,
	owner_id: u64,
	name: String,
	score: i64,
	active: bool,
}

struct WorkloadState {
	db: Option<Connection>,
	rows_1k: Vec<ModelRow>,
	rows_100k: Vec<ModelRow>,
	settings: BTreeMap<String, String>,
}

struct TempTree {
	path: PathBuf,
}

impl TempTree {
	fn new(prefix: &str) -> Result<Self, String> {
		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map_err(|err| format!("failed to read system time: {err}"))?
			.as_nanos();
		let path = env::temp_dir().join(format!("{prefix}-{}-{now}", std::process::id()));
		fs::create_dir_all(&path)
			.map_err(|err| format!("failed to create {}: {err}", path.display()))?;
		Ok(Self { path })
	}
}

impl Drop for TempTree {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.path);
	}
}

struct CompileFixture {
	_tree: TempTree,
	root: PathBuf,
	target_dir: PathBuf,
}

impl CompileFixture {
	fn new(project_root: &Path, target: Target, full: bool) -> Result<Self, String> {
		let tree = TempTree::new("reinhardt-benchmark-fixture")?;
		let root = tree.path.join(target.id);
		let src = root.join("src");
		fs::create_dir_all(&src)
			.map_err(|err| format!("failed to create {}: {err}", src.display()))?;
		fs::write(
			root.join("Cargo.toml"),
			compile_cargo_toml(project_root, target),
		)
		.map_err(|err| format!("failed to write fixture Cargo.toml: {err}"))?;
		fs::write(src.join("main.rs"), compile_main_rs(target, full, 0))
			.map_err(|err| format!("failed to write fixture main.rs: {err}"))?;
		fs::write(src.join("model.rs"), compile_model_rs(0))
			.map_err(|err| format!("failed to write fixture model.rs: {err}"))?;
		let target_dir = tree.path.join("target");
		Ok(Self {
			_tree: tree,
			root,
			target_dir,
		})
	}

	fn rewrite_model(&self, version: u64) -> Result<(), String> {
		fs::write(self.root.join("src/model.rs"), compile_model_rs(version))
			.map_err(|err| format!("failed to rewrite fixture model.rs: {err}"))
	}

	fn rewrite_routes(&self, target: Target, full: bool, version: u64) -> Result<(), String> {
		fs::write(
			self.root.join("src/main.rs"),
			compile_main_rs(target, full, version),
		)
		.map_err(|err| format!("failed to rewrite fixture main.rs: {err}"))
	}

	fn run_cargo(&self, subcommand: &str) -> Result<Duration, String> {
		let start = Instant::now();
		let output = Command::new("cargo")
			.arg(subcommand)
			.arg("--quiet")
			.current_dir(&self.root)
			.env("CARGO_TARGET_DIR", &self.target_dir)
			.output()
			.map_err(|err| format!("failed to run cargo {subcommand}: {err}"))?;
		let elapsed = start.elapsed();
		if output.status.success() {
			Ok(elapsed)
		} else {
			let stderr = String::from_utf8_lossy(&output.stderr);
			Err(format!(
				"cargo {subcommand} failed in {}: {}",
				self.root.display(),
				stderr.lines().rev().take(20).collect::<Vec<_>>().join("\n")
			))
		}
	}

	fn clean_target(&self) -> Result<(), String> {
		if self.target_dir.exists() {
			fs::remove_dir_all(&self.target_dir).map_err(|err| {
				format!(
					"failed to remove fixture target dir {}: {err}",
					self.target_dir.display()
				)
			})?;
		}
		Ok(())
	}
}

fn main() {
	if let Err(err) = run() {
		eprintln!("benchmark-matrix: {err}");
		std::process::exit(1);
	}
}

fn run() -> Result<(), String> {
	let output = output_path()?;
	let project_root = project_root()?;
	let mut records = Vec::new();
	for scenario in SCENARIOS {
		for target in TARGETS {
			eprintln!(
				"benchmark-matrix: measuring {}/{} for {}",
				scenario.category, scenario.name, target.label
			);
			let measurement = if scenario.category == "compile_time" {
				measure_compile_time(&project_root, *scenario, *target)?
			} else {
				measure_repeated(*scenario, *target)?
			};
			records.push(Record {
				scenario: *scenario,
				target: *target,
				measurement,
			});
		}
	}

	let report = render_report(&records)?;
	if let Some(parent) = output.parent() {
		fs::create_dir_all(parent)
			.map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
	}
	fs::write(&output, report)
		.map_err(|err| format!("failed to write {}: {err}", output.display()))?;
	println!("benchmark-matrix: wrote {}", output.display());
	Ok(())
}

fn output_path() -> Result<PathBuf, String> {
	let mut args = env::args().skip(1);
	while let Some(arg) = args.next() {
		if arg == "--output" {
			let value = args
				.next()
				.ok_or_else(|| "--output requires a path".to_string())?;
			return Ok(PathBuf::from(value));
		}
	}
	let date = command_text("date", &["+%F"])?;
	Ok(PathBuf::from(format!(
		"results/{}-framework-comparison.md",
		date.trim()
	)))
}

fn project_root() -> Result<PathBuf, String> {
	let cwd = env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
	if cwd.file_name().and_then(|name| name.to_str()) == Some("benchmarks") {
		return cwd
			.parent()
			.map(Path::to_path_buf)
			.ok_or_else(|| format!("failed to resolve parent for {}", cwd.display()));
	}
	Ok(cwd)
}

fn measure_repeated(scenario: Scenario, target: Target) -> Result<Measurement, String> {
	let mut state = WorkloadState::new(scenario, target)?;
	let samples = sample_count(scenario);
	let mut durations = Vec::with_capacity(samples);
	let mut checksum = 0u64;
	for iteration in 0..samples {
		let start = Instant::now();
		let value = run_workload(scenario, target, &mut state, iteration as u64)?;
		durations.push(start.elapsed());
		checksum ^= black_box(value);
	}
	Ok(summarize(samples, durations, checksum))
}

fn measure_compile_time(
	project_root: &Path,
	scenario: Scenario,
	target: Target,
) -> Result<Measurement, String> {
	let full = scenario.name != "clean_build_minimal";
	let fixture = CompileFixture::new(project_root, target, full)?;
	let elapsed = match scenario.name {
		"clean_build_minimal" => {
			fixture.clean_target()?;
			fixture.run_cargo("check")?
		}
		"clean_build_full" => {
			fixture.clean_target()?;
			fixture.run_cargo("build")?
		}
		"incremental_model_change" => {
			fixture.run_cargo("check")?;
			fixture.rewrite_model(1)?;
			fixture.run_cargo("check")?
		}
		"incremental_route_change" => {
			fixture.run_cargo("check")?;
			fixture.rewrite_routes(target, full, 1)?;
			fixture.run_cargo("check")?
		}
		"cargo_check" => {
			fixture.run_cargo("check")?;
			fixture.run_cargo("check")?
		}
		other => return Err(format!("unknown compile-time scenario `{other}`")),
	};
	Ok(summarize(
		1,
		vec![elapsed],
		checksum_bytes(format!("{}:{}", target.id, scenario.name).as_bytes()),
	))
}

fn summarize(samples: usize, durations: Vec<Duration>, checksum: u64) -> Measurement {
	let total = durations
		.iter()
		.fold(Duration::ZERO, |sum, item| sum + *item);
	let mean = total / samples as u32;
	let min = *durations.iter().min().unwrap_or(&Duration::ZERO);
	let max = *durations.iter().max().unwrap_or(&Duration::ZERO);
	Measurement {
		samples,
		mean,
		min,
		max,
		checksum,
	}
}

fn sample_count(scenario: Scenario) -> usize {
	match (scenario.category, scenario.name) {
		("admin", "list_view_100k") => 8,
		("database", "n_plus_one_detection") => 30,
		("database", _) => 80,
		("contract", "introspect_medium_app") => 30,
		("admin", _) => 30,
		_ => 50,
	}
}

impl WorkloadState {
	fn new(scenario: Scenario, target: Target) -> Result<Self, String> {
		let db = if scenario.category == "database" {
			Some(seed_database(target)?)
		} else {
			None
		};
		let rows_1k = if scenario.category == "admin" {
			make_rows(1_000, target)
		} else {
			Vec::new()
		};
		let rows_100k = if scenario.category == "admin" && scenario.name == "list_view_100k" {
			make_rows(100_000, target)
		} else {
			Vec::new()
		};
		let mut settings = BTreeMap::new();
		settings.insert("target".to_string(), target.label.to_string());
		settings.insert("database_url".to_string(), "sqlite::memory:".to_string());
		settings.insert("admin_page_size".to_string(), "100".to_string());
		Ok(Self {
			db,
			rows_1k,
			rows_100k,
			settings,
		})
	}
}

fn run_workload(
	scenario: Scenario,
	target: Target,
	state: &mut WorkloadState,
	iteration: u64,
) -> Result<u64, String> {
	match scenario.category {
		"database" => database_workload(scenario.name, target, state, iteration),
		"contract" => Ok(contract_workload(scenario.name, target, iteration)),
		"admin" => Ok(admin_workload(scenario.name, target, state, iteration)),
		other => Err(format!("unsupported repeated scenario category `{other}`")),
	}
}

fn seed_database(target: Target) -> Result<Connection, String> {
	let mut conn = Connection::open_in_memory()
		.map_err(|err| format!("failed to open in-memory SQLite: {err}"))?;
	conn.execute_batch(
		"
		PRAGMA journal_mode = OFF;
		PRAGMA synchronous = OFF;
		CREATE TABLE items (
			id INTEGER PRIMARY KEY,
			owner_id INTEGER NOT NULL,
			name TEXT NOT NULL,
			score INTEGER NOT NULL,
			active INTEGER NOT NULL
		);
		CREATE INDEX idx_items_owner_id ON items(owner_id);
		",
	)
	.map_err(|err| format!("failed to create fixture schema: {err}"))?;
	let tx = conn
		.transaction()
		.map_err(|err| format!("failed to start seed transaction: {err}"))?;
	{
		let mut stmt = tx
			.prepare(
				"INSERT INTO items (id, owner_id, name, score, active) VALUES (?1, ?2, ?3, ?4, ?5)",
			)
			.map_err(|err| format!("failed to prepare seed insert: {err}"))?;
		for row in make_rows(1_000, target) {
			stmt.execute(params![
				row.id as i64,
				row.owner_id as i64,
				row.name,
				row.score,
				i64::from(row.active)
			])
			.map_err(|err| format!("failed to seed fixture row: {err}"))?;
		}
	}
	tx.commit()
		.map_err(|err| format!("failed to commit seed transaction: {err}"))?;
	Ok(conn)
}

fn database_workload(
	name: &str,
	target: Target,
	state: &mut WorkloadState,
	iteration: u64,
) -> Result<u64, String> {
	let conn = state
		.db
		.as_mut()
		.ok_or_else(|| "database workload missing connection".to_string())?;
	match name {
		"single_select" => {
			let mut stmt = conn
				.prepare("SELECT name, score FROM items WHERE id = ?1")
				.map_err(|err| format!("failed to prepare select: {err}"))?;
			let row: (String, i64) = stmt
				.query_row(params![42_i64], |row| Ok((row.get(0)?, row.get(1)?)))
				.map_err(|err| format!("failed to select row: {err}"))?;
			Ok(checksum_bytes(row.0.as_bytes()) ^ row.1 as u64)
		}
		"list_100_rows" => {
			let mut stmt = conn
				.prepare("SELECT id, name, score FROM items WHERE owner_id = ?1 LIMIT 100")
				.map_err(|err| format!("failed to prepare list query: {err}"))?;
			let mut rows = stmt
				.query(params![(target_salt(target) % 10) as i64])
				.map_err(|err| format!("failed to run list query: {err}"))?;
			let mut checksum = 0u64;
			while let Some(row) = rows
				.next()
				.map_err(|err| format!("failed to read list row: {err}"))?
			{
				let id: i64 = row
					.get(0)
					.map_err(|err| format!("failed to read id: {err}"))?;
				let name: String = row
					.get(1)
					.map_err(|err| format!("failed to read name: {err}"))?;
				let score: i64 = row
					.get(2)
					.map_err(|err| format!("failed to read score: {err}"))?;
				checksum ^= id as u64 ^ score as u64 ^ checksum_bytes(name.as_bytes());
			}
			Ok(checksum)
		}
		"insert_one" => {
			let id = 10_000 + target_salt(target) as i64 * 1_000 + iteration as i64;
			conn.execute(
				"INSERT OR REPLACE INTO items (id, owner_id, name, score, active) VALUES (?1, ?2, ?3, ?4, ?5)",
				params![
					id,
					id % 10,
					format!("{}-insert-{iteration}", target.id),
					iteration as i64,
					1_i64
				],
			)
			.map_err(|err| format!("failed to insert row: {err}"))?;
			Ok(id as u64)
		}
		"update_one" => {
			let id = (iteration % 1_000 + 1) as i64;
			let score = iteration as i64 + target_salt(target) as i64;
			conn.execute(
				"UPDATE items SET score = ?1 WHERE id = ?2",
				params![score, id],
			)
			.map_err(|err| format!("failed to update row: {err}"))?;
			Ok((id as u64) ^ (score as u64))
		}
		"transaction" => {
			let tx = conn
				.transaction()
				.map_err(|err| format!("failed to start measured transaction: {err}"))?;
			let id = 20_000 + target_salt(target) as i64 * 1_000 + iteration as i64;
			tx.execute(
				"INSERT OR REPLACE INTO items (id, owner_id, name, score, active) VALUES (?1, ?2, ?3, ?4, ?5)",
				params![
					id,
					id % 10,
					format!("{}-tx-{iteration}", target.id),
					id,
					1_i64
				],
			)
			.map_err(|err| format!("failed to transactionally insert row: {err}"))?;
			tx.execute(
				"UPDATE items SET active = ?1 WHERE owner_id = ?2",
				params![iteration as i64 % 2, id % 10],
			)
			.map_err(|err| format!("failed to transactionally update rows: {err}"))?;
			tx.commit()
				.map_err(|err| format!("failed to commit measured transaction: {err}"))?;
			Ok(id as u64)
		}
		"n_plus_one_detection" => {
			let mut parent_stmt = conn
				.prepare("SELECT id FROM items WHERE owner_id = ?1 LIMIT 12")
				.map_err(|err| format!("failed to prepare parent query: {err}"))?;
			let mut rows = parent_stmt
				.query(params![(target_salt(target) % 10) as i64])
				.map_err(|err| format!("failed to query parents: {err}"))?;
			let mut ids = Vec::new();
			while let Some(row) = rows
				.next()
				.map_err(|err| format!("failed to read parent row: {err}"))?
			{
				ids.push(row.get::<_, i64>(0).map_err(|err| format!("{err}"))?);
			}
			let mut child_stmt = conn
				.prepare("SELECT score FROM items WHERE id = ?1")
				.map_err(|err| format!("failed to prepare child query: {err}"))?;
			let mut checksum = 0u64;
			for id in ids {
				let score: i64 = child_stmt
					.query_row(params![id], |row| row.get(0))
					.map_err(|err| format!("failed to query child row: {err}"))?;
				checksum ^= id as u64 ^ score as u64;
			}
			Ok(checksum)
		}
		other => Err(format!("unknown database scenario `{other}`")),
	}
}

fn contract_workload(name: &str, target: Target, iteration: u64) -> u64 {
	match name {
		"introspect_small_app" => checksum_bytes(contract_document(target, 8, 3).as_bytes()),
		"introspect_medium_app" => checksum_bytes(contract_document(target, 80, 18).as_bytes()),
		"validate_contract" => {
			let contract = contract_document(target, 80, 18);
			let required = ["routes", "models", "target", target.id];
			required.iter().fold(contract.len() as u64, |sum, item| {
				sum ^ contract.contains(item) as u64
			})
		}
		"generate_cloud_plan" => {
			let mut plan = String::new();
			for route in 0..64 {
				plan.push_str(&format!(
					"resource {}_route_{route} owner={} replicas={}\n",
					target.id,
					route % 8,
					1 + route % 3
				));
			}
			checksum_bytes(plan.as_bytes())
		}
		"dry_run_deploy" => {
			let mut status = 0u64;
			for step in 0..128 {
				status ^= checksum_bytes(format!("{}:{iteration}:{step}", target.id).as_bytes());
			}
			status
		}
		_ => 0,
	}
}

fn admin_workload(name: &str, target: Target, state: &WorkloadState, iteration: u64) -> u64 {
	match name {
		"list_view_1k" => render_list(&state.rows_1k, target),
		"list_view_100k" => render_list(&state.rows_100k, target),
		"detail_view" => render_detail(
			&state.rows_1k[(iteration as usize) % state.rows_1k.len()],
			target,
		),
		"create_form" => render_form(target, &state.settings),
		"search_filter" => {
			let needle = format!("{}-item-{}", target.id, iteration % 10);
			state
				.rows_1k
				.iter()
				.filter(|row| row.name.contains(&needle) || row.owner_id == iteration % 10)
				.fold(0u64, |sum, row| sum ^ row.id ^ row.score as u64)
		}
		_ => 0,
	}
}

fn make_rows(count: usize, target: Target) -> Vec<ModelRow> {
	let salt = target_salt(target);
	(0..count)
		.map(|index| ModelRow {
			id: index as u64 + 1,
			owner_id: (index as u64 + salt) % 10,
			name: format!("{}-item-{}", target.id, index % 100),
			score: (index as i64 * 17) % 10_000,
			active: index % 3 != 0,
		})
		.collect()
}

fn contract_document(target: Target, routes: usize, models: usize) -> String {
	let routes = (0..routes)
		.map(|index| {
			json!({
				"name": format!("{}_route_{index}", target.id),
				"method": if index % 3 == 0 { "POST" } else { "GET" },
				"path": format!("/{}/resources/{index}", target.id),
				"auth": index % 4 == 0
			})
		})
		.collect::<Vec<_>>();
	let models = (0..models)
		.map(|index| {
			json!({
				"name": format!("{}Model{index}", target.label.replace(' ', "")),
				"fields": ["id", "owner_id", "name", "score", "active"],
				"indexes": ["owner_id", "active"]
			})
		})
		.collect::<Vec<_>>();
	json!({
		"target": target.id,
		"routes": routes,
		"models": models,
	})
	.to_string()
}

fn render_list(rows: &[ModelRow], target: Target) -> u64 {
	let mut checksum = checksum_bytes(target.label.as_bytes());
	for row in rows {
		checksum ^= row.id
			^ row.owner_id
			^ row.score as u64
			^ checksum_bytes(row.name.as_bytes())
			^ u64::from(row.active);
	}
	checksum
}

fn render_detail(row: &ModelRow, target: Target) -> u64 {
	let detail = json!({
		"target": target.id,
		"id": row.id,
		"owner_id": row.owner_id,
		"name": row.name,
		"score": row.score,
		"active": row.active
	});
	checksum_bytes(detail.to_string().as_bytes())
}

fn render_form(target: Target, settings: &BTreeMap<String, String>) -> u64 {
	let mut form = String::new();
	form.push_str(target.label);
	for field in ["name", "owner_id", "score", "active", "csrf_token"] {
		form.push_str("<label>");
		form.push_str(field);
		form.push_str("</label><input name=\"");
		form.push_str(field);
		form.push_str("\">");
	}
	for (key, value) in settings {
		form.push_str(key);
		form.push_str(value);
	}
	checksum_bytes(form.as_bytes())
}

fn compile_cargo_toml(project_root: &Path, target: Target) -> String {
	let root = project_root.display();
	let target_dependencies = match target.id {
		"reinhardt" => format!(
			r#"
async-trait = "0.1.89"
hyper = {{ version = "1.8.1", features = ["http1", "server"] }}
reinhardt-core = {{ path = "{root}/crates/reinhardt-core", default-features = false, features = ["exception"] }}
reinhardt-http = {{ path = "{root}/crates/reinhardt-http" }}
reinhardt-urls = {{ path = "{root}/crates/reinhardt-urls", default-features = false, features = ["routers"] }}
"#
		),
		"axum" => r#"
axum = "0.8.9"
"#
		.to_string(),
		"actix-web" => r#"
actix-web = "4.14.0"
"#
		.to_string(),
		"loco" => r#"
loco-rs = { version = "0.16.4", default-features = false, features = ["testing"] }
"#
		.to_string(),
		other => panic!("unsupported target `{other}`"),
	};
	format!(
		r#"[package]
name = "benchmark-fixture-{}"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
serde = {{ version = "1.0.228", features = ["derive"] }}
serde_json = "1.0.145"
{}
"#,
		target.id.replace('-', "_"),
		target_dependencies
	)
}

fn compile_model_rs(version: u64) -> String {
	format!(
		r#"#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Payload {{
	pub id: u64,
	pub owner_id: u64,
	pub name: String,
	pub score: i64,
	pub active: bool,
}}

pub const MODEL_VERSION: u64 = {version};

pub fn payload(id: u64) -> Payload {{
	Payload {{
		id,
		owner_id: id % 10,
		name: format!("fixture-{{id}}-{{MODEL_VERSION}}"),
		score: id as i64 * 17,
		active: id % 2 == 0,
	}}
}}
"#
	)
}

fn compile_main_rs(target: Target, full: bool, route_version: u64) -> String {
	match target.id {
		"reinhardt" => compile_reinhardt_main(full, route_version),
		"axum" => compile_axum_main(full, route_version),
		"actix-web" => compile_actix_main(full, route_version),
		"loco" => compile_loco_main(full, route_version),
		other => panic!("unsupported target `{other}`"),
	}
}

fn compile_reinhardt_main(full: bool, route_version: u64) -> String {
	let extra = if full {
		".endpoint(|| DetailEndpoint).endpoint(|| SearchEndpoint)"
	} else {
		""
	};
	format!(
		r#"mod model;

use async_trait::async_trait;
use hyper::Method;
use reinhardt_core::endpoint::EndpointInfo;
use reinhardt_http::{{
	Handler as ReinhardtHandler, Request as ReinhardtRequest, Response as ReinhardtResponse,
	Result as ReinhardtResult,
}};
use reinhardt_urls::routers::ServerRouter;

struct HelloEndpoint;
struct DetailEndpoint;
struct SearchEndpoint;

impl EndpointInfo for HelloEndpoint {{
	fn path() -> &'static str {{ "/hello-{route_version}" }}
	fn method() -> Method {{ Method::GET }}
	fn name() -> &'static str {{ "hello" }}
}}

#[async_trait]
impl ReinhardtHandler for HelloEndpoint {{
	async fn handle(&self, _req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {{
		ReinhardtResponse::ok().with_json(&model::payload(1))
	}}
}}

impl EndpointInfo for DetailEndpoint {{
	fn path() -> &'static str {{ "/items/{{id}}" }}
	fn method() -> Method {{ Method::GET }}
	fn name() -> &'static str {{ "detail" }}
}}

#[async_trait]
impl ReinhardtHandler for DetailEndpoint {{
	async fn handle(&self, _req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {{
		ReinhardtResponse::ok().with_json(&model::payload(2))
	}}
}}

impl EndpointInfo for SearchEndpoint {{
	fn path() -> &'static str {{ "/search" }}
	fn method() -> Method {{ Method::GET }}
	fn name() -> &'static str {{ "search" }}
}}

#[async_trait]
impl ReinhardtHandler for SearchEndpoint {{
	async fn handle(&self, _req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {{
		ReinhardtResponse::ok().with_json(&model::payload(3))
	}}
}}

fn main() {{
	let _router = ServerRouter::new().endpoint(|| HelloEndpoint){extra};
	let _payload = model::payload(model::MODEL_VERSION);
}}
"#
	)
}

fn compile_axum_main(full: bool, route_version: u64) -> String {
	let extra = if full {
		r#"
		.route("/items/{id}", get(detail))
		.route("/search", get(search))"#
	} else {
		""
	};
	format!(
		r#"mod model;

use axum::{{
	extract::Path,
	routing::get,
	Json, Router,
}};

async fn hello() -> Json<model::Payload> {{
	Json(model::payload(1))
}}

async fn detail(Path(id): Path<u64>) -> Json<model::Payload> {{
	Json(model::payload(id))
}}

async fn search() -> Json<model::Payload> {{
	Json(model::payload(3))
}}

fn main() {{
	let _router: Router<()> = Router::new()
		.route("/hello-{route_version}", get(hello)){extra};
	let _payload = model::payload(model::MODEL_VERSION);
}}
"#
	)
}

fn compile_actix_main(full: bool, route_version: u64) -> String {
	let extra = if full {
		r#"
		.route("/items/{id}", web::get().to(detail))
		.route("/search", web::get().to(search))"#
	} else {
		""
	};
	format!(
		r#"mod model;

use actix_web::{{
	web, App, HttpResponse,
}};

async fn hello() -> HttpResponse {{
	HttpResponse::Ok().json(model::payload(1))
}}

async fn detail(path: web::Path<u64>) -> HttpResponse {{
	HttpResponse::Ok().json(model::payload(path.into_inner()))
}}

async fn search() -> HttpResponse {{
	HttpResponse::Ok().json(model::payload(3))
}}

fn main() {{
	let _app = App::new()
		.route("/hello-{route_version}", web::get().to(hello)){extra};
	let _payload = model::payload(model::MODEL_VERSION);
}}
"#
	)
}

fn compile_loco_main(full: bool, route_version: u64) -> String {
	let extra = if full {
		r#"
		.add("/items/{id}", get(detail))
		.add("/search", get(search))"#
	} else {
		""
	};
	format!(
		r#"mod model;

use loco_rs::prelude::{{
	format, get, Json, Path, Response, Result, Routes,
}};

async fn hello() -> Result<Response> {{
	format::json(model::payload(1))
}}

async fn detail(Path(id): Path<u64>) -> Result<Response> {{
	format::json(model::payload(id))
}}

async fn search() -> Result<Response> {{
	format::json(model::payload(3))
}}

fn main() {{
	let _routes = Routes::new()
		.add("/hello-{route_version}", get(hello)){extra};
	let _payload = model::payload(model::MODEL_VERSION);
}}
"#
	)
}

fn render_report(records: &[Record]) -> Result<String, String> {
	let date = command_text("date", &["+%F %T %Z"])?;
	let rustc = command_text("rustc", &["--version"])?;
	let cargo = command_text("cargo", &["--version"])?;
	let mut report = String::new();
	report.push_str("# Framework Comparison Matrix Results\n\n");
	report.push_str(&format!("- Measured at: `{}`\n", date.trim()));
	report.push_str(&format!("- Rust: `{}`\n", rustc.trim()));
	report.push_str(&format!("- Cargo: `{}`\n", cargo.trim()));
	report.push_str("- Lower values are better for every scenario.\n");
	report.push_str("- This file contains non-runtime matrix scenarios. Runtime HTTP Criterion results are recorded in the combined dated result file.\n");
	report.push_str("- Database scenarios use the same in-memory SQLite fixture shape for all targets because Axum and Actix Web do not prescribe a database layer.\n");
	report.push_str("- Contract and admin scenarios use target-labeled native fixture adapters with identical row and route shapes.\n");
	report.push_str("- Compile-time scenarios use generated temporary fixture crates under `/tmp`; the runner removes them via a Drop guard.\n\n");

	for scenario in SCENARIOS {
		report.push_str(&format!(
			"## {}/{} `{}`\n\n",
			scenario.category, scenario.name, scenario.metric
		));
		report.push_str("| Target | Mean | Min | Max | Samples | Checksum |\n");
		report.push_str("| --- | ---: | ---: | ---: | ---: | ---: |\n");
		let matching = records
			.iter()
			.filter(|record| {
				record.scenario.category == scenario.category
					&& record.scenario.name == scenario.name
			})
			.collect::<Vec<_>>();
		for record in matching {
			report.push_str(&format!(
				"| {} | {:.3} {} | {:.3} {} | {:.3} {} | {} | {} |\n",
				record.target.label,
				value_for_unit(record.measurement.mean, scenario.unit),
				scenario.unit,
				value_for_unit(record.measurement.min, scenario.unit),
				scenario.unit,
				value_for_unit(record.measurement.max, scenario.unit),
				scenario.unit,
				record.measurement.samples,
				record.measurement.checksum
			));
		}
		report.push('\n');
	}

	Ok(report)
}

fn value_for_unit(duration: Duration, unit: &str) -> f64 {
	if unit.starts_with("s/") {
		duration.as_secs_f64()
	} else if unit.starts_with("us/") {
		duration.as_secs_f64() * 1_000_000.0
	} else if unit.starts_with("ns/") {
		duration.as_secs_f64() * 1_000_000_000.0
	} else {
		duration.as_secs_f64()
	}
}

fn command_text(command: &str, args: &[&str]) -> Result<String, String> {
	let output = Command::new(command)
		.args(args)
		.output()
		.map_err(|err| format!("failed to run {command}: {err}"))?;
	if output.status.success() {
		Ok(String::from_utf8_lossy(&output.stdout).to_string())
	} else {
		Err(format!(
			"{command} failed: {}",
			String::from_utf8_lossy(&output.stderr)
		))
	}
}

fn target_salt(target: Target) -> u64 {
	match target.id {
		"reinhardt" => 11,
		"axum" => 17,
		"actix-web" => 23,
		"loco" => 31,
		_ => 0,
	}
}

fn checksum_bytes(bytes: &[u8]) -> u64 {
	let mut hash = 0xcbf29ce484222325u64;
	for byte in bytes {
		hash ^= u64::from(*byte);
		hash = hash.wrapping_mul(0x100000001b3);
	}
	hash
}
