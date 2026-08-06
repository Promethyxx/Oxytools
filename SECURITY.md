# Security Policy

## Supported Versions

Oxytools follows a single active development branch. Only the latest
published release receives security fixes — there is no backport policy
for older versions.

## Reporting a Vulnerability

Please report vulnerabilities by opening a **Pull Request**:

**→ https://github.com/Promethyxx/Oxytools/pulls**

Describe the issue and, if possible, include a fix directly in the PR.

A PR can be merged as soon as it's reviewed and approved — there is no
coordinated disclosure delay; the fix becomes public as soon as it's merged.

### What helps triage a report quickly

- Oxytools version affected (version number, or commit if built from source)
- Platform (Windows / Linux x64 / Linux ARM / macOS) and variant (bundled / office)
- Reproduction steps, or a minimal file/input that triggers the issue
- Estimated impact (what the vulnerability actually allows)

## What to expect

Oxytools is maintained by a single person, on their own time — there is no
formal SLA. An acknowledgement within a few days is a reasonable goal, not
a guarantee. Fix turnaround depends on severity and complexity.

## Scope

In scope: Oxytools' own source code (all modules), the repository's scripts
and CI workflows, and how the application invokes the third-party binaries
it bundles (ffmpeg, ffprobe, mkvpropedit).

Out of scope: vulnerabilities in ffmpeg, mkvtoolnix, or any other
third-party dependency itself — those should be reported directly to their
respective projects. That said, a report is still welcome here if a
vulnerable version of a bundled binary is still being distributed by
Oxytools after an upstream fix.

## Dependencies

The project's Rust dependencies are audited automatically in CI via
`cargo-deny` (licenses, sources, RustSec security advisories) — see
[`deny.toml`](./deny.toml) for the configuration and documented exceptions.
