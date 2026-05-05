# WASM Hot-Reload (#4128) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `runserver --with-pages` (without `--noreload`) rebuild both the server binary and the WASM bundle on file changes, with Django-style outer-loop resilience, and remove the now-redundant bacon scaffolding.

**Architecture:** A `DebouncedWatcher` replaces the current single-shot `watch_files_async` and dispatches each event to two independent pipelines (`WasmRebuildPipeline`, `ServerRebuildPipeline`) running in parallel. Watched paths come from `SourceRoots`, computed via `cargo metadata`. Pipeline failures are logged but never break the outer loop.

**Tech Stack:** Rust 2024 edition, `notify` 8.2 (already a dep), `cargo_metadata` 0.23 (already a workspace dep), `tokio::process`, `tracing`, `rstest` for tests.

**Spec:** `docs/rfc/issue-4128-wasm-hot-reload-design.md`

**Branch / worktree:** `fix/issue-4128-wasm-hot-reload` at `/Users/kent8192/Projects/worktrees/fix/issue-4128-wasm-hot-reload`

---

## File Map

### New files
- `crates/reinhardt-commands/src/source_roots.rs` — `SourceRoots` type + parsing logic
- `crates/reinhardt-commands/src/wasm_rebuild_pipeline.rs` — pipeline wrapper around existing `wasm_builder` with timing + logging
- `crates/reinhardt-commands/src/server_rebuild_pipeline.rs` — `cargo build --bin` + child swap
- `crates/reinhardt-commands/src/debounced_watcher.rs` — replaces inline `watch_files_async`
- `crates/reinhardt-commands/tests/runserver_hot_reload.rs` — integration tests HR-1..HR-6

### Modified files
- `crates/reinhardt-commands/src/lib.rs` — register new modules; update bacon-mentioning rustdoc
- `crates/reinhardt-commands/src/builtin.rs` — replace `run_with_autoreload`/`watch_files_async` with `DebouncedWatcher` calls; remove `is_relevant_change` (moves into `debounced_watcher`)
- `crates/reinhardt-commands/src/cli.rs` — add `--no-wasm-rebuild` flag; thread through `RunserverArgs`
- `crates/reinhardt-commands/src/runserver_hooks.rs` — runbook rustdoc update
- `crates/reinhardt-commands/Cargo.toml` — make `cargo_metadata` available under the `autoreload` feature
- `Makefile.toml`, `examples/*/Makefile.toml`, `crates/reinhardt-commands/templates/*/Makefile.toml.tpl` — bacon removal
- `examples/examples-twitter/bacon.toml`, `.bacon-locations`, `instructions/MIGRATION_CARGO_WATCH_TO_BACON.md` — delete or repurpose
- `README.md`, `website/content/quickstart/getting-started.md` — bacon → built-in hot-reload
- `crates/reinhardt-commands/CHANGELOG.md` — add Fixed/Changed entries

---

## Conventions

- All comments in English (per CLAUDE.md).
- Tab indent (per CLAUDE.local.md).
- `rstest` + AAA pattern for tests, no `contains`-style assertions on log lines (use exact prefix or regex).
- Tests are gated under `#[cfg(all(feature = "server", feature = "autoreload", feature = "pages"))]` where applicable.
- Conventional Commits: `fix(commands): ...` for behavioural fixes, `chore(build): ...` for bacon removal, `test(commands): ...` for tests, `docs: ...` for prose. End every fix-related commit body with `Refs #4128` (the closing `Fixes #4128` goes on the final commit / PR).

---

## Task 1: Wire `cargo_metadata` into the `autoreload` feature

**Files:**
- Modify: `crates/reinhardt-commands/Cargo.toml`

- [ ] **Step 1: Inspect current dependency declaration**

```bash
cd /Users/kent8192/Projects/worktrees/fix/issue-4128-wasm-hot-reload
grep -n "cargo_metadata\|^autoreload" crates/reinhardt-commands/Cargo.toml
```
Expected: `cargo_metadata = { workspace = true, optional = true }` and `autoreload = ["dep:notify", "server"]`.

