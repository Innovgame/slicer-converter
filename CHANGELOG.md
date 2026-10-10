# Changelog

This file records changes that users can see. Versions follow repository tags, for example `v0.1.0`.

## Unreleased

- The test page header links to the repository, a bug report, and a feature request.
- The test page uses a warm resin workbench. Source and export panels are marked, and export fields are grouped into output and target.
- The layer rail stays with the thumb while dragging, and the layer number can be typed.
- The test page can switch between English and Chinese. The choice is saved in the browser.
- The README opens with install steps and the Release binary names.

## 0.1.3

- Node can initialise the published package by reading the bundled wasm file. Browser loading still uses `fetch`.
- The package publishes to GitHub Packages as `@innovgame/slicer-converter`, and to npm as `slicer-converter` when `NPM_TOKEN` is set.

## 0.1.2

- GitHub Discussions are enabled.
- Dependabot checks npm, Cargo, and GitHub Actions weekly.
- crates.io publishing is wired for tagged releases and waits for `CARGO_REGISTRY_TOKEN`.
- `@innovgame/slicer-converter` publishes built `dist` and `lib` artifacts to GitHub Packages.

## 0.1.1

- Release assets add macOS Apple Silicon and `SHA256SUMS`. Intel macOS builds from source.
- `examples/sample.ctb` is a tiny file for a first CLI conversion.
- Maintainer notes moved under `docs/internal/`.

## 0.1.0

First public snapshot.

- The CLI, browser WebAssembly build, and test page share one Rust format registry.
- Import and export cover Chitubox CTB, CBDDLP, Photon, PHZ, FDG, PhotonS, and Zip; Elegoo GOO; Phrozen PRZ; Prusa SL1 and SL1S; Anycubic Photon Workshop and Anycubic Zip; Voxeldance VDT; and Creality CXDLPv4.
- Creality CXDLP v1, v2, and v3 are import-only.
- Browser conversion runs locally and does not need a backend.
- npm and crates.io packages are not published. Download the CLI from GitHub Releases and build the browser package from source.
