# Changelog

This file records changes that users can see. Versions follow repository tags, for example `v0.1.0`.

## Unreleased

## 0.1.5

- Mobile navigation, file and export drawers, layer controls, and preview tools share a floating frosted-glass style with light and dark themes.
- Mobile settings use iOS-style grouped lists and labeled segments for theme and language. Switching language keeps the settings page open.
- Pixel grids stay aligned while zooming and dragging in mobile device emulation. Preview zoom now reaches 128×, and dragging presents complete frames without flicker.
- Mobile preference and onboarding tests follow the current navigation and segmented controls. Local temporary diagnostic scripts are excluded from lint checks.

- Anycubic Zip keeps bottom exposure and layer count in its active resin settings on export and import.
- Creality files are read in full before checksum validation, so large CXDLPv4 files open through the browser document API.

- Truncated CTB files and invalid layer ranges are rejected before conversion.
- Cancelling conversion stops pending dispatch and discards in-flight conversion workers before pool reuse.
- Opening files quickly keeps the latest selection and closes obsolete documents.
- Package initialization also prepares the Rust format catalog for file opening and target selection.
- CLI tests use isolated temporary output directories. CI now requires the real benchmark fixture and includes a browser roundtrip smoke check.

- The test page header shows the published version beside the product name. On a phone, the links and toggles move below the name so both stay readable.
- Preview zoom sits in one field with its percent unit. The layer rail and the layer number beside it are narrower.
- A long source file name shows the full name on hover.

## 0.1.4

- The test page header links to the repository, a bug report, and a feature request.
- The test page uses a warm resin workbench. Source and export panels are marked, and export fields are grouped into output and target.
- The layer rail stays with the thumb while dragging, and the layer number can be typed.
- The test page can switch between English and Chinese. The choice is saved in the browser.
- A first visit offers a tour of opening a file, inspecting layers, exporting, and sending feedback. The tour follows the selected language, can be dismissed, and can be replayed from the help button.
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
