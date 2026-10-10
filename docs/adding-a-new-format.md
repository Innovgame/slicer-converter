# 如何新增一种切片格式（FormatSpec-first 指南）

本指南面向想为 slicer-converter 增加一种新切片格式（新的 Anycubic profile、ZIP 容器格式或自定义二进制格式）的开发者。当前目标架构是：

```text
Rust FormatFamilySpec / FormatSpec = 格式事实、执行内核和 archive output plan
Rust strategy slots = HeaderLayout / LayerCodec / CryptoPolicy / FooterPolicy / ArchiveLayout / FormatPipeline
TS BrowserFormatBehavior = 浏览器文件 API 专属行为补充
```

不要新增 `SlicerFormat` trait / `dyn SlicerFormat` / 具体 `*Format` wrapper。格式注册和执行入口都应落到 `FormatSpec`。

---

## 1. Rust 端：注册 `FormatSpec`

### 1.1 新增 Anycubic 二进制 profile

如果新格式与现有 Anycubic 协议族（V1 / V516 / V517 / V518）共享布局，优先在 [rust/src/formats/anycubic/specs.rs](../rust/src/formats/anycubic/specs.rs) 中追加一个 `anycubic_binary_spec!` 调用：

```rust
anycubic_binary_spec!(
    facts = MYFMT_ANYCUBIC,
    header_adapter = MYFMT_HEADER,
    codec_adapter = MYFMT_CODEC,
    spec = MYFMT_SPEC,
    profile = MYFMT_PROFILE,
    name = "myfmt",
    machine_id = AnycubicMachineId::PhotonMono4,
    display_name = "My Anycubic Machine",
    aliases = ["my anycubic machine"],
    resolution = (9024, 5120),
    display_mm = (153.408, 87.04),
    machine_z_mm = 165.0,
    supported_versions = [AnycubicVersion::V517],
    default_version = AnycubicVersion::V517,
    table_layout = AnycubicTableLayout::V517,
    layout = V517Layout,
    codec_impl = Pw0Codec,
    header = keys::HEADER_ANYCUBIC_V517,
    codec = keys::ANYCUBIC_PW0_RLE,
    default_export = false,
);
```

如果机型 facts 也是新的，直接在这个宏调用里补齐 `AnycubicMachineId`、显示名、别名、分辨率、屏幕尺寸、Z 高度、支持版本和默认版本。不要另建平行 profile 表；宏会同时生成 runtime `FormatSpec` 和 catalog `FormatProfile`，family facts 通过 `FormatProfile.facts` 暴露给 Rust registry、WASM facts 和前端 Anycubic 手动机型选择。

然后把 `&MYFMT_SPEC` 加入 `PROFILES`，把 `&MYFMT_PROFILE` 加入 `CATALOG_PROFILES`，并同步 `FAMILY.identity.aliases/extensions`。

### 1.2 新增全新二进制格式族

参照 [rust/src/formats/goo/specs.rs](../rust/src/formats/goo/specs.rs) 或 [rust/src/formats/prz/specs.rs](../rust/src/formats/prz/specs.rs)：

1. 在格式模块下实现专用策略，例如 `MyFmtHeaderLayout`、`MyFmtLayerCodec`、`MyFmtFooterPolicy`。
2. 在 `specs.rs` 声明 `pub static MYFMT_SPEC: FormatSpec`。
3. 将 `header` / `layer_codec` / `crypto` / `footer` / `archive` / `pipeline` slot 指向对应策略。
4. 声明 `pub static FAMILY: FormatFamilySpec`，把 `profiles` 指向 `&[&MYFMT_SPEC]`。
5. 声明 `pub static CATALOG_PROFILES: &[&FormatProfile]`。没有 family-specific facts 时使用 `FormatFacts::None`。
6. 在 [rust/src/formats/mod.rs](../rust/src/formats/mod.rs) 中把新 family 加入 `FORMAT_FAMILIES` 和 `CATALOG_PROFILE_GROUPS`。

`FormatSpec` 是运行期 profile 的事实源。完整文件解析/组装走 `spec.pipeline.parse_document(spec, data)` 和 `spec.pipeline.encode_document_into(...)`；单层编解码与二进制 header/footer 仍通过 `decode_layer_into`、`encode_layer_into`、`build_header`、`finalize_header` 等 `FormatSpec` inherent methods 进入对应策略。

### 1.3 新增 ZIP / archive 格式

ZIP 容器格式仍然通过 `FormatSpec` 注册，但通常需要：

- `container: ContainerKind::Archive`
- `capabilities: FormatCapabilities::archive_import_export()`
- 专用 `ArchiveLayout`
- `DefaultArchivePipeline` 或更具体的 `FormatPipeline`

可参考 [rust/src/formats/vdt/specs.rs](../rust/src/formats/vdt/specs.rs)、[rust/src/formats/sl1/specs.rs](../rust/src/formats/sl1/specs.rs)、[rust/src/formats/anycubic/specs.rs](../rust/src/formats/anycubic/specs.rs) 中的 archive profile。

### 1.4 Rust 测试

至少补以下验证：

- `rust/tests/format_family_registry.rs`：注册表能列出新 profile，capabilities 正确。
- `rust/tests/format_spec_kernel.rs`：不引入 wrapper / legacy adapter / 旧执行入口。
- `rust/tests/fixture_files.rs` 或格式专属测试：真实 fixture 可 parse，并能逐层 decode。
- 如支持导出，补 `build_header` / `finalize_header` / `encode_layer_into` roundtrip 或 CLI smoke。

