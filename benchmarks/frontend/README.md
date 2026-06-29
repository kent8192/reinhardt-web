# Frontend Framework Benchmarks

This suite compares `reinhardt-pages`, Vite React, Vite Vue, Next.js, and Nuxt
on the same Core UI fixture.

## Commands

Prerequisites: Node.js, Chromium installed by Playwright, and `wasm-pack` for
the Reinhardt Pages WASM fixture.

```bash
cargo make frontend-benchmark-check
cargo make frontend-benchmark-runtime
cargo make frontend-benchmark-build
cargo make frontend-benchmark-measure
```

## Methodology

Runtime measurements use Playwright with Chromium and fresh browser contexts.
Production runtime uses production build artifacts only. Build, bundle, and
development-loop metrics are reported separately from browser runtime metrics.

## Metrics

- `boot_ready_ms`
- `hydration_ready_ms`
- `click_update_ms`
- `input_update_ms`
- `list_update_ms`
- `navigation_ms`
- `prod_build_ms`
- `prod_start_ms`
- `bundle_bytes`
- `bundle_gzip_bytes`
- `bundle_brotli_bytes`
- `dev_start_ms`
- `hmr_update_ms`

## Fairness Notes

Vite React and Vite Vue are CSR targets. Next.js and Nuxt are SSR plus
hydration targets. Reinhardt Pages is measured on the recommended WASM path.
The report avoids an overall ranking and ranks only within individual metric
tables.