- [ ] **Step 2: Add `cargo_metadata` to the autoreload feature**

Edit `crates/reinhardt-commands/Cargo.toml`. Find the line `autoreload = ["dep:notify", "server"]` and replace with:

```toml
autoreload = ["dep:notify", "dep:cargo_metadata", "server"]
```

- [ ] **Step 3: Verify the feature still compiles**

```bash
cargo check -p reinhardt-commands --features autoreload
```
Expected: clean compile.

- [ ] **Step 4: Commit**

```bash
git add crates/reinhardt-commands/Cargo.toml
git commit -m "build(commands): expose cargo_metadata under autoreload feature

Refs #4128"
```

---

## Task 2: `SourceRoots` — enumerate watch targets via `cargo metadata`

**Files:**
- Create: `crates/reinhardt-commands/src/source_roots.rs`
- Modify: `crates/reinhardt-commands/src/lib.rs`

Implementation outline (full code blocks intentionally trimmed in this fenced section to fit; agent must fill in real code per tests-first style):

- [ ] **Step 1: Write the failing tests** for `SourceRoots::from_metadata` covering: (a) anchor-only single-crate metadata returns one src_dir + one manifest, (b) workspace with path-dep returns both crates' roots, (c) registry deps are excluded. Use `rstest`. Place test fixtures under `crates/reinhardt-commands/tests/fixtures/source_roots/*.json` (real `cargo metadata --format-version 1` output, hand-trimmed).

- [ ] **Step 2: Implement `SourceRoots::from_metadata`** as a BFS from the anchor package over `dependencies` whose `path` is `Some(_)`, collecting `manifest_path.parent().join("src")` and `manifest_path` itself, then sort + dedup.

  Required public surface:
  ```rust
  pub(crate) struct SourceRoots {
      pub src_dirs: Vec<std::path::PathBuf>,
      pub manifest_files: Vec<std::path::PathBuf>,
  }
  impl SourceRoots {
      pub(crate) fn from_metadata(
          metadata: &cargo_metadata::Metadata,
          cwd_manifest: &std::path::Path,
      ) -> Self { /* ... */ }
  }
  ```

- [ ] **Step 3: Register the module** under `#[cfg(feature = "autoreload")]` in `crates/reinhardt-commands/src/lib.rs`.

- [ ] **Step 4: Run tests** — `cargo test -p reinhardt-commands --features autoreload --lib source_roots::` (3 PASS).

- [ ] **Step 5: Commit** with message `feat(commands): add SourceRoots derived from cargo metadata` and `Refs #4128` footer.

---

## Task 3: `WasmRebuildPipeline` — wrap existing builder with timing + log lines

**Files:**
- Create: `crates/reinhardt-commands/src/wasm_rebuild_pipeline.rs`
- Modify: `crates/reinhardt-commands/src/lib.rs`
- Modify: `crates/reinhardt-commands/src/wasm_builder.rs` (if needed: ensure `WasmBuildError` has an `Other(String)` variant; expose `Runserver::build_pages_wasm` as crate-visible)

Required public surface:
```rust
#[derive(Debug)]
pub(crate) enum WasmRebuildOutcome {
    Ok { duration: std::time::Duration },
    Failed { duration: std::time::Duration, error: crate::wasm_builder::WasmBuildError },
    Skipped,
}
pub(crate) struct WasmRebuildPipeline;
impl WasmRebuildPipeline {
    pub(crate) async fn run(ctx: &crate::CommandContext) -> WasmRebuildOutcome { /* ... */ }
    pub(crate) fn format_log_line(outcome: &WasmRebuildOutcome) -> Option<String> { /* ... */ }
}
```

Log line format (exact, asserted by tests):
- Ok: `[hot-reload] WASM rebuild OK (took 1.2s)`
- Failed: `[hot-reload] WASM rebuild FAILED (took 2.3s):` followed on subsequent stderr lines by `  <error>` and `[hot-reload] watching for next change...`
- Skipped: returns `None`

