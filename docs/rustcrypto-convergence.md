# RustCrypto duplicate allowance retirement

[Issue #6840](https://github.com/kent8192/reinhardt-web/issues/6840) tracks
removing the 15 RustCrypto duplicate-version allowances in
[`deny.toml`](../deny.toml). They were introduced by
[PR #6841](https://github.com/kent8192/reinhardt-web/pull/6841) to preserve AWS
S3 SigV4a and authentication functionality while their dependencies require
incompatible API families. The retirement condition has not been met.

## Verified dependency graph

On 2026-10-24, fresh resolution of `develop/0.4.0` at
`d9c88baea811f6e1fce669ade2f3091f837584ac`, without an existing `Cargo.lock`,
retained all 15 duplicate lines. The environment was macOS with Rust/Cargo
1.96.0 and cargo-deny 0.19.9, matching the cargo-deny version in the
[Security Audit workflow](../.github/workflows/security-audit.yml).

The incompatible requirements are:

- `aws-sigv4 1.6.3` requires `p256 ^0.14.0` and `crypto-bigint ^0.5.4` for
  SigV4a. Its inverse paths include `aws-runtime` and `aws-sdk-s3`.
- `openidconnect 4.0.1` requires `p256 ^0.13.2` and `p384 ^0.13.0`, including
  with default features disabled. It is an interoperability-test dependency
  of `reinhardt-auth`.
- `reinhardt-auth` also directly requires `p256 ^0.13` for social authentication.
- `jsonwebtoken 10.4.0` requires `signature ^2.2.0`. Published
  `jsonwebtoken 11.1.0` still requires that signature API and optionally
  `p256 ^0.13.2`, so that upgrade alone does not establish convergence.

These requirements are recorded in the published manifests for
[aws-sigv4 1.6.3](https://docs.rs/crate/aws-sigv4/1.6.3/source/Cargo.toml),
[openidconnect 4.0.1](https://docs.rs/crate/openidconnect/4.0.1/source/Cargo.toml),
and [jsonwebtoken 11.1.0](https://docs.rs/crate/jsonwebtoken/11.1.0/source/Cargo.toml),
and the local [authentication manifest](../crates/reinhardt-auth/Cargo.toml).

| Package | Older line | Newer line |
|---|---|---|
| base16ct | 0.2.0 | 1.0.0 |
| crypto-bigint | 0.5.5 | 0.7.6 |
| der | 0.7.10 | 0.8.2 |
| ecdsa | 0.16.9 | 0.17.0 |
| elliptic-curve | 0.13.8 | 0.14.1 |
| ff | 0.13.1 | 0.14.0 |
| group | 0.13.0 | 0.14.0 |
| p256 | 0.13.2 | 0.14.0 |
| pem-rfc7468 | 0.7.0 | 1.0.0 |
| pkcs8 | 0.10.2 | 0.11.0 |
| primeorder | 0.13.6 | 0.14.0 |
| rfc6979 | 0.4.0 | 0.6.0 |
| sec1 | 0.7.3 | 0.8.1 |
| signature | 2.2.0 | 3.0.0 |
| spki | 0.7.3 | 0.8.1 |

The unchanged policy passed `bans`, `licenses`, and `sources`. Removing only
the 15 `bans.skip` entries referring to #6840 from a temporary policy produced
exactly 15 `error[duplicate]` diagnostics against the same captured all-feature
metadata, one for each package above. None of these allowances was unnecessary.
The unrelated existing `pem` unnecessary-skip warning remained.

## Advisory status is separate

The complete audit failed its advisory category on
[RUSTSEC-2026-0387](https://github.com/rustsec/advisory-db/blob/main/crates/crypto-bigint/RUSTSEC-2026-0387.md),
published on 2026-10-23 for timing variability in `crypto-bigint`. The advisory
lists versions `>=0.7.6` as patched. Fresh resolution included both patched
`0.7.6` and affected `0.5.5`; AWS signing and `elliptic-curve 0.13` retained
the older line. No Reinhardt-specific exploitability was established.

A duplicate-version allowance does not suppress an advisory. #6840 does not
authorize accepting this advisory, and the audit failure must remain visible
until separately remediated. Passing the duplicate check alone is insufficient
evidence that the complete Security Audit passes.

## Recheck and removal procedure

Use a dedicated checkout of the affected release branch with no existing
`Cargo.lock`; retain any lockfile needed by other work. From that checkout,
resolve the full feature graph and inspect its current inverse dependencies:

```bash
cargo deny --workspace --all-features check all
cargo tree --workspace --all-features --invert p256@0.13.2 --depth 2
cargo tree --workspace --all-features --invert p256@0.14.0 --depth 2
cargo tree --workspace --all-features --invert crypto-bigint@0.5.5 --depth 2
```

The version selectors above describe the verified graph. Update them when
resolution selects different releases; a missing package selector is a reason
to inspect the new graph, not proof that every tracked family has converged.

After a relevant upstream release, assess each allowance separately:

1. Confirm that fresh all-feature resolution no longer retains both API lines
   for that package. Preserve S3 SigV4a, social authentication, JWT support, and
   OpenID Connect interoperability coverage.
2. Remove only the corresponding #6840 entries from `bans.skip`. Preserve
   `multiple-versions = "deny"` and unrelated policy entries.
3. Run `cargo deny --workspace --all-features check bans` and then
   `cargo deny --workspace --all-features check all`. Require all four categories
   to pass before reporting complete audit success.
4. If dependency declarations or authentication/signing behavior changed,
   validate the affected components and independently resolved consumers under
   [Verification Scope](../instructions/AGENT_WORKFLOW.md#verification-scope).

The final state has no #6840 RustCrypto entries in `bans.skip`, preserves the
supported features, and passes the full audit under fresh resolution. A
documentation update, dependency version bump, or tracker closure alone does
not satisfy that state. This record follows
[Upstream Issue Reporting](../instructions/UPSTREAM_ISSUE_REPORTING.md); no
external upstream report was submitted.
