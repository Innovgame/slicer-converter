# Rust 源码快速入门

本文面向刚开始读 Rust 的开发者，目标是先看懂 `slicer-converter` 的主流程，而不是一次钻进所有格式细节。

建议先沿着 `GOO -> CTB` 这条真实转换链路阅读。它能覆盖 CLI 参数、格式识别、header 解析、逐层转码、fast path、fallback、文件组装这些核心概念。

---

## 1. 先建立项目地图

Rust 代码集中在 `rust/`：

```text
rust/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── common/
│   ├── formats/
│   └── transcode/
└── tests/
```

几个入口的职责：

| 文件                           | 作用                                                                  |
| ------------------------------ | --------------------------------------------------------------------- |
| `rust/Cargo.toml`              | Rust crate 配置，声明依赖、lib、bench、release 参数                   |
| `rust/src/lib.rs`              | WASM/JS API 入口，例如 `parse_header`、`decode_layer`、`encode_layer` |
| `rust/src/main.rs`             | CLI 入口，支持 `slicer-converter --input ... --output ...`            |
| `rust/src/formats/mod.rs`      | 格式 family 注册表，负责返回 `FormatSpec`                             |
| `rust/src/transcode/engine.rs` | 跨格式逐层转码引擎，负责 fast path 和 fallback 编排                   |
| `rust/tests/cli.rs`            | CLI 行为测试，适合从“用户怎么调用”开始读                              |

这个项目同时支持浏览器 WASM 和 native CLI。新手阅读时，建议先看 CLI，因为 CLI 链路更直观：读文件、转换、写文件。

---

## 2. 如果你从 TypeScript / 前端背景切入

这个仓库不是“只有 Rust”。它更像三层：

```text
React / TypeScript UI
-> @slicer-converter/wasm browser facade
-> wasm-bindgen exports
-> rust/src/lib.rs
-> Rust core: formats + transcode
```

对应文件：

| 层                  | 文件                                                             | 作用                                          |
| ------------------- | ---------------------------------------------------------------- | --------------------------------------------- |
| React controller    | `apps/slicer-app/src/app/hooks/use-runtime-controller.ts`        | 初始化 WASM、同步格式注册表、预热 worker pool |
| Worker handler      | `apps/slicer-app/src/worker/handlers/conversion-handler.ts`      | 在 Web Worker 中调用 WASM 做逐层转换          |
| WASM browser facade | `packages/slicer-wasm/src/api.ts`                                | 初始化 WASM、导出最小 public facade           |
| Header helper       | `packages/slicer-wasm/src/browser/header.ts`                     | 把 Rust 返回的 JSON 转成 TS 对象              |
| App WASM facade     | `apps/slicer-app/src/infrastructure/wasm/slicer-wasm-runtime.ts` | app-local WASM 入口和初始化组合               |
| Rust WASM API       | `rust/src/lib.rs`                                                | `#[wasm_bindgen] pub fn ...` 的真实实现       |
| Rust core           | `rust/src/formats/*`、`rust/src/transcode/*`                     | 格式解析、层解码/编码、fast path/fallback     |

从 TS 看，常见入口是：

```typescript
await initAppSlicerWasmRuntime();
const formats = getSupportedFormats();
syncFormatFactsFromWasm(formats);
```

这对应 Rust 里的：

```rust
#[wasm_bindgen]
pub fn list_supported_formats() -> Result<String, JsValue> {
    // 从 formats::global_registry() 读取 Rust 格式注册表
}
```

所以你可以先建立这个映射：

```text
TS 调 parseBrowserFileHeader(...)
-> WASM 导出 parse_header(...)
-> rust/src/lib.rs::parse_header(...)
-> formats::global_registry().detect_spec(...)
-> spec.pipeline.parse_document(spec, data)
```

逐层转换时，Worker 里会调用：

```typescript
const encoded = deps.decodeEncodeLayer(
    new Uint8Array(data),
    index,
    sourceFormat,
    targetFormat,
    width,
    height,
    antiAliasLevel || 1,
    encryptionKey || 0,
    layerMetadataJson
);
```

这条线会落到 Rust 的 `decode_encode_layer`，再进入 `TranscodeEngine`：

```text
TS worker decodeEncodeLayer
-> rust/src/lib.rs::decode_encode_layer
-> transcode::default_engine().transcode_layer_into(...)
-> fast path 或 fallback
-> source_spec.decode_layer_into + target_spec.encode_layer_into
```

因此，如果你更熟 TS，建议用这个方法读 Rust：