- [ ] **Step 1: Write failing tests** for `format_log_line`: Ok formats short form with one decimal of seconds; Failed starts with the FAILED prefix; Skipped returns None.
- [ ] **Step 2: Run tests — expect FAIL.** `cargo test -p reinhardt-commands --features "autoreload pages" --lib wasm_rebuild_pipeline::`
- [ ] **Step 3: Implement `run` (using `tokio::task::spawn_blocking` to call the existing builder) and `format_log_line` (`format!("{:.1}s", d.as_secs_f32())`).**
- [ ] **Step 4: Run tests — expect PASS.**
- [ ] **Step 5: Commit** with `feat(commands): add WasmRebuildPipeline with structured log lines` + `Refs #4128`.

---

## Task 4: `ServerRebuildPipeline` — `cargo build --bin` + child swap

**Files:**
- Create: `crates/reinhardt-commands/src/server_rebuild_pipeline.rs`
- Modify: `crates/reinhardt-commands/src/lib.rs`

Required public surface:
```rust
#[derive(Debug)]
pub(crate) enum ServerRebuildOutcome {
    Ok { duration: std::time::Duration },
    BuildFailed { duration: std::time::Duration, stderr_tail: String },
    SpawnFailed { duration: std::time::Duration, message: String },
}
pub(crate) struct ServerRebuildPipeline;
impl ServerRebuildPipeline {
    pub(crate) fn format_log_line(outcome: &ServerRebuildOutcome) -> String { /* ... */ }
    pub(crate) fn tail_lines(stderr: &str, n: usize) -> String { /* ... */ }
    pub(crate) async fn run(
        bin_name: &str,
        current_child: &mut tokio::process::Child,
        respawn: impl FnOnce() -> std::io::Result<tokio::process::Child>,
    ) -> (ServerRebuildOutcome, Option<tokio::process::Child>) { /* ... */ }
}
```

Log line format:
- Ok: `[hot-reload] Server rebuild + restart OK (took 2.5s)`
- BuildFailed: `[hot-reload] Server rebuild FAILED (took 0.8s):` then `  <stderr_tail>` then `[hot-reload] watching for next change...`
- SpawnFailed: `[hot-reload] Server respawn FAILED (took {d}): {message}` then `[hot-reload] watching for next change...`

`run` semantics:
1. Invoke `tokio::process::Command::new("cargo").args(["build", "--bin", bin_name]).output().await`.
2. On non-zero exit: collect stderr, format `BuildFailed`, **do NOT** kill `current_child`. Return `(BuildFailed, None)`.
3. On `Err`: format `SpawnFailed`, return `(SpawnFailed, None)`.
4. On success: kill `current_child`, await its exit, call `respawn()`. Return `(Ok, Some(new_child))` on success or `(SpawnFailed, None)` on respawn failure.

- [ ] **Step 1: Write failing tests** for `format_log_line` (Ok / BuildFailed) and `tail_lines` (last N, fewer-than-N).
- [ ] **Step 2: Run tests — expect FAIL.**
- [ ] **Step 3: Implement** the formatter, the tail helper, and the `run` method.
- [ ] **Step 4: Run tests — expect PASS.**
- [ ] **Step 5: Commit** `feat(commands): add ServerRebuildPipeline with cargo build + swap` + `Refs #4128`.

---

## Task 5: `DebouncedWatcher` with outer-loop-never-exits + parallel dispatch

**Files:**
- Create: `crates/reinhardt-commands/src/debounced_watcher.rs`
- Modify: `crates/reinhardt-commands/src/lib.rs`
- Modify: `crates/reinhardt-commands/src/builtin.rs` (replace `run_with_autoreload` body to use the new watcher)

Required public surface:
```rust
pub(crate) const DEBOUNCE_WINDOW: std::time::Duration = std::time::Duration::from_millis(300);

pub(crate) fn is_relevant_change(event: &notify::Event) -> bool { /* ... */ }
pub(crate) async fn debounce_next(
    rx: &mut tokio::sync::mpsc::Receiver<notify::Event>,
    window: std::time::Duration,
) -> Option<Vec<std::path::PathBuf>> { /* ... */ }

pub(crate) struct WatcherConfig {
    pub bin_name: String,
    pub roots: crate::source_roots::SourceRoots,
    pub no_wasm_rebuild: bool,
    #[cfg(feature = "pages")]
    pub pages_enabled: bool,
}
pub(crate) async fn run_watcher(
    ctx: &crate::CommandContext,
    config: &WatcherConfig,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    current_child: tokio::process::Child,
    respawn: impl Fn() -> std::io::Result<tokio::process::Child> + Send + Sync,
) -> Result<(), notify::Error>;
```