缺真实样本时，按 `AGENTS.md` 的本地样本派生规则：优先从 `rust/data/` 的真实大样本通过现有 CLI / codec / pipeline 派生到 `/tmp`，并记录源文件、命令、输出路径和验证结果。

---

## 2. TS 端：声明浏览器 `FormatAdapter`

Rust/WASM 是格式事实源。TS 侧只保留 streaming parse、header build、ZIP entry build 等浏览器专属行为。

### 2.1 最小骨架

新建 [apps/slicer-app/src/core/formats/myfmt-plugin.ts](../apps/slicer-app/src/core/formats/myfmt-plugin.ts)：

```typescript
import type { FormatAdapter } from "./plugin";

export const myfmtPlugin: FormatAdapter = {
    format: "myfmt"
};
```

然后在 [apps/slicer-app/src/core/formats/index.ts](../apps/slicer-app/src/core/formats/index.ts) 的 `BUILTIN_FORMAT_ADAPTERS` 数组里追加 `myfmtPlugin`。

`wasm-facts-sync.ts` 会在启动时调用 runtime 的 `getSupportedFormats()`，再经 `syncFormatFactsFromWasm(...)` 同步 aliases / extensions / capabilities / isZip。

### 2.2 ZIP 容器格式

ZIP 输出策略不要放在 TS adapter 里。新增 ZIP 容器格式时，应优先在 Rust `ArchiveLayout` 中实现：

- `build_metadata_entries(ctx, metadata, total_layers, spec)`：生成 `config.ini` / `manifest.json` 等 metadata entries。
- `describe_layer_entry(ctx, layer_index, spec)`：返回单层 ZIP entry 的 name + compression method。

浏览器侧通过 `packages/slicer-wasm` 暴露的 `buildArchiveMetadataEntries` / `describeArchiveLayerEntries` 获取 typed entry plan，再交给 `ZipStreamAssembler` 和 `OutputSink` 流式写出，避免把完整 ZIP 复制进 WASM linear memory。

如果该格式还需要特殊的浏览器加载路径，再参考 [apps/slicer-app/src/core/formats/sl1-behavior.ts](../apps/slicer-app/src/core/formats/sl1-behavior.ts) 与 [apps/slicer-app/src/core/formats/vdt-behavior.ts](../apps/slicer-app/src/core/formats/vdt-behavior.ts) 实现 `documentOpener`。

### 2.3 TS 类型名

格式字符串 union 的公开类型是 `SlicerFormatId`，不是 `SlicerFormat`。如果新增 profile，需要同步：

- [packages/slicer-wasm/src/browser/types.ts](../packages/slicer-wasm/src/browser/types.ts)
- [packages/slicer-wasm/src/browser/format.ts](../packages/slicer-wasm/src/browser/format.ts) 中的 `asSlicerFormatId`
- [apps/slicer-app/src/core/formats/anycubic-plugin.ts](../apps/slicer-app/src/core/formats/anycubic-plugin.ts) 或对应 plugin 的 exhaustiveness 检查

---

## 3. WASM 同步

应用启动后 `useRuntimeController` 会调用：

```typescript
syncWasmFactsFromRuntime(getAppWasmFactsRuntime());
```

该函数：

- 以 Rust `FormatSpec` registry 上报的 facts 为准。
- 保留 TS `BrowserFormatBehavior` 的 `documentOpener` 等 browser-only hooks。
- 校验内置 adapter 必须能从 WASM facts 中找到对应格式。

因此新增格式时，Rust `FormatSpec` 是第一步；TS adapter 只补浏览器侧行为。

---

## 4. Worker 端处理新消息类型（如有）

如果新格式需要在 Web Worker 里走全新的处理路径（少见），见 [apps/slicer-app/src/worker/handlers/](../apps/slicer-app/src/worker/handlers/) 与 `WorkerRequest` union。新增消息类型若漏注册，应让 typecheck 暴露出来。

---

## 5. 提交前自检

```bash
# Rust
cargo test --manifest-path rust/Cargo.toml
cargo check --manifest-path rust/Cargo.toml --benches

# TS / app
pnpm typecheck
pnpm --filter @slicer-converter/wasm test:unit
pnpm --filter @slicer-converter/slicer-app test:unit
```

如新增真实样本或派生样本，还需要补 CLI smoke / roundtrip / Criterion semantic verification，不能只以“文件生成成功”作为验收。

---

## 6. 参考实现

- 二进制 `FormatSpec`：[goo/specs.rs](../rust/src/formats/goo/specs.rs) / [prz/specs.rs](../rust/src/formats/prz/specs.rs)
- Anycubic profile 批量声明：[anycubic/specs.rs](../rust/src/formats/anycubic/specs.rs)
- Archive format：[sl1/specs.rs](../rust/src/formats/sl1/specs.rs) / [vdt/specs.rs](../rust/src/formats/vdt/specs.rs)
- TS 最简 adapter：[ctb-plugin.ts](../apps/slicer-app/src/core/formats/ctb-plugin.ts) / [prz-plugin.ts](../apps/slicer-app/src/core/formats/prz-plugin.ts)
- TS ZIP adapter：[sl1-plugin.ts](../apps/slicer-app/src/core/formats/sl1-plugin.ts) / [vdt-plugin.ts](../apps/slicer-app/src/core/formats/vdt-plugin.ts)