1. 先从 `packages/slicer-wasm/src/api.ts` 和 `packages/slicer-wasm/src/browser/*.ts` 看 TS typed helpers 如何收口 raw WASM ABI。
2. 再去 `rust/src/lib.rs` 找同名 `#[wasm_bindgen] pub fn`。
3. 再从 `lib.rs` 往下追 `formats::...` 或 `transcode::...`。
4. 最后才进入具体格式目录，比如 `formats/goo`、`formats/ctb`。

---

## 3. 从测试开始看“怎么用”

先看 `rust/tests/cli.rs` 里的 `cli_converts_goo_to_ctb_with_input_and_output_flags`：

```rust
let input = fixture_path("sample.goo");
let output = temp_output_path("output.ctb");

let status = Command::new(cli_binary())
    .arg("--input")
    .arg(&input)
    .arg("--output")
    .arg(&output)
    .status()
    .expect("CLI process should launch");

assert!(status.success(), "CLI should exit successfully");
```

这段告诉你最重要的使用方式：

```bash
slicer-converter --input sample.goo --output output.ctb
```

测试还会读回输出文件，通过 `formats::detect_format_spec(&data, Some("ctb"))` 确认它真的是 CTB。

读测试的好处是：你不用先理解所有实现，就能知道系统对外承诺了什么行为。

---

## 4. CLI 主线：`main.rs`

CLI 从 `rust/src/main.rs` 的 `main` 开始：

```rust
fn main() {
    match run() {
        Ok(()) => {}
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}
```

先记住：

- `run()` 成功时返回 `Ok(())`
- 失败时返回 `Err(message)`
- `match` 根据成功/失败分别处理

真正的业务在 `run()`：

```rust
fn run() -> Result<(), String> {
    match parse_args(std::env::args().skip(1))? {
        CliAction::Help => {
            println!("{}", usage());
            Ok(())
        }
        CliAction::Run(args) => convert_file_with_options(&args.input, &args.output, &args)
            .map_err(|error| {
                format!(
                    "conversion failed for '{}' -> '{}': {error}",
                    args.input.display(),
                    args.output.display()
                )
            }),
    }
}
```

这里的阅读重点：

- `parse_args(...)` 负责解析 `--input`、`--output`、`--timing`、`--threads`
- `?` 表示失败就提前返回
- `CliAction::Help` 打印帮助
- `CliAction::Run(args)` 进入真实转换
- `convert_file_with_options(...)` 是 CLI 主转换函数

---

## 5. 文件转换主流程

`convert_file_with_options` 可以当成第一条核心阅读路线。

它的大致流程：

```text
读取输入文件 bytes
-> 获取输入/输出扩展名
-> detect_format_spec 识别源格式 spec
-> spec_by_extension 选择目标格式 spec
-> parse_header 得到 metadata 和 layer_offsets
-> transcode_layers 逐层转码
-> assemble_target_file 组装目标文件
-> fs::write 写入磁盘
```

对应概念：

| 概念                              | 含义                                              |
| --------------------------------- | ------------------------------------------------- |
| `source_bytes: Vec<u8>`           | 整个输入文件的字节                                |
| `source_format: &FormatSpec`      | 运行时识别出的源格式 spec，例如 GOO               |
| `target_format: &FormatSpec`      | 运行时根据输出扩展名选出的目标格式 spec，例如 CTB |
| `metadata: FileMetadata`          | 分辨率、层数、曝光、运动参数等通用元数据          |
| `layer_offsets: Vec<LayerOffset>` | 每一层图像数据在源文件里的 offset 和 size         |
| `encoded_layers: Vec<Vec<u8>>`    | 转码后每层的目标格式 bytes                        |

这条线读通以后，再看单个格式实现会容易很多。

---

## 6. 浏览器转换主流程

浏览器里不会直接调用 `main.rs`。`main.rs` 只属于 native CLI。浏览器路径走的是：

```text
React hook
-> Worker pool
-> worker handler
-> @slicer-converter/wasm
-> rust/src/lib.rs
-> Rust core
```

初始化阶段：

```text
useRuntimeController
-> initAppSlicerWasmRuntime
-> getSupportedFormats
-> syncWasmFactsFromRuntime
-> initManagedWorkerPool
```

这件事对应几个关键点：

| TS 概念                        | Rust 对应                                            |
| ------------------------------ | ---------------------------------------------------- |
| `list_supported_formats()`     | 读取 `formats::global_registry().catalog_profiles()` |
| `syncFormatFactsFromWasm(...)` | 把 Rust 注册表事实同步到前端 domain facts            |
| `FormatAdapter`                | 前端对 ZIP、文件命名、输出装配等 browser 行为的补充  |
| `WorkerPoolState`              | 前端并行调度层，不属于 Rust core                     |

