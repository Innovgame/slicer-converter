# slicer-converter

[English](README.md) | [简体中文](README.zh-CN.md)

树脂光固化（MSLA / DLP）切片文件转换器。同一套 Rust 格式注册表同时驱动命令行和浏览器 WebAssembly。

[![CI](https://github.com/Innovgame/slicer-converter/actions/workflows/ci.yml/badge.svg)](https://github.com/Innovgame/slicer-converter/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/slicer-converter)](https://www.npmjs.com/package/slicer-converter)
[![release](https://img.shields.io/github/v/release/Innovgame/slicer-converter)](https://github.com/Innovgame/slicer-converter/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**本项目受到 [UVtools](https://github.com/sn4k3/UVtools) 的启发。** 格式探测、版本矩阵、archive 布局、manifest 字段和图层编码都与 UVtools 对齐。本仓库是独立的 MIT 许可 Rust 转换与预览引擎，与 UVtools 没有从属关系。

![Slicer File Converter 预览](docs/images/app-preview-2.png)

## 安装

当前版本 `0.1.4`。问题可以发到 [Discussions](https://github.com/Innovgame/slicer-converter/discussions)。变更见 [CHANGELOG.md](CHANGELOG.md)。

### CLI

从 [Releases](https://github.com/Innovgame/slicer-converter/releases) 下载二进制。每次发布都包含 `SHA256SUMS`。

```bash
sha256sum -c SHA256SUMS
```

在 Windows 上，把文件哈希和 `SHA256SUMS` 里对应的一行比较：

```powershell
Get-FileHash .\slicer-converter-windows-x86_64.exe -Algorithm SHA256
```

### npm

```bash
npm install slicer-converter
```

```ts
import { initSlicerConverter } from "slicer-converter";

const api = await initSlicerConverter();
const formats = api.getSupportedFormats();
```

调用其他 API 之前，先执行 `initSlicerConverter()`。Node 会直接读取随包的 wasm 文件。浏览器则用 `fetch` 加载它。

### GitHub Packages

先在 `.npmrc` 里写上，再安装 `@innovgame/slicer-converter`：

```text
@innovgame:registry=https://npm.pkg.github.com
```

```bash
npm install @innovgame/slicer-converter
```

导入路径是 `@innovgame/slicer-converter`。API 与 npm 包相同。

## 快速开始

下载 Release 二进制后，用你自己的切片文件运行。Windows：

```powershell
.\slicer-converter-windows-x86_64.exe --input your-file.goo --output your-file.ctb
```

Linux 或 macOS 先加上可执行权限，再使用对应的文件名：

```bash
chmod +x ./slicer-converter-linux-x86_64
./slicer-converter-linux-x86_64 --input your-file.goo --output your-file.ctb
```

macOS Apple Silicon 使用 `slicer-converter-macos-aarch64`。源格式从文件内容识别，目标格式从输出文件名推断。

如果克隆了本仓库，`examples/sample.ctb` 是一个很小的样本，只够试 CLI，不是可打印模型。

```bash
slicer-converter --input examples/sample.ctb --output sample.sl1
```

浏览器测试页：<https://innovgame.github.io/slicer-converter/>。

## 限制

- Creality CXDLP v1、v2、v3 只读。CXDLPv4 可以导入和导出。
- Release 二进制覆盖 Windows、Linux glibc、Linux musl 和 macOS Apple Silicon。Intel macOS 从源码构建。
- Node 和浏览器在转换或预览前都要先调用 `initSlicerConverter()`。

## 能做什么

- 在已注册的切片格式之间转换。能否导入、能否导出由 Rust registry 决定。
- 在浏览器里解析、预览和转换。WebAssembly 在本地运行，转换不需要后端。
- 用文件内容做格式探测。扩展名只是提示。有字节时，magic / version 优先于扩展名。
- 按图层流式编解码，长时间转换可以通过 `AbortSignal` 取消。

## 支持的格式

完整扩展名列表以 `slicer-converter --help` 和 `getSupportedFormats()` 为准。下表按家族汇总当前 registry。

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
- SL1 与 VDT 是 ZIP archive。图层为 PNG。配置分别为 INI 与 JSON manifest。
- Anycubic 二进制文件按机器 profile 区分。布局版本用 `--version` 选择，例如 `1`、`515`、`516`、`517`、`518`。
- `.zip` 只是 Chitubox Zip 的扩展名提示。实际格式由 archive 内容判定。

Anycubic Photon Workshop 扩展名：`.pw0` `.pws` `.pwx` `.dlp` `.pwmo` `.pwms` `.pwma` `.pwmx` `.pmsq` `.pm3` `.pm3m` `.dl2p` `.pmx2` `.pwmb` `.pm3r` `.pm3n` `.pm4n` `.px6s` `.pm5` `.pm5s` `.m5sp`。

## 命令行

```bash
# 源格式从文件内容识别；目标格式默认从输出文件名推断
slicer-converter --input sample.goo --output sample.ctb

# 显式指定 CTB v4，不依赖文件名里的版本后缀
slicer-converter --input sample.goo --output sample.ctb --profile ctb-v4

# Anycubic 布局版本
slicer-converter --input sample.goo --output sample.pwmx --version 1
```

`--profile` 接受 profile id 或别名，例如 `ctb-v4`、`pwmx`、`cxdlpv4`。`--threads <n>` 覆盖图层转码的工作线程数。`--timing` 把分阶段耗时打到 stderr。

从源码构建：

```bash
cargo build --manifest-path rust/Cargo.toml -p slicer-converter-cli --release
```

本地多平台 CLI 产物见 [部署 Runbook](docs/deployment-runbook.md)。

## 浏览器与 Node API

从 npm 安装 `slicer-converter`，或从 GitHub Packages 安装 `@innovgame/slicer-converter`。优先使用 typed helper。底层 wasm-bindgen ABI 保留在 Rust/WASM 边界。

```ts
import {
    convertSlicerFile,
    decodeSlicerLayerPreview,
    initSlicerConverter,
    openSlicerDocumentFile,
    parseBrowserFileHeader
} from "slicer-converter";

await initSlicerConverter();

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

调用方负责展示 `preview.bitmap`，以及下载或保存 `result.blob`。更多 helper 见 [浏览器集成参考](docs/browser-integration-reference.md) 与 [packages/slicer-wasm/README.md](packages/slicer-wasm/README.md)。

| Helper                     | 用途                                |
| -------------------------- | ----------------------------------- |
| `initSlicerConverter`      | 加载随包的 wasm 模块                |
| `convertSlicerFile`        | 转换浏览器 `File` / `Blob`          |
| `openSlicerDocumentFile`   | 打开浏览器 `File`，扩展名取自文件名 |
| `decodeSlicerLayerPreview` | 解码已打开文档的单层预览            |
| `parseBrowserFileHeader`   | 解析文件头                          |
| `getSupportedFormats`      | 读取 Rust registry 的格式事实       |

## 开发

前置依赖：Rust stable、[wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)、Node.js 22、pnpm 10+。

```bash
pnpm install
pnpm build:wasm
pnpm dev
```

`pnpm dev` 会先构建 release WASM，再启动 `apps/slicer-app` 测试页。

```bash
pnpm check
pnpm test
pnpm test:rust
pnpm test:wasm
pnpm test:browser
pnpm test:full
```

推送到 `master`，或打开指向 `master` 的 pull request，会运行 [CI](.github/workflows/ci.yml)。`master` 上的 CI 成功后，[Deploy to GitHub Pages](.github/workflows/deploy.yml) 会发布测试页。

仓库布局：

- `rust/crates/core/`：格式编解码、registry、transcode
- `rust/crates/wasm/`：wasm-bindgen 导出
- `rust/crates/cli/`：命令行入口
- `packages/slicer-wasm/`：浏览器与 Node facade
- `apps/slicer-app/`：Vite + React 19 测试页

## 文档

- [文档索引](docs/README.md)
- [故障排查 / FAQ](docs/troubleshooting-faq.md)
- [部署 Runbook](docs/deployment-runbook.md)
- [浏览器集成参考](docs/browser-integration-reference.md)
- [新增格式指南](docs/adding-a-new-format.md)
- [架构图](docs/architecture-diagrams.md)
- [Rust quickstart](docs/rust-quickstart.md)

维护笔记在 `docs/internal/`，不是使用说明。

## 参与贡献

见 [CONTRIBUTING.md](CONTRIBUTING.md)、[SECURITY.md](SECURITY.md) 和 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 致谢

感谢 [Tiago Conceição](https://github.com/sn4k3) 与 [UVtools](https://github.com/sn4k3/UVtools) 贡献者。UVtools 让树脂切片格式有了一份可持续对照的公开实现。

## 许可证

[MIT](LICENSE)
