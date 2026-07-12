# Security Audit for 0.3.0 Stable

This document records the dependency-advisory review for the 0.3.0 stable
release gate.

## Audit Target

- Branch: `origin/develop/0.3.0`
- Commit audited: `6916596167`
- Workspace version: `0.3.0-rc.5`
- Local audit date: 2026-06-27
- Command: `cargo make audit`
- Advisory remediation tracker: [#5492](https://github.com/kent8192/reinhardt-web/issues/5492)

`cargo make audit` failed before project-level advisory ignores were added
because `cargo audit` reported 5 vulnerability findings. The same command now
uses `.cargo/audit.toml`, matches the GitHub Actions security-audit gate, and
passes with the temporary ignores listed below.

The advisory DB did not report any Critical or High CVSS scores for the current
findings. `RUSTSEC-2023-0071` has CVSS 5.9, and the remaining 4 findings do not
publish CVSS scores in the local RustSec advisory data.

## Temporary Vulnerability Ignores

| Advisory | Crate | Version | Patched | Local path | 0.3.0 release decision |
| --- | --- | --- | --- | --- | --- |
| [`RUSTSEC-2025-0009`](https://rustsec.org/advisories/RUSTSEC-2025-0009) | `ring` | `0.16.20` | `>=0.17.12` | `cloud-storage 0.11.1 -> jsonwebtoken 7.2.0` under GCS support | Accepted temporarily. Reinhardt's active JWT dependency is `jsonwebtoken 10.3.0` with `aws_lc_rs`; this older `ring` path is isolated to optional GCS signed-URL support. The advisory depends on overflow-checking builds or very large single-chunk AES operations. Remove by replacing `cloud-storage` or its signing path. |
| [`RUSTSEC-2023-0071`](https://rustsec.org/advisories/RUSTSEC-2023-0071) | `rsa` | `0.9.10` | none | `sqlx 0.8.6 -> sqlx-mysql 0.8.6` under MySQL support | Accepted temporarily. The advisory has no patched `rsa` release. The exposure is in MySQL authentication support through `sqlx-mysql`, not direct Reinhardt RSA private-key handling. Remove when `sqlx`/`sqlx-mysql` removes the vulnerable dependency or when MySQL support can be isolated from the release gate. |
| [`RUSTSEC-2026-0098`](https://rustsec.org/advisories/RUSTSEC-2026-0098) | `rustls-webpki` | `0.101.7` | `>=0.103.12` | AWS SDK rustls 0.21 transport and `rskafka` TLS transport | Accepted temporarily. The issue requires otherwise properly issued certificates with URI name constraints and is reachable after signature verification. Remove when upstream AWS SDK/rskafka transport dependencies move off rustls-webpki 0.101.x. |
| [`RUSTSEC-2026-0099`](https://rustsec.org/advisories/RUSTSEC-2026-0099) | `rustls-webpki` | `0.101.7` | `>=0.103.12` | AWS SDK rustls 0.21 transport and `rskafka` TLS transport | Accepted temporarily. The issue requires certificate misissuance around DNS name constraints and wildcard names. Remove with the same rustls-webpki transport upgrade tracked in #5492. |
| [`RUSTSEC-2026-0104`](https://rustsec.org/advisories/RUSTSEC-2026-0104) | `rustls-webpki` | `0.101.7` | `>=0.103.13` | AWS SDK rustls 0.21 transport and `rskafka` TLS transport | Accepted temporarily. Applications that do not parse CRLs through rustls-webpki are not affected by the CRL parsing panic described by the advisory. Remove with the same rustls-webpki transport upgrade tracked in #5492. |
| [`RUSTSEC-2026-0194`](https://rustsec.org/advisories/RUSTSEC-2026-0194) | `quick-xml` | `0.31.0`, `0.39.4` | `>=0.41.0` | legacy `azure_core 0.21`, Azure SDK 1.0 `typespec` XML support, and `syntect -> plist 1.9` | Accepted temporarily. Reinhardt's direct `quick-xml` dependencies are pinned to `0.41.0`; the remaining vulnerable versions are held by upstream dependency constraints. Remove when the Azure SDK and `plist` release dependency ranges that admit `quick-xml >=0.41.0`, or when the legacy Azure staticfiles backend is removed. |
| [`RUSTSEC-2026-0195`](https://rustsec.org/advisories/RUSTSEC-2026-0195) | `quick-xml` | `0.31.0`, `0.39.4` | `>=0.41.0` | legacy `azure_core 0.21`, Azure SDK 1.0 `typespec` XML support, and `syntect -> plist 1.9` | Accepted temporarily. This shares the same upstream-bound paths as RUSTSEC-2026-0194. Direct Reinhardt dependencies are remediated; remove with the same upstream quick-xml dependency refresh tracked in #5492. |

## Allowed Warnings Reviewed

`cargo audit` also reports 16 allowed warnings. These are informational or
maintenance-status advisories rather than vulnerability findings in the default
audit policy, so they do not block the 0.3.0 stable gate. They remain tracked in
#5492 because several will disappear as part of the same dependency cleanup.

| Kind | Advisory | Crate | Version | Decision |
| --- | --- | --- | --- | --- |
| notice | `RUSTSEC-2026-0174` | `http-types` | `2.12.0` | Retain temporarily through transitive users; review while removing older HTTP client stacks. |
| unmaintained | `RUSTSEC-2021-0141` | `dotenv` | `0.15.0` | Retain temporarily; replacement is release-safe but not required for this gate. |
| unmaintained | `RUSTSEC-2023-0089` | `atomic-polyfill` | `1.0.3` | Retain temporarily through transitive dependencies. |
| unmaintained | `RUSTSEC-2024-0320` | `yaml-rust` | `0.4.5` | Retain temporarily through transitive tooling dependencies. |
| unmaintained | `RUSTSEC-2024-0370` | `proc-macro-error` | `0.4.12`, `1.0.4` | Retain temporarily through proc-macro dependencies; remove with macro dependency refresh. |
| unmaintained | `RUSTSEC-2024-0384` | `instant` | `0.1.13` | Retain temporarily through older transitive dependencies. |
| unmaintained | `RUSTSEC-2024-0436` | `paste` | `1.0.15` | Retain temporarily; no direct runtime security impact identified by cargo-audit. |
| unmaintained | `RUSTSEC-2025-0010` | `ring` | `0.16.20` | Covered by the `ring` vulnerability remediation above. |
| unmaintained | `RUSTSEC-2025-0057` | `fxhash` | `0.2.1` | Retain temporarily through transitive dependencies. |
| unmaintained | `RUSTSEC-2025-0134` | `rustls-pemfile` | `1.0.4`, `2.2.0` | Retain temporarily; remove with TLS transport dependency refresh. |
| unmaintained | `RUSTSEC-2025-0141` | `bincode` | `1.3.3`, `2.0.1` | Retain temporarily through transitive dependencies. |
| unmaintained | `RUSTSEC-2026-0173` | `proc-macro-error2` | `2.0.1` | Retain temporarily through proc-macro dependencies. |
| unsound | `RUSTSEC-2026-0097` | `rand` | `0.7.3` | Retain temporarily through transitive dependencies; no direct use of `rand::rng()` with a custom logger is present in Reinhardt code. |

## Follow-Up Plan

The 0.3.0 stable audit is complete with documented temporary exceptions. The
longer dependency work is intentionally tracked outside this release-readiness
issue in [#5492](https://github.com/kent8192/reinhardt-web/issues/5492):

- replace or isolate `cloud-storage` so `ring 0.16.20` leaves the all-features
  dependency graph;
- upgrade, isolate, or feature-gate the `sqlx-mysql -> rsa 0.9.10` path;
- move AWS/Kafka TLS transports off `rustls-webpki 0.101.7`;
- move upstream-constrained Azure SDK and `plist` paths off `quick-xml <0.41.0`;
- remove `.cargo/audit.toml` entries as each advisory is remediated.
