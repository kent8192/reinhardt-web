# Design: WASM hot-reload integration in `runserver` (issue #4128)

- **Date**: 2026-05-05
- **Issue**: <https://github.com/kent8192/reinhardt-web/issues/4128>
- **Branch**: `fix/issue-4128-wasm-hot-reload`
- **Related**: #4127 (closed, static-mode counterpart), #4122 (downstream symptom)

## 1. Problem

`cargo run --bin manage -- runserver --with-pages` (without `--noreload`) currently:

1. Watches only `Path::new("src")` and `Cargo.toml` of the *cwd* — wasm-side crates living outside that directory are not observed.
2. On a relevant change, returns from the watcher and re-spawns the server child, but **does not** invoke the WASM build pipeline (`cargo build --target wasm32-unknown-unknown` + `wasm-bindgen`).
3. Does not re-`cargo build` the server binary either, so even server-side `.rs` changes only take effect when the developer restarts `cargo run` manually.
4. Emits no log line summarising what was rebuilt.

Result: developers iterating on WASM-side code see a stale `dist/` after server restart and chase ghost regressions (root cause of #4122).

Code reference: `crates/reinhardt-commands/src/builtin.rs:1959-1986` (`watch_files_async`) and surrounding `is_relevant_change` / restart loop.

## 2. Goals

- Hot-reload covers both server-side and WASM-side Rust changes.
- WASM rebuild does **not** restart the HTTP server (zero downtime for client-only edits).
- Server rebuild (re-`cargo build` + restart) replaces the current "spawn-only" loop so server-side edits actually take effect.
- A failed rebuild **never** terminates the watcher — Django-style "fix-and-save-again" recovery.
- Each rebuild emits a single, greppable log line with duration and outcome.
- Existing `bacon`-based wrapper tasks are removed from templates and examples in the same PR, since the built-in loop supersedes them.

## 3. Non-goals

- TCP socket inheritance (`systemfd`/`listenfd` style) for sub-second server restarts. Future work; out of scope.
- Browser-side live-reload injection (auto page refresh). Out of scope; user reloads manually.
- Source-level `cfg` analysis to determine which target a `.rs` change affects. We deliberately delegate this to cargo's incremental build (see §5).
- Replacing `notify` with another file-watching crate.
- General-purpose `cargo make watch` / `test-watch` / `clippy-watch` replacement. Those bacon tasks go away; users wanting watch-mode quality gates run their own tool.

## 4. Architecture

```
runserver --with-pages (no --noreload)
  │
  │  cargo metadata
  ▼
SourceRoots ─── enumerate cdylib crate + path-dep workspace members
  │
  ▼
notify::Watcher ── debounce 300ms ──▶ Event
                                       │
                                       ▼
                            ┌──────── dispatch ────────┐
                            │ (parallel, independent)  │
                            ▼                          ▼
                  WasmRebuildPipeline       ServerRebuildPipeline
                  cargo build wasm32         cargo build --bin
                  wasm-bindgen → dist/       kill child + spawn new
                            │                          │
                            ▼                          ▼
                       log line                   log line
                  (success or failure)       (success or failure)

Outer loop never exits on pipeline failure. Only SIGINT/SIGTERM stops it.
```

### 4.1 Why parallel pipelines, not path-classified

In a typical Reinhardt project a single crate exports both:

- `[lib] crate-type = ["cdylib", "rlib"]` for the WASM target,
- `[[bin]]` for the server,

with code split by `#[cfg(target_arch = "wasm32")]`. Path-based classification of a file event is therefore unreliable — the same `.rs` file may contain code for both targets.

Instead, every relevant event fires both pipelines. Cargo's incremental build correctly skips work for the target that wasn't actually affected, so the cost of "always run both" is bounded by an incremental no-op (sub-second in practice). This keeps the dispatcher simple and resists future cfg complexity ("Magic must be understandable").

## 5. Components

| Unit | Location | Responsibility | Dependencies |
|------|----------|----------------|--------------|
| `SourceRoots` | `crates/reinhardt-commands/src/runserver_hooks.rs` (new type) | Resolve cdylib crate + transitive path-dep workspace members; produce a `Vec<PathBuf>` of `src/` directories plus `Cargo.toml` paths | `cargo_metadata` crate |
| `WasmRebuildPipeline` | same file | Thin wrapper around existing `build_pages_wasm`; measures duration; formats log line; returns `Result` | `wasm_builder` |
| `ServerRebuildPipeline` | same file | `cargo build --bin <name>` (in dev profile) → on success, kill old child + spawn new; on failure, leave the running child untouched | `tokio::process` |
| `DebouncedWatcher` | replaces current `watch_files_async` in `builtin.rs` | Owns the `notify::RecommendedWatcher`, debounces with `tokio::time::sleep`, dispatches events to both pipelines without awaiting either | `notify`, `tokio` |
| CLI flag `--no-wasm-rebuild` | `crates/reinhardt-commands/src/cli.rs` | Disables the WASM pipeline (server pipeline still runs) | clap |
| Startup banner | runserver entry | One-shot info log enumerating watched roots, pipelines, and resilience policy | tracing |

### 5.1 Data flow

```
file save
  → notify::Event
  → 300ms debounce window collects bursts (IDE save flurries)
  → single coalesced "rebuild request"
  → spawn 2 tokio tasks (wasm_task, server_task) — fire-and-track
  → each task logs its own outcome
  → dispatcher returns to recv loop immediately (does NOT await tasks)
  → next event may arrive while tasks still running; dispatcher coalesces
    by aborting in-flight tasks and starting fresh ones (latest-wins)
```

Latest-wins semantics matter: if the developer saves three times in two seconds, we want to rebuild only the final state, not queue three sequential rebuilds.

"Abort in-flight task" is implemented by killing the spawned `cargo` child process (via the stored `Child::kill` handle), then awaiting its exit before starting the replacement. Dropping the `tokio::task::JoinHandle` alone is insufficient because cargo would continue running detached.

## 6. Resilience requirements

These four invariants are MUST-level. Each has a corresponding integration test (§9.2).

- **OL-1**: The outer `recv` loop never exits because of a build failure. Termination requires SIGINT/SIGTERM or an explicit user quit.
- **OL-2**: A failed pipeline writes one summary line to stderr in the form
  ```
  [hot-reload] WASM rebuild FAILED (took 2.3s):
    cargo build --target wasm32-unknown-unknown exit code: 101
    stderr (last 20 lines):
      <…>
  [hot-reload] watching for next change...
  ```
  Successful runs use the shorter form `[hot-reload] WASM rebuild OK (took 1.2s)`. The `[hot-reload]` prefix is fixed and greppable.
- **OL-3**: On partial failure, the running state is preserved:
  - WASM fails / server succeeds → old `dist/`, new server process.
  - WASM succeeds / server fails → new `dist/`, old server process.
  - Both fail → nothing changes.
  In all cases the next save retriggers both pipelines.
- **OL-4**: An integration test must drive a save → break → save → fix sequence and assert both the failure log line and the recovery success log line.

## 7. CLI / UX

Startup banner (single block, info level):

```
[hot-reload] enabled
  watching: 3 source roots
    - <abs path>/crates/foo/src
    - <abs path>/crates/bar/src
    - <abs path>/Cargo.toml
  pipelines: server rebuild + restart, wasm rebuild
  on failure: keep watching (Ctrl+C to quit)
```

Flags:

- `--no-wasm-rebuild` — disable the WASM pipeline only (server pipeline unaffected).
- `--noreload` — existing flag; disables both pipelines (no behavioural change).

No new debounce flag; 300ms is fixed (sufficient for IDE save bursts; matches cargo-leptos / dx serve precedent).

## 8. Compatibility

- Public API surface added: `--no-wasm-rebuild` flag. Additive; non-breaking.
- Behaviour change for users currently relying on `runserver` without `--noreload`:
  - Before: `.rs` change → server restart with **stale binary** (silent footgun).
  - After: `.rs` change → `cargo build` runs, then restart. First save after this PR may take noticeably longer because of cold cargo build cache; subsequent saves use incremental.
  - This is a correctness improvement, not a regression. Documented in CHANGELOG under `Fixed`.
- Bacon removal (see §10) changes available `cargo make` task names. Documented under `Changed`.

## 9. Testing

### 9.1 Unit

- `SourceRoots::from_metadata` — fixture-driven `cargo metadata` JSON; assert exact set of `src/` paths returned for layouts that include path-deps, dev-deps, registry deps, workspace members.
- `is_relevant_change` — `.rs`/`.toml` accept; `target/`, `.git/`, `~`, `.swp`, `.tmp` reject.
- Debounce coalescing — drive synthetic events with `tokio::time::pause`; assert N events within 300ms produce 1 dispatch.
- Latest-wins semantics — assert in-flight task is aborted when a new event lands.

### 9.2 Integration (`crates/reinhardt-commands/tests/runserver_hot_reload.rs`)

Built on a `reinhardt-test` fixture that spins up a minimal cdylib + bin crate in a `tempdir`, no real DB, no TestContainers.

- **HR-1 (golden path, WASM)**: edit a wasm-side `.rs`; assert `dist/<crate>_bg.wasm` mtime advances within N seconds and the success log line appears.
- **HR-2 (golden path, server)**: edit a server-side `.rs`; assert the spawned server PID changes and the success log line appears.
- **HR-3 (`--no-wasm-rebuild`)**: edit a wasm-side `.rs` with the flag set; assert `dist/` mtime is unchanged and no WASM log line is emitted.
- **HR-4 (resilience — OL-1, OL-2, OL-4)**: edit a `.rs` to introduce a syntax error; assert the failure log line; assert the watcher process is still alive; edit again to fix; assert the recovery success log line.
- **HR-5 (partial failure — OL-3)**: introduce a wasm-only error (e.g. `wasm-bindgen` flag mismatch) leaving server build healthy; assert WASM failure log + server success log; assert server PID changed.
- **HR-6 (issue #4128 reproduction)**: literal walk-through of the issue's reproduction steps. Acts as the verification artefact for the bug report.

All assertions use strict equality / explicit log-line matching per `instructions/TESTING_STANDARDS.md` (no `contains`-style matches without justification).

### 9.3 Manual

Documented in `crates/reinhardt-commands/src/runserver_hooks.rs` rustdoc as a runbook: start `runserver --with-pages`, edit a wasm file, observe `[hot-reload] WASM rebuild OK`, hard-refresh browser.

## 10. Bacon removal

Removed in the same PR. The built-in loop replaces every existing watch task.

| File | Action |
|------|--------|
| `Makefile.toml` (root) | Remove `[tasks.watch]`, `[tasks.test-watch]`, `[tasks.clippy-watch]`, `[tasks.install-bacon]` |
| `.bacon-locations` (root) | Delete |
| `crates/reinhardt-commands/templates/project_pages_template/Makefile.toml.tpl` | Remove `runserver-watch`, `test-watch`, `dev-watch`, `install-bacon`; update help text |
| `crates/reinhardt-commands/templates/project_restful_template/Makefile.toml.tpl` | Same |
| `examples/examples-tutorial-basis/Makefile.toml` | Remove bacon-wrapped tasks |
| `examples/examples-tutorial-rest/Makefile.toml` | Same |
| `examples/examples-twitter/Makefile.toml` | Same |
| `examples/examples-twitter/bacon.toml` | Delete |
| `instructions/MIGRATION_CARGO_WATCH_TO_BACON.md` | Replace with a short note pointing to built-in hot-reload, or delete |
| `README.md` (root) | Update sections that mention bacon |
| `website/content/quickstart/getting-started.md` | Same |
| `crates/reinhardt-commands/src/lib.rs` | Update rustdoc that mentions bacon |

Rationale for removing rather than coexisting:

- Two reload loops layered on top of each other (bacon → cargo run manage → built-in watcher) makes signal handling and stdout interleaving fragile and complicates the integration tests in §9.2.
- The bacon path historically only restarted the server; it never rebuilt WASM. Keeping it would re-introduce the exact footgun this PR fixes.
- `cargo make watch`/`test-watch`/`clippy-watch` are not specific to runserver; users who want them can run `bacon` directly without project-level wrapping.

## 11. Reference implementations consulted

(Knowledge-based, not search-derived; see brainstorming transcript for context.)

- **cargo-leptos** — outer-loop-never-exits pattern; parallel server/wasm tasks; `tracing::error!` on rebuild failures; broadcast channel for build state.
- **Dioxus `dx serve`** — formatted cargo error output on rebuild failure; watcher continues; latest-wins task coalescing.
- **Trunk** — WASM-only, but its "rebuild fails → keep serving the old asset" policy is exactly the §6 OL-3 invariant.
- **Django `runserver`** — `StatReloader` continues across `SyntaxError`; outer loop terminates only on signal. The behavioural model this PR mirrors.
- **`systemfd` / `listenfd`** — TCP socket inheritance for sub-second restart. Noted as future work in §3 non-goals.

## 12. Implementation sequence

1. Add `cargo_metadata` workspace dependency.
2. Implement `SourceRoots` + unit tests.
3. Extract `WasmRebuildPipeline` from existing `build_pages_wasm`; add duration + log line.
4. Implement `ServerRebuildPipeline` (cargo build + child swap).
5. Replace `watch_files_async` with `DebouncedWatcher` (debounce + parallel dispatch + outer-loop-never-exits).
6. Add `--no-wasm-rebuild` CLI flag and startup banner.
7. Write integration test fixture and HR-1..HR-6.
8. Remove bacon scaffolding (§10).
9. Update CHANGELOG with two entries:
   - `fix(commands): rebuild wasm bundle on hot-reload file change`
   - `chore(build): remove bacon-based watch tasks in favour of built-in runserver hot-reload`
10. Update rustdoc in `runserver_hooks.rs`, root README, getting-started.md.

## 13. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| First save after this lands is slow because cargo build cache is cold | Documented in CHANGELOG; subsequent saves are incremental |
| `cargo build` invoked twice (wasm + server) thrashes target dir | Two pipelines write to disjoint target sub-dirs (`target/wasm32-unknown-unknown/` vs `target/debug/`); no contention in practice |
| `notify` on macOS can drop events under heavy save bursts | Debounce window absorbs typical IDE behaviour; persistent issue would warrant `notify-debouncer-full`, but added only if HR-1 flakes |
| User had `bacon.toml` customisations | Documented breaking change in CHANGELOG; users can keep their own `bacon.toml` and run bacon directly outside `cargo make` |
| Test flake from real cargo invocation latency | Integration tests use a minimal one-file crate; cold build budget set generously (e.g. 60s) with strict assertions on the log lines, not wall-clock |

## 14. Open questions

None outstanding from brainstorming. If discovered during implementation, they will be raised as PR-time discussion or follow-up issues.