解析文件头时：

```text
parseBrowserFileHeader(data, extension)
-> parse_header(data, extension)
-> rust/src/lib.rs::parse_header
-> detect_format_spec
-> source_spec.pipeline.parse_document(source_spec, data)
```

转换单层时：

```text
worker conversion handler
-> deps.decodeEncodeLayer(...)
-> WASM decode_encode_layer(...)
-> rust/src/lib.rs::decode_encode_layer
-> TranscodeEngine
```

这说明一个边界：

```text
TS/React 负责 UI、Worker 调度、ZIP/browser 行为和下载装配
Rust 负责格式识别、metadata 解析、layer decode/encode、fast path 转码
```

读源码时不要把这两层混在一起。比如 `apps/slicer-app/src/domain/format/format-facts.ts` 不是 Rust 格式解析器，它只是浏览器侧同步后的格式事实快照；真正的格式 parse/decode/encode 在 `rust/src/formats/*`。

---

## 7. 核心抽象：`FormatSpec`

`rust/src/formats/mod.rs` 和 `rust/src/formats/common/spec.rs` 是 Rust 格式内核的关键位置。

当前架构里，注册表不再返回某个具体 `*Format` wrapper，而是返回 `&'static FormatSpec`：

```rust
pub struct FormatSpec {
    pub identity: FormatIdentity,
    pub catalog: FormatCatalog,
    pub container: ContainerKind,
    pub capabilities: FormatCapabilities,
    pub keys: FormatKeys,
    pub header: &'static dyn HeaderLayout,
    pub layer_codec: &'static dyn LayerCodec,
    pub crypto: &'static dyn CryptoPolicy,
    pub footer: &'static dyn FooterPolicy,
    pub archive: &'static dyn ArchiveLayout,
    pub pipeline: &'static dyn FormatPipeline,
}
```

可以把 `FormatSpec` 先理解成“运行时执行内核 + 通用 catalog 字段”。`catalog` 放所有格式族都通用的 UI/registry 字段；Chitubox / Anycubic 这类格式族专属事实放在 `formats::catalog::FormatProfile.facts`，由 `global_registry().catalog_profiles()` 暴露给 WASM 和前端。具体行为由小策略对象组合出来：

| 入口 / slot                                | 作用                                              |
| ------------------------------------------ | ------------------------------------------------- |
| `pipeline.parse_document(spec, data)`      | 从完整文件 bytes 解析 metadata 和 layer offset 表 |
| `pipeline.encode_document_into(...)`       | 按 profile pipeline 组装完整输出文件              |
| `decode_layer` / `decode_layer_into`       | 把某一层的格式私有 bytes 解码成灰度像素           |
| `encode_layer` / `encode_layer_into`       | 把灰度像素编码成该格式的层数据                    |
| `header.build_header(...)`                 | 根据 metadata 构建目标文件 header                 |
| `header.finalize_header(...)`              | 部分格式需要在知道 layer size 后回填 header       |
| `footer.build_footer(...)`                 | 构建 footer，默认空                               |
| `header.try_detect(...)` / family resolver | 通过 magic bytes 判断是否是该格式                 |

真正的可替换接口更小，例如：

```rust
pub trait HeaderLayout: Sync { ... }
pub trait LayerCodec: Sync { ... }
```

所以你后续看到：

```text
GOO_SPEC 组合了 GooFamilyHeaderLayout + GooFamilyLayerCodec + GooFamilyFooterPolicy
CTB_V5_SPEC 组合了 CTB header/layout/codec 策略
调用层只依赖 FormatSpec，不需要硬编码每个格式的细节
```

---

## 8. 格式注册表

同一个文件里还有 `FORMAT_FAMILIES`：

```rust
pub const FORMAT_FAMILIES: &[&common::FormatFamilySpec] = &[
    &chitubox::FAMILY,
    &anycubic::FAMILY,
    &goo::FAMILY,
    &prz::FAMILY,
    &sl1::FAMILY,
    &vdt::FAMILY,
];
```

这就是 Rust 端支持格式的 family 注册入口。`BuiltinRegistry` 会把所有 family 的 `profiles` 展平成 `FormatSpec` 列表。

常见调用：

```rust
formats::detect_format_spec(&source_bytes, Some(input_extension))
formats::global_registry().spec_by_extension(output_extension)
```

读法：

