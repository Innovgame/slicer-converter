# Contributing

Thanks for improving slicer-converter. The project is inspired by [UVtools](https://github.com/sn4k3/UVtools). Format behavior is aligned with UVtools. The implementation is an independent Rust registry, CLI, and WebAssembly build.

## Setup

- Rust stable. `rust-toolchain.toml` installs `rustfmt`, `clippy`, and the `wasm32-unknown-unknown` target.
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js 22
- pnpm 10+

```bash
pnpm install
pnpm build:wasm
pnpm dev
```

## Checks before a pull request

```bash
pnpm check
pnpm test:rust
```

Run `pnpm test:wasm:unit` after browser or WASM package changes. Run `pnpm test:app` after test-page changes. Use `pnpm test:full` for the full local gate.

Format Rust from the `rust/` directory with `cargo fmt`. Running `cargo fmt --manifest-path rust/Cargo.toml` from the repository root can fail to find targets.

## Adding or changing a format

Read the matching UVtools implementation first:

- UVtools formats: `../UVtools/UVtools.Core/FileFormats/`
- Local guide: [docs/adding-a-new-format.md](docs/adding-a-new-format.md)

Check entry naming, manifest and config fields, version matrices, layer codecs, and archive layout. New test names use English first and Chinese second.

## Commits

Use a short English conventional commit, such as `fix:`, `feat:`, `docs:`, or `ci:`. Say what a user can observe.

## Releases

Releases are cut from `master` with a `v*` tag. Do not keep a long-lived release branch.

1. Move the `Unreleased` notes in [CHANGELOG.md](CHANGELOG.md) under the new version heading, for example `## 0.1.2`.
2. Update the version in [README.md](README.md) to the same version.
3. Commit those changes.
4. Wait until CI passes for that commit.
5. Tag and push the tag:

```bash
git tag v0.1.2
git push origin v0.1.2
```

Pushing the tag builds the CLI binaries and uploads them to GitHub Releases.

## Reporting issues

- Defects: [bug report](.github/ISSUE_TEMPLATE/bug_report.md)
- A file that will not open or convert: [format compatibility](.github/ISSUE_TEMPLATE/format_compatibility.md)
- New capability: [feature request](.github/ISSUE_TEMPLATE/feature_request.md)
- Vulnerabilities: [SECURITY.md](SECURITY.md). Do not open a public issue.
