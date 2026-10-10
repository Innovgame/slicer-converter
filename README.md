# slicer-converter

Resin slicer file converter for MSLA and DLP printers. One Rust format registry drives the CLI and the browser WebAssembly build.

[![CI](https://github.com/Innovgame/slicer-converter/actions/workflows/ci.yml/badge.svg)](https://github.com/Innovgame/slicer-converter/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

树脂光固化切片文件转换器。同一套 Rust 格式注册表同时服务命令行和浏览器，在 Chitubox、Anycubic、Elegoo、Phrozen、Prusa、Voxeldance、Creality 等 MSLA/DLP 文件之间转换，并支持图层预览。

**本项目受到 [UVtools](https://github.com/sn4k3/UVtools) 的启发。** [UVtools](https://github.com/sn4k3/UVtools) 由 [Tiago Conceição](https://github.com/sn4k3) 维护，是树脂切片文件解析、查看、修复和格式互转的公开参考实现。slicer-converter 沿这条路线生长：格式探测、版本矩阵、archive 布局、manifest / config 字段，以及 RLE / PNG 图层编码，都以 UVtools 的格式行为为对齐基准，再做成可嵌入 CLI 与浏览器的 Rust 转换引擎。代码、运行时和许可证独立维护。本仓库与 UVtools 没有从属关系。

## 目录

- [和 UVtools 的关系](#和-uvtools-的关系)
- [能做什么](#能做什么)
- [支持的格式](#支持的格式)
- [快速开始](#快速开始)
- [命令行](#命令行)
- [浏览器 API](#浏览器-api)
- [开发](#开发)
- [文档](#文档)
- [致谢](#致谢)
- [许可证](#许可证)

## 和 UVtools 的关系

- 灵感来自 UVtools 对树脂切片格式的长期公开实现。
- 对齐范围包括 entry naming、manifest / config、版本矩阵、图层 codec 和 archive layout。新增或调整格式时，先对照 UVtools 对应实现，再落到本仓库的 spec 与测试。
- 本仓库的形状是 registry 驱动的 Rust core，外加 CLI 与 WebAssembly。浏览器和命令行读取同一份格式事实。
- UVtools 使用 [AGPL-3.0](https://github.com/sn4k3/UVtools/blob/master/LICENSE)。本仓库使用 [MIT](LICENSE)。

## 能做什么

- 在已注册的切片格式之间转换。可导入、可导出的 profile 由 Rust registry 决定。
- 在浏览器里完成解析、预览和转换。WebAssembly 在本地运行，转换本身不需要后端。
- 按图层流式解码和编码，方便处理较大的切片文件。
- 用文件内容做格式探测，扩展名作为提示。有字节时，magic / version 优先于扩展名。
- 在 wasm32 上对部分 RLE 路径使用 SIMD128。
- 按 CPU 核心数使用 Web Worker 做多层并行编解码，长时间转换可以通过 `AbortSignal` 取消。
- 打开文件后预览图层，测试页会显示首层缩略图。

当前版本是 `0.1.0`。包在 pnpm workspace 内使用，从源码构建。

## 支持的格式

完整、实时的扩展名列表以 `slicer-converter --help` 和浏览器侧 `getSupportedFormats()` 为准。下表按家族汇总当前 registry。

| 家族                                 | 扩展名                                        | 导入 | 导出 |
| ------------------------------------ | --------------------------------------------- | ---- | ---- |
| Chitubox CTB                         | `.ctb`                                        | 支持 | 支持 |
| Chitubox CBDDLP                      | `.cbddlp` `.cbddlp-v1`                        | 支持 | 支持 |
| Anycubic Photon（Chitubox 兼容文件） | `.photon`                                     | 支持 | 支持 |
| Chitubox PHZ                         | `.phz`                                        | 支持 | 支持 |
| Voxelab FDG                          | `.fdg`                                        | 支持 | 支持 |
| Anycubic PhotonS                     | `.photons`                                    | 支持 | 支持 |
| Chitubox Zip                         | `.zip`                                        | 支持 | 支持 |
| Elegoo GOO                           | `.goo`                                        | 支持 | 支持 |
| Phrozen PRZ                          | `.prz`                                        | 支持 | 支持 |
| Prusa SL1 / SL1S                     | `.sl1` `.sl1s`                                | 支持 | 支持 |
| Anycubic Photon Workshop             | 见下方列表                                    | 支持 | 支持 |
| Anycubic Zip                         | `.pm4u` `.pm7` `.pm7m` `.pwsz` `.pp1` `.pp1m` | 支持 | 支持 |
| Voxeldance VDT                       | `.vdt`                                        | 支持 | 支持 |
| Creality CXDLPv4                     | `.cxdlpv4`                                    | 支持 | 支持 |
| Creality CXDLP v1 / v2 / v3          | `.cxdlp` `.v1.cxdlp`                          | 支持 | 只读 |

补充：

- CTB 覆盖 v1–v5，以及 GKtwo profile。加密 CTB v4 / v5 也可导入、导出，扩展名仍是 `.ctb`。指定版本时使用 `--profile`，或使用带版本的输出名，例如 `model.v4.ctb`。
- GOO 当前 profile 是 GOO v3。PRZ 复用 GOO family 的容器和 RLE 形态，并保持独立的格式身份。
- SL1 与 VDT 是 ZIP archive：图层为 PNG，配置分别为 INI 与 JSON manifest。
- Anycubic 二进制文件按机器 profile 区分。布局版本用 `--version` 选择，例如 `1`、`515`、`516`、`517`、`518`。
- Creality CXDLP v1 / v2 / v3 当前只读。CXDLPv4 可以导入和导出。
- `.zip` 只是 Chitubox Zip 的扩展名提示。实际格式仍由 archive 内容判定。

Anycubic Photon Workshop 扩展名：`.pw0` `.pws` `.pwx` `.dlp` `.pwmo` `.pwms` `.pwma` `.pwmx` `.pmsq` `.pm3` `.pm3m` `.dl2p` `.pmx2` `.pwmb` `.pm3r` `.pm3n` `.pm4n` `.px6s` `.pm5` `.pm5s` `.m5sp`。

## 快速开始

前置依赖：Rust stable、[wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)、Node.js 22（与 CI 一致）、pnpm 10+。

```bash
pnpm install
pnpm build:wasm
pnpm dev
```

`pnpm dev` 会先构建 release WASM，再启动 `apps/slicer-app` 测试页。release 构建避免开发时误用 debug 版，导致转换性能失真。

常用产物：

- `packages/slicer-wasm/lib/`：wasm-pack 产物与 JS 胶水代码
- `apps/slicer-app/dist/`：`pnpm build` 生成的静态测试页

## 命令行

```bash
cargo build --manifest-path rust/Cargo.toml -p slicer-converter-cli --release
```

可执行文件：

- Windows：`rust/target/release/slicer-converter.exe`
- Linux / macOS：`rust/target/release/slicer-converter`

```bash
# 源格式从文件内容识别；目标格式默认从输出文件名推断
slicer-converter --input sample.goo --output sample.ctb

# 显式指定 CTB v4，不依赖文件名里的版本后缀
slicer-converter --input sample.goo --output sample.ctb --profile ctb-v4

# Anycubic 布局版本
slicer-converter --input sample.goo --output sample.pwmx --version 1

# 把分阶段耗时打到 stderr：read、transcode、assemble、write、total
slicer-converter --timing --input sample.goo --output sample.ctb
```

`--profile` 接受 profile id 或别名，例如 `ctb-v4`、`pwmx`、`cxdlpv4`。`--threads <n>` 覆盖图层转码的工作线程数。

在 Windows 上生成发布产物：

```powershell
pnpm build:cli:artifacts
```

这条命令生成当前平台的 Windows 版本，并在 WSL 可用时生成 Linux glibc 与 Linux musl 版本：

- `dist/windows/slicer-converter.exe`
- `dist/linux-glibc/slicer-converter`
- `dist/linux-musl/slicer-converter`

单个平台：

```powershell
./scripts/build-cli-artifacts.ps1 -Windows
./scripts/build-cli-artifacts.ps1 -LinuxGlibc
./scripts/build-cli-artifacts.ps1 -LinuxMusl
```

macOS 二进制需要在 macOS 上构建。Intel 与 Apple Silicon 分别产出 `x86_64-apple-darwin` 和 `aarch64-apple-darwin`。需要通用二进制时，再在 macOS 上用 `lipo` 合成。

## 浏览器 API

`@slicer-converter/wasm` 是浏览器侧入口。推荐使用 typed helper。底层 wasm-bindgen ABI，例如 `parse_header`、`decode_layer`、`encode_layer`，保留在 Rust/WASM 边界，根入口不把它们当作日常调用面。

```ts
import {
    convertSlicerFile,
    decodeSlicerLayerPreview,
    openSlicerDocumentFile,
    parseBrowserFileHeader
} from "@slicer-converter/wasm";

const document = await openSlicerDocumentFile({
    file,
    parseHeader: parseBrowserFileHeader
});

const preview = await decodeSlicerLayerPreview({
    document,
    layerIndex: 0,
    quality: "settled",
    maxPreviewSize: 1024
});

const result = await convertSlicerFile({
    file,
    target: "vdt",
    signal: abortController.signal,
    onProgress(progress) {
        console.log(progress.phase, progress.current, progress.total);
    }
});
```

调用方负责展示 `preview.bitmap`，以及下载或保存 `result.blob`。更完整的 facade、worker 和 archive helper 见 [Browser Integration Reference](docs/browser-integration-reference.md) 与 [packages/slicer-wasm/README.md](packages/slicer-wasm/README.md)。

常用 helper：

| Helper                     | 用途                                |
| -------------------------- | ----------------------------------- |
| `convertSlicerFile`        | 转换浏览器 `File` / `Blob`          |
| `openSlicerDocumentFile`   | 打开浏览器 `File`，扩展名取自文件名 |
| `decodeSlicerLayerPreview` | 解码已打开文档的单层预览            |
| `parseBrowserFileHeader`   | 解析文件头                          |
| `getSupportedFormats`      | 读取 Rust registry 的格式事实       |

## 开发

仓库是 pnpm workspace。根目录承载 workspace scripts；Rust 源码、Cargo 清单和构建缓存在 `rust/`。

```text
package.json                 # workspace scripts
pnpm-workspace.yaml
rust/                        # core / wasm / cli
packages/slicer-wasm/        # WASM 包与浏览器 facade
apps/slicer-app/             # Vite + React 19 测试页
docs/                        # 规格、架构与排障文档
```

- `rust/crates/core/`：格式编解码、registry、transcode
- `rust/crates/wasm/`：wasm-bindgen 导出
- `rust/crates/cli/`：命令行入口
- `packages/slicer-wasm/src/index.ts`：浏览器统一导出
- `apps/slicer-app/src/main.tsx`：测试页入口

前端 workspace 使用 TypeScript 6.x 与 React 19。

```bash
pnpm check          # Prettier + ESLint + TypeScript
pnpm test           # typecheck + Rust + WASM 包单测 + 前端单测 + scripts
pnpm test:rust
pnpm test:wasm      # wasm-pack Node runner
pnpm test:browser   # WASM browser contract + Playwright e2e
pnpm test:full      # pnpm test + pnpm test:browser
pnpm benchmark:rust
pnpm benchmark:app
pnpm fixtures       # 刷新 rust/crates/core/tests/fixtures/ 中的生成样本
```

`pnpm benchmark:wasm:browser` 运行 WASM SIMD 微基准。该 harness 目前通过 panic 文本回传吞吐数据，命令会以非零退出码结束。

Rust 真实文件 fast-path 基准可以设置 `SC_REAL_GOO_PATH`、`SC_REAL_PRZ_PATH`、`SC_REAL_CTB_PATH`。可选的 `SC_REAL_LAYER_LIMIT`，或对应的 `SC_REAL_GOO_LAYER_LIMIT` / `SC_REAL_PRZ_LAYER_LIMIT` / `SC_REAL_CTB_LAYER_LIMIT`，用来限制采样层数。

推送到 `master`，或打开指向 `master` 的 pull request，会运行 [CI](.github/workflows/ci.yml)。CI 包含 lint、Rust 测试与 Clippy、前端和脚本测试、wasm-pack，以及不依赖本机 UVtools 的 Playwright 用例。

`master` 上的 CI 成功后，[Deploy to GitHub Pages](.github/workflows/deploy.yml) 会把测试页发布到 <https://innovgame.github.io/slicer-converter/>。也可以在 Actions 里手动运行这个部署工作流。Pages 的 Source 需要设为 GitHub Actions。

## 文档

- [故障排查 / FAQ](docs/troubleshooting-faq.md)
- [部署 Runbook](docs/deployment-runbook.md)
- [Browser Integration Reference](docs/browser-integration-reference.md)
- [新增格式指南](docs/adding-a-new-format.md)
- [架构图](docs/architecture-diagrams.md)
- [Rust quickstart](docs/rust-quickstart.md)

## 致谢

感谢 [Tiago Conceição](https://github.com/sn4k3) 与 [UVtools](https://github.com/sn4k3/UVtools) 贡献者。UVtools 让树脂切片格式有了一份可持续对照的公开实现。本项目的格式行为对齐建立在这份工作之上。

## 许可证

[MIT](LICENSE)
