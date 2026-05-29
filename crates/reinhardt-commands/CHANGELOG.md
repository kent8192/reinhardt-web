# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.2...reinhardt-commands@v0.1.3) - 2026-05-29

### Documentation

- align documentation with current APIs
- fix version marker counts

## [0.1.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0...reinhardt-commands@v0.1.1) - 2026-05-24

### Added

- *(commands)* add router dispatch to runserver

### Fixed

- *(commands)* call auto_register_router before runserver starts
- *(runserver)* address CodeRabbit review — 413 on body overflow + real peer addr

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-commands@v0.1.0-rc.30...reinhardt-commands@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-commands` as part of the
reinhardt-web 0.1.0 release. This crate is Reinhardt's Django-style
management command framework: it ships the built-in commands
(`runserver`, `migrate`, `makemigrations`, `collectstatic`,
`createsuperuser`, `startproject`, `startapp`, `check`, `introspect`),
the `TemplateSource` trait that backs scaffolding, the hot-reload
WASM / server rebuild pipelines, and the per-app file templates that
`reinhardt new` emits.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`runserver` with built-in hot reload** — watches the workspace
  via `notify` and runs `WasmRebuildPipeline` + `ServerRebuildPipeline`
  in parallel on file change, restarting the server only after both
  artefacts are produced. Pipeline failures do not terminate the
  watcher: a fresh save retriggers the failed pipeline ([#4128](https://github.com/kent8192/reinhardt-web/issues/4128)).
  `--no-wasm-rebuild` opts out of the in-process WASM rebuild;
  `--no-override-wasm` reuses existing `dist/` artefacts when up to
  date ([#4205](https://github.com/kent8192/reinhardt-web/issues/4205)).
  Project `static/` is auto-mounted at `/static/`.
- **`createsuperuser` powered by `SuperuserCreator`** — works
  against any user type marked `#[user(full = true)]` + `#[model]`
  via the inventory-backed `SuperuserCreator` registry.
  `--noinput` reads the password from
  `REINHARDT_SUPERUSER_PASSWORD`, with the same minimum-length rule
  the interactive prompt enforces and an explicit mutually-exclusive
  check against `--no-password` ([#4233](https://github.com/kent8192/reinhardt-web/issues/4233)).
- **`startproject` / `startapp` with pluggable templates** — the
  `TemplateSource` trait has `Embedded`, `Filesystem`, and `Merged`
  implementations. `--template-dir` (or `REINHARDT_TEMPLATE_DIR`)
  switches templates per invocation. `startapp` appends the new app
  to the `installed_apps!` block automatically, and the
  Rust-2024-style `{name}.rs` module path is used (only `lib.rs` is
  renamed for default locations, not custom targets). Apps and
  projects with the `reinhardt_` prefix are rejected to prevent
  collisions with framework crates.
- **Per-app templates aligned with the [#4476](https://github.com/kent8192/reinhardt-web/discussions/4476) layout** —
  `apps/<app>/server_fn.rs.tpl`, `client.rs.tpl`,
  `client/components.rs.tpl`, `client/pages.rs.tpl`, plus the
  `urls/server_urls.rs.tpl` / `urls/client_urls.rs.tpl` /
  `urls/ws_urls.rs.tpl` triple. The project-level templates wire
  the client through `ClientRouter` and the `bootstrap.rs.tpl`
  entry point.
- **`makemigrations --merge`** — produces merge migrations for
  diverged branches without manual hand-editing. The `migrate`
  command auto-initializes the ORM dispatch and propagates the
  MySQL branch.
- **`introspect` and `check`** — `check` consumes
  `ProjectSettings` (no `env::var` reads) for typed access to
  configuration. `introspect` exposes `InfraSignals` with gRPC,
  storage, mail, session, graphql, admin, and i18n detection so
  CI / agent tooling can answer "which features are wired up".
- **`RunserverHook` for concurrent service startup** — registered
  via inventory; runs in parallel with the HTTP listener and is
  awaited before the listener accepts connections, replacing the
  ad-hoc startup-order coupling some integrations relied on.

### Notable Breaking Changes

- **Per-app handlers move to `apps/<app>/`** ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476))
  — `commands/templates/...` no longer carries handler code;
  existing projects must relocate matching source files.
  `reinhardt new` already emits the new layout.
- **`ProjectSettings` replaces `env::var`** ([#4295](https://github.com/kent8192/reinhardt-web/discussions/4295))
  — commands read configuration through `ProjectSettings` /
  `CommandContext::settings: Arc<dyn HasCommonSettings>` instead
  of touching `std::env`.
- **`runserver --with-pages` rebuilds WASM by default** ([#4205](https://github.com/kent8192/reinhardt-web/issues/4205))
  — the previous "skip if artefacts exist" behaviour is now opt-in
  via `--no-override-wasm`. `--force-wasm` is now redundant and
  emits a deprecation warning.
- **`cargo make watch` and friends removed** — the built-in
  hot-reload supersedes `bacon`-driven watch tasks ([#4128](https://github.com/kent8192/reinhardt-web/issues/4128)).

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for project-wide guidance. Command-specific moves:

- Move per-app `server_fn` and client UI from
  `commands/templates/<app>/` into `apps/<app>/` per [#4476](https://github.com/kent8192/reinhardt-web/discussions/4476).
- Replace `std::env::var("REINHARDT_...")` calls in custom
  commands with reads from `CommandContext::settings`.
- Drop `--force-wasm` from `runserver` invocations and use
  `--no-override-wasm` if you intentionally pre-built WASM.