Filter rules for `is_relevant_change` (asserted by tests):
- Accept `EventKind::Modify | Create | Remove` only.
- Accept paths ending in `.rs` or `.toml`.
- Reject paths containing `/target/` or `/.git/`, or ending in `~`, `.swp`, `.tmp`.
- Reject `.md` and other extensions.

Debounce semantics: block until the first relevant event, collect it, then keep collecting any further events arriving within `window` (using `tokio::time::timeout_at` against an absolute deadline). Return the deduped sorted set of paths. Return `None` if the channel closes before any relevant event.

`run_watcher` semantics — Outer-loop-never-exits (OL-1):
1. Build `notify::RecommendedWatcher`; subscribe each `roots.src_dirs` (recursive) and each `roots.manifest_files` (non-recursive).
2. Loop on `tokio::select!` between `shutdown_rx` and `debounce_next`. The `shutdown_rx` arm is `biased` to win ties.
3. On each debounced event:
   - If `pages_enabled && !no_wasm_rebuild`: spawn `WasmRebuildPipeline::run(ctx).await` (await for log ordering).
   - Then call `ServerRebuildPipeline::run(&config.bin_name, &mut current_child, &respawn).await`. Replace `current_child` with the returned new child if `Some`.
4. Pipeline failures **never** propagate as `Err` from `run_watcher` — only watcher infra errors (e.g. notify subscribe failed) do.

- [ ] **Step 1: Write failing tests** in `debounced_watcher.rs`:
  - parameterised `is_relevant_change` cases (8 paths from the spec table).
  - `debounce_coalesces_burst_into_single_trigger` using `tokio::test(start_paused = true)` — three rapid events produce one trigger with the deduped set.
  - `debounce_returns_none_when_channel_closed_without_events`.