- `detect_format_spec`：根据文件内容和扩展名提示识别源格式 spec
- `spec_by_extension`：根据输出扩展名选择目标格式 spec

---

## 9. 转码引擎：fast path 和 fallback

`rust/src/transcode/engine.rs` 很短，但很核心：

```rust
pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}
```

它持有一个转码策略注册表。真正的逐层转码逻辑在：

```rust
pub fn transcode_layer_into_with_stats(...)
```

核心流程：

```rust
let route = TranscodeRouteDescriptor::from_specs(src_fmt, tgt_fmt);
for transcoder in self.registry.lookup_route(route.route_key) {
    stats.strategy_attempts += 1;
    match transcoder.try_transcode_layer_into(src, req, scratch, out)? {
        FastPathOutcome::Applied => {
            stats.fast_path_applied = true;
            return Ok(stats);
        }
        FastPathOutcome::Fallback => {
            stats.strategy_declines += 1;
        }
    }
}

fallback::transcode_layer_with_formats(src_fmt, tgt_fmt, src, req, scratch, out)?;
stats.fallback_used = true;
Ok(stats)
```

翻译成业务语言：

```text
1. 从 source/target FormatSpec 派生转换 route
2. 看当前 route 有没有专门优化路径，例如 goo-rle -> ctb-rle
3. 有的话逐个尝试
4. 某个 fast path 成功，就直接返回
5. fast path 不适用，就继续尝试下一个
6. 所有 fast path 都没命中，走 fallback
7. fallback 就是稳定的 decode -> encode 通用路径
```

这也是项目里很重要的分层：

| 层                | 负责什么                                    |
| ----------------- | ------------------------------------------- |
| `FormatSpec`      | 单个格式 profile 的 parse/decode/encode     |
| `execute` facade  | 组装单层 request、复用 scratch、调用 engine |
| `TranscodeEngine` | source format 到 target format 的逐层编排   |
| `LayerTranscoder` | 某个格式对的专门 fast path                  |
| `fallback`        | 没有 fast path 时的通用 decode -> encode    |

---

## 10. 建议阅读顺序

### Rust CLI 线

1. `rust/tests/cli.rs`
2. `rust/src/main.rs`
3. `rust/src/formats/mod.rs`
4. `rust/src/transcode/execute.rs`
5. `rust/src/transcode/engine.rs`
6. `docs/rust-engine-walkthrough.md`
7. `rust/src/transcode/fallback.rs`

这条线最适合 Rust 新手，因为没有 React、Worker、浏览器异步装配干扰。

### TypeScript / 浏览器线

1. `packages/slicer-wasm/src/api.ts`
2. `packages/slicer-wasm/src/browser/header.ts`
3. `apps/slicer-app/src/app/hooks/use-runtime-controller.ts`
4. `apps/slicer-app/src/worker/handlers/conversion-handler.ts`
5. `rust/src/lib.rs`
6. `rust/src/transcode/engine.rs`
7. `docs/rust-engine-walkthrough.md`

这条线适合从前端调用链往 Rust core 追。

### 具体格式线

1. `rust/src/formats/goo/mod.rs`
2. `rust/src/formats/goo/decoder.rs`
3. `rust/src/formats/goo/encoder.rs`
4. `rust/src/formats/ctb/mod.rs`
5. `rust/src/formats/ctb/decoder.rs`
6. `rust/src/formats/ctb/encoder.rs`

### 性能和算法线

1. `rust/src/formats/goo/rle.rs`
2. `rust/src/formats/ctb/rle.rs`
3. `rust/src/transcode/paths/goo_to_ctb.rs`
4. `rust/src/transcode/paths/ctb_to_goo.rs`
5. `rust/benches/transcode_fast_path_bench.rs`

---

## 11. 新手阅读时先不要纠结的内容

刚开始可以先跳过：

- RLE 位运算细节
- SIMD 实现
- ZIP container 流式装配
- Anycubic 多 layout/profile 泛型
- benchmark 的统计细节
- WASM bindgen 边界里的 JS error 结构

先把这句话读懂：

```text
FormatSpec 负责 decode/encode，转码引擎负责调度 fast path 或 fallback，CLI/WASM 只是入口。
```

这就是本项目 Rust 核心的最小心智模型。

如果从 TS 角度再翻译一遍：

```text
React/Worker 负责什么时候调用、并发多少层、怎么组装下载；
@slicer-converter/wasm 负责把 TS 调用转成 WASM 调用；
rust/src/lib.rs 负责把 WASM API 参数转成 Rust core 参数；
formats 和 transcode 才是真正的格式转换核心。
```
