use std::time::{Duration, Instant};

use reinhardt_commands::template_manifest::collect_template_source;
use reinhardt_pages::hmr::SourceId;

fn percentile(samples: &[Duration], rank: f64) -> Duration {
	let index = ((samples.len().saturating_sub(1) as f64) * rank).ceil() as usize;
	samples[index]
}

fn main() {
	let mut args = std::env::args_os().skip(1);
	let Some(path) = args.next().map(std::path::PathBuf::from) else {
		eprintln!("usage: page_hot_patch_bench <rust-source-path> [--iterations N]");
		std::process::exit(2);
	};
	let mut iterations = 30usize;
	while let Some(argument) = args.next() {
		if argument == "--iterations" {
			iterations = args
				.next()
				.and_then(|value| value.to_str()?.parse().ok())
				.unwrap_or(iterations);
		}
	}
	if iterations == 0 {
		eprintln!("iterations must be greater than zero");
		std::process::exit(2);
	}

	let mut samples = Vec::with_capacity(iterations);
	let source = std::fs::read_to_string(&path).unwrap_or_else(|error| {
		eprintln!("failed to read {}: {error}", path.display());
		std::process::exit(1);
	});
	let source_id = SourceId(path.to_string_lossy().replace('\\', "/"));
	for _ in 0..iterations {
		let started = Instant::now();
		let parsed = collect_template_source(&source_id, &source).unwrap_or_else(|error| {
			eprintln!("source is not a valid page! template: {error}");
			std::process::exit(1);
		});
		let descriptors = parsed
			.templates
			.iter()
			.map(|template| &template.descriptor)
			.collect::<Vec<_>>();
		let payload_size = serde_json::to_vec(&descriptors)
			.expect("template descriptors should be serializable")
			.len();
		std::hint::black_box((parsed.templates.len(), payload_size));
		samples.push(started.elapsed());
	}
	samples.sort_unstable();
	let p50 = percentile(&samples, 0.50).as_secs_f64() * 1_000.0;
	let p95 = percentile(&samples, 0.95).as_secs_f64() * 1_000.0;
	println!("iterations={iterations} p50_ms={p50:.3} p95_ms={p95:.3}");
}