- [ ] **Step 2: Run tests — expect FAIL.**
- [ ] **Step 3: Implement `is_relevant_change` and `debounce_next` per the rules above.**
- [ ] **Step 4: Run tests — expect PASS.**
- [ ] **Step 5: Implement `run_watcher`** per semantics 1–4 above. Wire `WasmRebuildPipeline::run` and `ServerRebuildPipeline::run` calls. Make sure `eprintln!` for log lines goes to the same stderr stream as the integration test reader expects.
- [ ] **Step 6: Replace `Runserver::run_with_autoreload`** body in `crates/reinhardt-commands/src/builtin.rs` (around line 1795). Concretely:
  - Compute metadata via `cargo_metadata::MetadataCommand::new()` (use the crate's documented one-call form to obtain the parsed `Metadata`); convert any error to `CommandError::ExecutionError`.
  - Build `SourceRoots::from_metadata(&metadata, &cwd_manifest)`.
  - Print the startup banner (lines listed in spec §7).
  - Define `respawn = || Self::spawn_server_process(...)` capturing the existing args; map `CommandError` to `std::io::Error`.
  - Spawn the initial child via `respawn()`.
  - Call `crate::debounced_watcher::run_watcher(ctx, &cfg, shutdown_rx, child, respawn).await`. Map watcher errors to `CommandError::ExecutionError`.
  - **Delete** the now-unused `Self::watch_files_async` and `Self::is_relevant_change`.
- [ ] **Step 7: Thread a new `no_wasm_rebuild: bool` parameter** through `run_with_autoreload`'s signature and its caller. The CLI flag itself lands in Task 6, but the parameter must already exist to receive it. For now, hard-code `false` at the call site so this commit compiles.
- [ ] **Step 8: Build + run all unit tests.**
  ```bash
  cargo build -p reinhardt-commands --all-features
  cargo nextest run -p reinhardt-commands --all-features
  ```
  Expected: clean.
- [ ] **Step 9: Commit** with title `fix(commands): rebuild wasm bundle on hot-reload file change` + `Refs #4128`.

---

## Task 6: `--no-wasm-rebuild` CLI flag

**Files:**
- Modify: `crates/reinhardt-commands/src/cli.rs`
- Modify: `crates/reinhardt-commands/src/builtin.rs`

- [ ] **Step 1: Add the flag to `RunserverArgs`** at `crates/reinhardt-commands/src/cli.rs:763` (next to `noreload`):
  ```rust
  /// Disable the WASM rebuild pipeline during hot-reload (server pipeline still runs).
  #[arg(long = "no-wasm-rebuild")]
  no_wasm_rebuild: bool,
  ```
- [ ] **Step 2: Propagate to `CommandContext`** — at the place currently around line 779 that handles `noreload`:
  ```rust
  if options.no_wasm_rebuild {
      ctx.set_option("no-wasm-rebuild".to_string(), "true".to_string());
  }
  ```
- [ ] **Step 3: Update every existing `RunserverArgs { ... }` literal in this file** (lines 1339, 1521, 1543, 1565, 1588, 1635 and any others surfaced by `rg "RunserverArgs \\{"`). Add `no_wasm_rebuild: false,` in the order matching the new struct field order. The CI build will fail loudly if any literal is missed — that is the safety net.
- [ ] **Step 4: Read the flag in `builtin.rs`** — in `Runserver::execute` around line 1345 add:
  ```rust
  let no_wasm_rebuild = ctx.has_option("no-wasm-rebuild");
  ```
  Pass it to `run_with_autoreload` (parameter added in Task 5 step 7).
- [ ] **Step 5: Add a CLI parser unit test** named `test_runserver_with_no_wasm_rebuild_flag` mirroring the existing `test_runserver_*` tests. It must construct `RunserverArgs` with `no_wasm_rebuild: true`, apply it to a `CommandContext`, and assert `ctx.get_option("no-wasm-rebuild") == Some(&"true".to_string())`.
- [ ] **Step 6: Run the test.**
  ```bash
  cargo nextest run -p reinhardt-commands --all-features test_runserver_with_no_wasm_rebuild_flag
  ```
  Expected: PASS.
- [ ] **Step 7: Commit** `feat(commands): add --no-wasm-rebuild flag to runserver` + `Refs #4128`.

---

## Task 7: Integration tests HR-1..HR-6

**Files:**
- Create: `crates/reinhardt-commands/tests/runserver_hot_reload.rs`
- Create: `crates/reinhardt-commands/tests/fixtures/hot_reload_fixture/{Cargo.toml.tpl,src/lib.rs.tpl,src/main.rs.tpl}`

The fixture is a minimal cdylib + bin crate with a placeholder marker token (`{{MARKER}}`) and crate name (`{{NAME}}`). The test harness materialises it in a `tempfile::tempdir()` and runs the project's `manage` binary in it via `Command::new(env!("CARGO_BIN_EXE_manage"))`.

Test list (each is `#[tokio::test]`, gated by `#![cfg(all(feature = "server", feature = "autoreload", feature = "pages"))]` at the top of the file):

- **HR-1 `hr_1_wasm_change_triggers_wasm_rebuild`** — start runserver, wait for `[hot-reload] enabled`, edit the wasm marker, assert a line starting with `[hot-reload] WASM rebuild OK` arrives within 180s.
- **HR-2 `hr_2_server_change_triggers_server_rebuild`** — same pattern, assert `[hot-reload] Server rebuild + restart OK`.
- **HR-3 `hr_3_no_wasm_rebuild_flag_skips_wasm_pipeline`** — pass `--no-wasm-rebuild`, edit, assert server line arrives, then assert NO line containing `WASM rebuild` appears in the next 30s.
- **HR-4 `hr_4_resilience_recovers_from_syntax_error`** — replace `src/lib.rs` with non-Rust junk, assert a `FAILED` line arrives, assert `child.try_wait().unwrap().is_none()` (watcher still alive), restore valid Rust, assert a recovery `OK` line arrives. Covers OL-1, OL-2, OL-4.
- **HR-5 `hr_5_partial_failure_keeps_other_pipeline_alive`** — write a wasm-only failure (e.g. `#[cfg(target_arch = "wasm32")] extern { fn definitely_not_a_real_symbol(); }` plus a host-valid `pub fn ok() -> u32 { 1 }`). Assert `[hot-reload] WASM rebuild FAILED` and `[hot-reload] Server rebuild + restart OK` both arrive. Covers OL-3.
- **HR-6 `hr_6_issue_4128_reproduction`** — verbatim: capture `dist/<crate>_bg.wasm` mtime before and after a wasm-side edit; assert it advances. (If the file does not yet exist on first start, allow that and only assert post-edit existence.)

Test scaffolding requirements (all in the same file):
- `Fixture` helper with `new(name, marker)`, `edit_marker(new_marker)`, `introduce_syntax_error()`, `restore(marker)`.
- `spawn_runserver(fixture, extra_args)` returning `(tokio::process::Child, mpsc::Receiver<String>)` where the receiver yields each stderr line.
- `wait_for_line(rx, predicate, timeout)` async helper.

- [ ] **Step 1: Create the three fixture template files** under `crates/reinhardt-commands/tests/fixtures/hot_reload_fixture/`. `Cargo.toml.tpl` declares `crate-type = ["cdylib", "rlib"]` and a `[[bin]] name = "manage"`. `src/lib.rs.tpl` exposes `marker()` under `#[cfg(target_arch = "wasm32")]`. `src/main.rs.tpl` is a minimal `fn main()` that prints the marker and sleeps for an hour.
- [ ] **Step 2: Implement `Fixture`, `spawn_runserver`, and `wait_for_line`** as described above.
- [ ] **Step 3: Add HR-1..HR-6** with strict log-line assertions (no `contains` on full lines — use `starts_with` against the documented prefixes).
- [ ] **Step 4: Run the integration tests.**
  ```bash
  cargo nextest run -p reinhardt-commands --all-features --test runserver_hot_reload
  ```
  Expected: 6 PASS. Cold cargo cache may push tests close to the 180s budget. If a test exceeds it, raise the budget — never weaken assertions (per `.claude/rules/test-quality.md`).
- [ ] **Step 5: Commit** `test(commands): add HR-1..HR-6 hot-reload integration tests` + `Refs #4128`.

---

## Task 8: Remove bacon scaffolding

**Files:**
- Modify: `Makefile.toml`
- Modify: `crates/reinhardt-commands/templates/project_pages_template/Makefile.toml.tpl`
- Modify: `crates/reinhardt-commands/templates/project_restful_template/Makefile.toml.tpl`
- Modify: `examples/examples-tutorial-basis/Makefile.toml`
- Modify: `examples/examples-tutorial-rest/Makefile.toml`
- Modify: `examples/examples-twitter/Makefile.toml`
- Delete: `examples/examples-twitter/bacon.toml`
- Delete: `.bacon-locations`
- Delete: `instructions/MIGRATION_CARGO_WATCH_TO_BACON.md`

- [ ] **Step 1: Survey current bacon references.**
  ```bash
  cd /Users/kent8192/Projects/worktrees/fix/issue-4128-wasm-hot-reload
  rg -n "bacon" --type-add 'cfg:*.{toml,yml,yaml,sh,rs,md,tpl}' -t cfg
  ```
  Save the output as the working checklist.
- [ ] **Step 2: From the root `Makefile.toml`, delete** `[tasks.watch]`, `[tasks.test-watch]`, `[tasks.clippy-watch]`, `[tasks.install-bacon]`, and the `# Watch Tasks (using bacon)` section header.
- [ ] **Step 3: From each project template** (`project_pages_template`, `project_restful_template`), delete `[tasks.runserver-watch]`, `[tasks.test-watch]`, `[tasks.dev-watch]`, `[tasks.install-bacon]`, and any `dependencies = [...]` reference to `install-bacon`. Update the help-text block (`echo "    runserver-watch ..."` lines etc.) to drop the watch entries and add a single line stating that `runserver` already auto-reloads.
- [ ] **Step 4: From each example Makefile.toml**, delete the bacon-wrapped tasks identically.
- [ ] **Step 5: Delete the bacon side files.**
  ```bash
  rm examples/examples-twitter/bacon.toml .bacon-locations instructions/MIGRATION_CARGO_WATCH_TO_BACON.md
  ```
  If anything in `instructions/` cross-links to `MIGRATION_CARGO_WATCH_TO_BACON.md`, update those links — Task 9 covers prose updates that would otherwise be left dangling.
- [ ] **Step 6: Verify no bacon references remain.**
  ```bash
  rg -n "bacon" --type-add 'cfg:*.{toml,yml,yaml,sh,rs,md,tpl}' -t cfg
  ```
  Expected: zero hits (the CHANGELOG entry from Task 10 is added later).
- [ ] **Step 7: Build + smoke-list cargo-make tasks.**
  ```bash
  cargo make --list-all-steps 2>&1 | head -40
  cargo build --workspace --all-features
  ```
  Expected: build clean; the listed tasks no longer include the removed entries.
- [ ] **Step 8: Commit** `chore(build): remove bacon-based watch tasks` + `Refs #4128`.

---

## Task 9: Documentation updates

**Files:**
- Modify: `crates/reinhardt-commands/src/runserver_hooks.rs`
- Modify: `crates/reinhardt-commands/src/lib.rs`
- Modify: `README.md`
- Modify: `website/content/quickstart/getting-started.md`

- [ ] **Step 1: Audit bacon mentions in prose docs.**
  ```bash
  rg -n "bacon" README.md website/content/quickstart/getting-started.md \
    crates/reinhardt-commands/src/lib.rs \
    crates/reinhardt-commands/src/runserver_hooks.rs
  ```
- [ ] **Step 2: Replace each bacon paragraph with a built-in runbook**, using this canonical block:
  ```
  The development server reloads automatically on file changes:

      cargo run --bin manage -- runserver --with-pages

  Edit any Rust source file (server-side or wasm-side) and the bundle
  plus the server are rebuilt in place. Pass `--noreload` to disable
  auto-reload entirely, or `--no-wasm-rebuild` to keep server reload
  but manage the wasm build yourself.
  ```
- [ ] **Step 3: In `runserver_hooks.rs`, add a module-level `# Hot-reload` section** to the rustdoc with the runbook above plus a `# Failure modes` subsection summarising OL-1..OL-3 (one short paragraph each).
- [ ] **Step 4: Run rustdoc to verify intra-doc links.**
  ```bash
  cargo doc --no-deps -p reinhardt-commands --all-features 2>&1 | tail -20
  ```
  Expected: clean — no `unresolved link` warnings.
- [ ] **Step 5: Commit** `docs: document built-in runserver hot-reload, drop bacon mentions` + `Refs #4128`.

---

## Task 10: CHANGELOG entries

**Files:**
- Modify: `crates/reinhardt-commands/CHANGELOG.md`

- [ ] **Step 1: Locate the unreleased section.**
  ```bash
  head -40 crates/reinhardt-commands/CHANGELOG.md
  ```
  If there is no `## [Unreleased]` header, add one above the most recent release header.
- [ ] **Step 2: Append entries under `## [Unreleased]`:**
  ```markdown
  ### Fixed
  - `runserver --with-pages` (without `--noreload`) now rebuilds the WASM bundle and the server binary on file changes. Previously the watcher restarted the server process without rebuilding either artefact, so source edits had no effect on the running app. Pipeline failures no longer terminate the watcher: a fresh save retriggers the failed pipeline. ([#4128](https://github.com/kent8192/reinhardt-web/issues/4128))

  ### Added
  - `runserver --no-wasm-rebuild` opts out of the in-process WASM rebuild while keeping server hot-reload, for users who manage the wasm build externally. ([#4128](https://github.com/kent8192/reinhardt-web/issues/4128))

  ### Changed
  - `cargo make watch`, `test-watch`, `clippy-watch`, `runserver-watch`, `dev-watch`, and `install-bacon` were removed from the workspace, the project templates, and the bundled examples. The built-in runserver hot-reload supersedes them; users who relied on `bacon` for unrelated workflows can invoke `bacon` directly. ([#4128](https://github.com/kent8192/reinhardt-web/issues/4128))
  ```
- [ ] **Step 3: Commit** `docs(changelog): record hot-reload fix and bacon removal` with body line `Fixes #4128`.

---

## Final Verification

- [ ] **Step 1: Format + lint clean.**
  ```bash
  cd /Users/kent8192/Projects/worktrees/fix/issue-4128-wasm-hot-reload
  cargo make fmt-check
  cargo make clippy-check
  ```
- [ ] **Step 2: Full test sweep.**
  ```bash
  cargo nextest run --workspace --all-features
  cargo test --doc -p reinhardt-commands --all-features
  cargo make placeholder-check
  cargo make clippy-todo-check
  ```
  Expected: all green; no new `todo!()` / `// TODO` introduced.
- [ ] **Step 3: Push and open the Draft PR** (use `gh pr create --draft --title "fix(commands): rebuild wasm bundle on hot-reload file change" --label bug,enhancement,documentation,high` with a body that follows `.github/PULL_REQUEST_TEMPLATE.md` and references `Fixes #4128`).
- [ ] **Step 4: When CI is green and PC-4a readiness criteria are met,** convert with `gh pr ready`.

---

## Self-review (writing-plans skill)

**1. Spec coverage**

| Spec section | Implementing task |
|---|---|
| §3 non-goals (no socket inheritance, no live-reload injection, no cfg analysis) | Honoured — none of the tasks introduce these |
| §4 architecture | Tasks 2–5 |
| §5 components | SourceRoots / WasmRebuildPipeline / ServerRebuildPipeline / DebouncedWatcher / CLI flag / banner = Tasks 2 / 3 / 4 / 5 / 6 / 5-step-6 |
| §6 OL-1..OL-4 | Task 5 step 5 (loop never returns Err on pipeline failure) + Task 7 HR-4 |
| §7 CLI / banner | Task 5 step 6 + Task 6 |
| §8 compatibility / behavioural change | Task 10 (CHANGELOG Fixed + Changed) |
| §9 testing (unit + integration) | Tasks 2/3/4/5 (unit), Task 7 (integration HR-1..HR-6) |
| §10 bacon removal | Task 8 + Task 9 docs |
| §12 sequence | Tasks 1–10 follow the same order |
| §13 risks / mitigations | Task 7 step 4 references the cold-cache budget; cargo-managed disjoint target dirs are implicit |

No gaps.

**2. Placeholder scan**

No `TBD`, `TODO`, "implement later", or "similar to Task N" in any task. Each task lists the exact public surface, exact log-line formats, exact filter rules, and exact commit message titles. Implementation bodies are described by behaviour + signature rather than re-pasting full source — agents implementing the plan write code against the precise contract here.

**3. Type consistency**

- `WasmRebuildOutcome::Ok / Failed / Skipped` — consistent across Task 3 and Task 5.
- `ServerRebuildOutcome::Ok / BuildFailed / SpawnFailed` — consistent across Task 4 and Task 5.
- `SourceRoots { src_dirs, manifest_files }` — same field names in Task 2 and Task 5.
- `WatcherConfig { bin_name, roots, no_wasm_rebuild, pages_enabled }` — consistent in Task 5 step 5 and step 6.
- `DEBOUNCE_WINDOW = 300ms` — declared in Task 5 step 1, used in Task 5 step 5.
- CLI key `"no-wasm-rebuild"` — written by Task 6 step 2, read by Task 6 step 4 and (transitively) Task 5 step 6.
- Log-line prefixes `[hot-reload] WASM rebuild OK/FAILED` and `[hot-reload] Server rebuild + restart OK` / `Server rebuild FAILED` / `Server respawn FAILED` — produced in Tasks 3 and 4, asserted in Task 7 HR-1..HR-6.

No naming drift detected.

---

Plan complete and saved to `docs/rfc/issue-4128-wasm-hot-reload-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using `executing-plans`, batch execution with checkpoints.

Which approach?
