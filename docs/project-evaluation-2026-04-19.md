# 项目综合评估报告

**评估日期**: 2026-04-19
**项目**: slicer-converter (MSLA/DLP 切片文件格式转换器)
**综合评分**: 4.4 / 5

**复评说明**: 本次为基于最新仓库状态的增量复评，重点复核了浏览器侧适配器职责重组、preview session 边界下沉、target format catalog 集中化、测试补强、README/API/CI 文档补充，以及代码风格配置显式化等变化。

## 本次较上一版的关键变化

1. 浏览器侧 `FormatAdapter` 已从“多个零散可选方法”收敛为 `documentOpener` + `zipOutput` 两个职责块。
2. `preview-session.ts` 和 `format-target-catalog.ts` 将预览派发与目标格式目录从 controller 中抽离，前端边界更清晰。
3. 仓库已加入 `.prettierrc`，代码风格从隐式默认转为显式约束。
4. `README.md` 已补充 API、CI、Benchmark、测试分层等信息，文档完整度明显提升。
5. 单元测试新增了 preview session、format target catalog、opened document、runtime metrics 等边界覆盖。

---

## 目录

- [一、架构六维度评估](#一架构六维度评估)
    - [1. 易扩展性](#1-易扩展性--4分)
    - [2. 性能](#2-性能--5分)
    - [3. 面向对象封装](#3-面向对象封装--45分)
    - [4. 多态](#4-多态--5分)
    - [5. 组合优于继承](#5-组合优于继承--5分)
    - [6. 面向接口编程](#6-面向接口编程--45分)
- [二、工程化维度评估](#二工程化维度评估)
    - [7. 测试覆盖与策略](#7-测试覆盖与策略--45分)
    - [8. 构建与部署流程](#8-构建与部署流程--45分)
    - [9. 文档完整度](#9-文档完整度--4分)
    - [10. 依赖健康度](#10-依赖健康度--45分)
    - [11. 代码规范与 lint](#11-代码规范与-lint--45分)
    - [12. 错误处理与健壮性](#12-错误处理与健壮性--3分)
- [三、评分总览](#三评分总览)
- [四、改进建议优先级](#四改进建议优先级)

---

## 一、架构六维度评估

### 1. 易扩展性 — 4分

#### Rust 侧（优秀）

新增格式只需 3 步：

1. 创建模块实现 `SlicerFormat` trait
2. 在 `FORMATS` 常量表添加 `slicer_format!("xxx", ["xxx"], XxxFormat)` 一行
3. 完成 — 注册表自动发现

Anycubic 家族用泛型 `AnycubicFormat<C: AnycubicLayerCodec, L: AnycubicLayoutStrategy>` 搭配宏，一行生成 21 种格式变体，扩展成本极低：

```rust
// rust/src/formats/anycubic/mod.rs
anycubic_format!(PW0_V1_FORMAT, extension = "pw0", codec = Pw0Codec, layout = V1Layout);
```

宏定义（`rust/src/formats/mod.rs`）：

```rust
macro_rules! slicer_format {
    ($name:literal, [$($alias:literal),+ $(,)?], $format:expr) => {
        FormatDesc {
            name: $name,
            aliases: &[$($alias),+],
            format: &$format as &(dyn SlicerFormat + Sync),
        }
    };
}
```

#### TypeScript 侧（较上一版明显改善）

浏览器侧扩展模型已经比上一版更清晰：

```typescript
export interface DocumentOpener {
    open(file: File): Promise<OpenedDocumentSource>;
}

export interface ZipOutputPolicy {
    buildHeaderBytes?(...): Uint8Array;
    buildMetadataEntries?(...): ZipArchiveEntry[];
    describeLayer?(...): Pick<ZipArchiveEntry, "name" | "compressionMethod">;
}

export interface FormatAdapter {
    readonly format: SlicerFormat | string;
    readonly documentOpener?: DocumentOpener;
    readonly zipOutput?: ZipOutputPolicy;
}
```

此外，`format-target-catalog.ts` 已将目标格式展示顺序、默认目标、benchmark 目标矩阵集中管理，减少了 UI 层散落规则。

当前剩余的扩展性短板主要有两点：

1. `apps/slicer-app/src/core/format-registry.ts` 仍硬编码 `KNOWN_FORMAT_INFOS`，没有完全以 Rust/WASM 为单一事实源。
2. 浏览器内置 adapter 仍需要在 `apps/slicer-app/src/core/formats/index.ts` 手工聚合注册。

因此，浏览器侧的“接口粒度”问题已显著改善，但“跨语言注册事实重复”问题仍然存在，评分维持 4 分。

---

### 2. 性能 — 5分

性能设计是本项目最突出的亮点：

| 优化手段                                       | 代码位置                                           | 效果                     |
| ---------------------------------------------- | -------------------------------------------------- | ------------------------ |
| **WASM SIMD128** RLE 编解码                    | Rust CTB/GOO codec                                 | 16 bytes/cycle           |
| **Thread-local buffer 池**                     | `rust/src/lib.rs` `decode_encode_layer`            | 避免跨 WASM 调用反复分配 |
| **`_into` 变体** (decode/encode_layer_into)    | 所有 `SlicerFormat` impl                           | 缓冲区复用，零额外分配   |
| **自适应 Worker 池** + 空闲回收                | `apps/slicer-app/src/core/worker-pool.ts`          | cores-1 并行, max 8      |
| **自适应预取窗口**                             | layer-prefetch                                     | I/O 与计算解耦           |
| **流式 ZIP 组装** + 乱序缓冲                   | `apps/slicer-app/src/core/zip-stream-assembler.ts` | 1000+ 层不需全部驻留内存 |
| **`LayerEncodeMetadata<'a>`** 借用视图         | `rust/src/formats/mod.rs`                          | 编码路径零拷贝           |
| **Preview 降采样** 最近邻 + OffscreenCanvas 池 | worker preview handler                             | 避免全尺寸解码           |

`_into` 方法的默认实现 fallback 到分配版本，保证了 API 向后兼容：

```rust
// rust/src/formats/anycubic/codec.rs
fn decode_layer_into(&self, data: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
    *output = self.decode_layer(data)?;  // 默认 fallback
    Ok(())
}
```

Thread-local buffer 池在 WASM 调用间复用内存：

```rust
// rust/src/lib.rs
thread_local! {
    static PIXEL_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ENCODE_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}
```

---

### 3. 面向对象封装 — 4.5分

#### Rust 侧

`FileMetadata` 嵌套组合结构体封装了 6 个关注点，每个都有清晰的职责边界：

```
FileMetadata
├── PrinterSpec      (分辨率、显示尺寸、机器名)
├── SlicingSpec       (层高、层数、AA、加密)
├── ExposurePlan      (曝光时间、PWM)
├── MotionPlan        (抬升/回退速度)
├── WaitTimes         (等待时间)
└── PrintStatistics   (打印时间、体积、重量)
```

Builder 模式 + 验证逻辑确保 `FileMetadata` 不会处于非法状态：

```rust
impl FileMetadata {
    pub fn builder() -> FileMetadataBuilder { ... }
}

impl FileMetadataBuilder {
    pub fn resolution(mut self, x: u32, y: u32) -> Self { ... }
    pub fn build(self) -> Result<FileMetadata, ConvertError> { /* 验证 */ }
}
```

`ConvertError` 枚举统一了所有编解码错误，且带有结构化上下文（offset, expected/actual magic 等）。

#### TypeScript 侧

Hook 组合层次仍然清晰：

```
useConverterController (编排者)
  ├── useDocumentController  (文件加载/解析)
  ├── usePreviewController   (预览渲染)
  ├── useConversionController(格式转换)
  └── useRuntimeController   (WASM 初始化/Worker 池)
```

相比上一版，前端封装边界已经明显更健康：

1. `preview-session.ts` 将 raw/zip 两种预览派发负载构建统一收口，不再由 controller 直接拼接 worker request。
2. `format-target-catalog.ts` 将目标格式展示、默认目标和 benchmark 矩阵集中封装，UI 不再散落格式策略。
3. `document-loader.ts` 只负责编排，遇到特殊格式时通过 `adapter.documentOpener.open()` 委派给格式插件。
4. `conversion-controller.ts` 通过 `descriptor.zipOutput` 消费 ZIP 构建能力，减少针对 SL1/VDT 的条件分支泄漏。

这轮重构让浏览器侧对象职责更接近“会话对象 + 策略对象 + 编排者”的分层，因此该维度从 4 分上调到 4.5 分。当前主要保留问题是转换链路中的错误上下文仍主要以字符串传播，封装还没有延伸到错误模型层。

---

### 4. 多态 — 5分

项目同时运用了**编译期多态**和**运行时多态**，选择恰当：

#### 编译期多态（泛型/单态化）

```rust
// 21 种 Anycubic 格式 → 编译期特化，零虚函数开销
impl<C: AnycubicLayerCodec, L: AnycubicLayoutStrategy> SlicerFormat for AnycubicFormat<C, L>
```

#### 运行时多态（trait object）

```rust
// 注册表需要异构格式集合 → 动态分发
pub format: &'static (dyn SlicerFormat + Sync)
fn global_registry() -> &'static dyn FormatRegistry
```

#### TypeScript 侧

- `OutputSink` 接口的 `BlobOutputSink` / `StreamOutputSink` 双实现 — 策略模式
- Worker 消息协议的 discriminated union (`type: "layer-done" | "preview-done" | ...`) — 类型安全的分发
- `FormatAdapter` 的两个可选能力字段 `documentOpener` / `zipOutput` → 运行时按格式特性选择行为

---

### 5. 组合优于继承 — 5分

项目**全面贯彻组合模式**，零继承使用：

#### Rust 侧

- `AnycubicFormat<C, L>` 通过组合 Codec + Layout 策略对象替代继承层级

```rust
pub struct AnycubicFormat<C, L>
where
    C: AnycubicLayerCodec,
    L: AnycubicLayoutStrategy,
{
    pub extension: &'static str,
    pub extensions: &'static [&'static str],
    pub profile: &'static AnycubicMachineProfile,
    pub codec: C,       // 策略对象组合
    pub layout: L,      // 策略对象组合
}
```

- `FileMetadata` 通过组合 6 个子结构体替代单一大结构体
- `LayerEncodeMetadata<'a>` 通过借用组合替代数据复制

#### TypeScript 侧

- `FormatDescriptor = FormatFacts & Partial<FormatAdapter>` — 交叉类型组合
- Hook 组合替代类继承：`useConverterController` 组合 4 个子 controller
- `getFormatDescriptor()` 通过 spread 合并 Facts 和 Adapter：

```typescript
export function getFormatDescriptor(format: string): FormatDescriptor {
    return { ...getFormatFacts(format), ...getFormatAdapter(format) };
}
```

- 无任何 `class extends` 使用

---

### 6. 面向接口编程 — 4.5分

#### Rust 侧（优秀）

三层 trait 抽象形成清晰的合约体系：

| Trait                    | 职责                  | 实现者                         |
| ------------------------ | --------------------- | ------------------------------ |
| `SlicerFormat`           | 格式编解码全生命周期  | 5 个格式 + AnycubicFormat<C,L> |
| `FormatRegistry`         | 格式发现与查找        | BuiltinRegistry                |
| `AnycubicLayerCodec`     | Anycubic 编解码策略   | Pw0Codec, PwsCodec 等          |
| `AnycubicLayoutStrategy` | Anycubic 文件布局策略 | V1Layout, V516Layout 等        |

`FormatRegistry` trait 将注册表抽象化，`BuiltinRegistry` 只是一种实现，为测试注入 mock 提供了可能：

```rust
pub trait FormatRegistry: Sync {
    fn by_name(&self, name: &str) -> Option<&'static dyn SlicerFormat>;
    fn by_extension(&self, extension: &str) -> Option<&'static dyn SlicerFormat>;
    fn detect(&self, data: &[u8], extension_hint: Option<&str>) -> Option<&'static dyn SlicerFormat>;
    fn all(&self) -> &'static [FormatDesc];
}
```

#### TypeScript 侧（较上一版显著改善）

- `OutputSink` 接口仍保持优秀的策略模式设计，调用者无需感知 Blob / stream 两条落盘路径。
- `FormatAdapter` 已拆成 `documentOpener` 与 `zipOutput` 两个能力块，接口隔离问题基本被修复。
- `createWorkerRuntime(deps)` 通过 `WorkerRuntimeDeps` 注入依赖，再把 preview / conversion handlers 组合成 handler map，worker 内部也遵循了面向接口编程。

当前残留问题已经不再是“接口太大”，而是：

1. `FormatFacts` 与 adapter capability 的一致性还缺少自动校验。
2. `KNOWN_FORMAT_INFOS` 仍是浏览器侧预热 fallback，尚未完全退场。

因此，这一维度从 4 分上调到 4.5 分。下一步更合理的优化目标不是继续拆接口，而是建立“格式事实 + adapter 能力”的一致性校验与单一事实源。

---

## 二、工程化维度评估

### 7. 测试覆盖与策略 — 4.5分

#### 测试层次

| 层级                   | 框架                               | 文件数 | 覆盖重点                                               |
| ---------------------- | ---------------------------------- | ------ | ------------------------------------------------------ |
| Rust 核心 / 浏览器验证 | `cargo test` + `wasm-bindgen-test` | 9      | 6 种格式互转、Anycubic 档案矩阵、WASM browser contract |
| WASM 单元测试          | Vitest                             | 5      | 接口绑定、header 解析                                  |
| 前端单元测试           | Vitest + React Testing Library     | 32     | controller、session、registry、ZIP、组件               |
| E2E 测试               | Playwright                         | 3      | fixture 上传、格式转换下载、预览交互                   |
| 前端 benchmark         | Playwright bench                   | 1      | ZIP streaming 端到端采样                               |

**总计**: 约 50 个测试/验证入口文件

#### 测试命令全景

```bash
pnpm test              # typecheck + rust + wasm:unit + app (快速 CI 门控)
pnpm test:full         # test + browser (完整套件)
pnpm test:rust         # cargo test
pnpm test:app          # Vitest 前端单元 (32 文件)
pnpm test:wasm:unit    # WASM 包单元 (5 文件)
pnpm test:app:e2e      # Playwright 浏览器 (3 spec)
pnpm benchmark:rust    # Criterion (codec + preview)
pnpm bench:app         # Playwright bench (zip-streaming)
```

#### Mock 隔离策略

Worker 和 WASM 调用都有完整 mock 实现：

```typescript
// FakeZipWorker: 完整模拟 Worker 行为
class FakeZipWorker extends EventTarget {
    busy = false;
    calls = 0;
    postedNames: string[] = [];
    // ...
}

vi.mock("@innovgame/slicer-converter", async () => {
    const actual = await vi.importActual<typeof SlicerWasmModule>("@innovgame/slicer-converter");
    return { ...actual, crc32_bytes: mocks.crc32Bytes };
});
```

#### 增量亮点

- 新增 `preview-session.test.ts`，验证 raw / zip 两种预览派发负载的元数据保真。
- `format-registry.test.ts` 已覆盖 `documentOpener` / `zipOutput` 两个显式角色，以及 alias / extension / third-party adapter 注册路径。
- `opened-document.test.ts`、`runtime-metrics.test.ts`、`conversion-session.test.ts` 等让“会话边界”和“热路径指标”不再依赖 controller 级别大测试兜底。

#### 不足

- CI 仅跑 Ubuntu + Chrome，无 Firefox/Safari 覆盖
- 无代码覆盖率报告集成（c8/nyc）
- 大文件测试依赖外部磁盘样本，CI 中 skip
- 无网络降级模拟

---

### 8. 构建与部署流程 — 4.5分

#### CI 流水线

```
┌────────────────────────────────────────────────────────────┐
│                    GitHub Actions CI                        │
├────────────────────────────────────────────────────────────┤
│ JOB 1: check (gate)                                       │
│  → typecheck → cargo test → wasm:unit → app unit → bench  │
│                                                            │
│ JOB 2: lint (parallel)                                     │
│  → ESLint → Prettier format check                          │
│                                                            │
│ JOB 3: wasm (depends on check)                            │
│  → wasm-pack test --node                                   │
│                                                            │
│ JOB 4: e2e (depends on check)                             │
│  → build WASM → Playwright chromium                        │
│                                                            │
│ BONUS: rust-bench (non-blocking, continue-on-error)       │
└────────────────────────────────────────────────────────────┘
```

#### Rust Release 构建配置

```toml
[profile.release]
opt-level = 3        # 最高优化级别
lto = true           # 链接时优化
codegen-units = 1    # 单编译单元以获得更好 LTO
strip = true         # 去除调试符号
```

#### 关键特性

- **并发控制**: 同 ref 自动取消进行中的旧运行
- **缓存策略**: Cargo registry/git/target + pnpm store 独立缓存
- **Deploy**: GitHub Pages 手动触发，独立 `deploy.yml` workflow
- **可复现性**: `pnpm install --frozen-lockfile`

---

### 9. 文档完整度 — 4分

#### 文档清单

| 文档                                                          | 类型     | 状态                              |
| ------------------------------------------------------------- | -------- | --------------------------------- |
| `README.md` — 快速入门 + API + CI + Benchmark                 | 入门/API | ✅ 已覆盖开发、测试、CI、核心 API |
| `docs/adding-a-new-format.md` — 扩展指南                      | 开发     | ✅ Rust + TS 双侧步骤             |
| `docs/testing-strategy-spec.md` — 测试规范                    | 规范     | ✅ 四层策略详述                   |
| `docs/architecture-evolution-plan.md` + follow-up enhancement | 架构     | ✅ 近期演进计划较完整             |
| `docs/refactor-extensible-registry.md`                        | 重构     | ✅ 浏览器侧 registry 演进参考     |
| `docs/perf/t3.1-zip-streaming-protocol.md`                    | 性能     | ✅ 基准口径与协议细节齐全         |
| `docs/architecture-diagrams.md`                               | 可视化   | ✅ 已补充模块关系与数据流图       |

#### 缺失文档

1. **缺故障排查 / FAQ** — 常见编译、ChromeDriver、wasm-pack、样本生成问题仍未集中沉淀
2. **缺部署 runbook** — 除 GitHub Pages 之外，自托管/静态托管步骤仍然分散
3. **缺贡献者约定** — 模块边界、测试基线、命名约束目前更多散落在实现和计划文档中
4. **README 的 API 表仍偏高层** — browser facade、worker protocol、adapter capability 还没有更细粒度 reference

---

### 10. 依赖健康度 — 4.5分

#### Rust 依赖

| Crate                  | 版本 | 用途          | 状态                            |
| ---------------------- | ---- | ------------- | ------------------------------- |
| `wasm-bindgen`         | 0.2  | JS ↔ WASM FFI | ✅ 业界标准                     |
| `serde` + `serde_json` | 1    | 序列化        | ✅ 业界标准                     |
| `binrw`                | 0.15 | 二进制解析    | ✅ 活跃维护，零拷贝友好         |
| `png`                  | 0.18 | PNG 编解码    | ✅ 维护中                       |
| `flate2`               | 1    | Deflate 压缩  | ✅ 标准库                       |
| `zip`                  | 8    | ZIP 处理      | ✅ 禁用默认 feature，仅 deflate |
| `crc32fast`            | 1    | CRC32 校验    | ✅ 性能优化版                   |
| `thiserror`            | 2    | 错误处理派生  | ✅ 最新主版本                   |

#### 前端依赖

| 依赖         | 版本    | 状态                        |
| ------------ | ------- | --------------------------- |
| React        | ^19.2.5 | ✅ 最新稳定版               |
| Vite         | ^8.0.3  | ✅ 最新                     |
| Tailwind CSS | ^4.2.2  | ✅ 最新                     |
| Playwright   | ^1.59.1 | ✅ 近期                     |
| Vitest       | ^4.1.4  | ✅ 稳定                     |
| TypeScript   | ^6.0.2  | ✅ 新主版本，需留意生态兼容 |
| pnpm         | 10.33.0 | ✅ 当前                     |
| Node.js      | 22      | ✅ LTS                      |

**当前判断（基于版本、维护状态、CI 配置与依赖结构的推断）**: 依赖栈整体健康，暂未看到明显红旗；需要持续关注的主要是 TypeScript 6.x 与周边工具链的兼容节奏，而不是已确认的安全或维护风险。

---

### 11. 代码规范与 lint — 4.5分

#### ESLint 配置 (`eslint.config.mjs`)

```javascript
eqeqeq: ["error", "always"]                          // 强制全等
no-var: "error"                                       // 禁止 var
prefer-const: "warn"                                  // 推荐 const
@typescript-eslint/no-unused-vars: {                  // 允许 _ 前缀忽略
    argsIgnorePattern: "^_",
    varsIgnorePattern: "^_"
}
@typescript-eslint/no-explicit-any: "warn"            // 限制 any
@typescript-eslint/consistent-type-imports: "error"   // 类型导入一致性
```

#### TypeScript 配置 (`tsconfig.base.json`)

```json
{
    "target": "ES2024",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "verbatimModuleSyntax": true
}
```

#### Prettier

- 仓库已显式提供 `.prettierrc`
- 当前约束包括：`tabWidth: 4`、`printWidth: 120`、`trailingComma: none`、`semi: true`
- CI 通过 `pnpm format:check` 强制，且根脚本提供 `pnpm check:fix`

**当前不足**: JavaScript/TypeScript 侧规范已经较完整，但 Rust 侧仍未在根 CI 中加入 `cargo fmt --check` 或 `cargo clippy` 作为同等级门禁。

---

### 12. 错误处理与健壮性 — 3分

#### Rust 侧（良好）

统一 `ConvertError` 枚举 + `thiserror` 派生，运行时主路径基本统一为 `Result`；但当前仍存在少量编译期/守卫式 `panic!`，例如 Anycubic profile 的 const 查找守卫：

```rust
pub const fn profile_by_extension_const(extension: &str) -> &'static AnycubicMachineProfile {
    // ...
    panic!("profile_by_extension_const: unknown extension")
}
```

核心错误模型仍然集中在：

```rust
#[derive(Error, Debug)]
pub enum ConvertError {
    InvalidMagic { expected: u32, actual: u32 },
    UnsupportedVersion(u32),
    UnknownFormat,
    BufferTooSmall { need: usize, got: usize },
    RleDecodeError { offset: usize, detail: String },
    PngError(String),
    ZipError(String),
    IoError(#[from] std::io::Error),
    BinrwError(String),
    NoMatchingAnycubicProfile { ... },
    MetadataValidation(String),
    // ...
}
```

#### TypeScript 侧（需改进）

| 问题                    | 严重度 | 说明                                                       |
| ----------------------- | ------ | ---------------------------------------------------------- |
| Worker 错误无重试机制   | 🔴 高  | 单层失败后无回退/重试                                      |
| 错误消息丢失上下文      | 🟡 中  | `getErrorMessage(error)` 只保留 message，丢失 stack + 层号 |
| WASM 错误为自由字符串   | 🟡 中  | JS 侧无法按类型匹配 WASM 错误                              |
| 无结构化日志            | 🟡 中  | 错误发送到主线程但无持久化                                 |
| 验证错误与 I/O 错误混合 | 🟠 低  | 用户输入错误和系统错误未区分                               |

#### 亮点

文件解析有 `parseDocumentSourceWithWasmRetry()` — 512KB 头部不够时自动扩大重试，最大到 32MB，这是优秀的渐进式错误恢复设计。另一个进步是 worker runtime 已拆成 conversion / preview handlers，局部失败更容易定位和测试。

#### 改进建议

1. **添加 `WorkerErrorContext`** 包装错误，携带层号、格式、时间戳
2. **实现可重试错误的 exponential backoff**
3. **分离验证错误和运行时错误** 到不同类型
4. **WASM 侧导出结构化错误码** 而非自由文本
5. **添加结构化日志** 支持错误码（如 `E001: InvalidMagic`）

---

## 三、评分总览

| 维度             | 评分        | 关键亮点                                            | 主要不足                              |
| ---------------- | ----------- | --------------------------------------------------- | ------------------------------------- |
| **易扩展性**     | ⭐⭐⭐⭐    | Rust trait 注册稳定，TS adapter 已职责分组          | 格式事实仍跨语言重复                  |
| **性能**         | ⭐⭐⭐⭐⭐  | SIMD, buffer 池, 批量 API, 流式 ZIP                 | —                                     |
| **面向对象封装** | ⭐⭐⭐⭐½   | session/catalog 边界清晰，会话对象职责更明确        | 错误模型尚未同步封装                  |
| **多态**         | ⭐⭐⭐⭐⭐  | 编译期 + 运行时多态恰当选用                         | —                                     |
| **组合优于继承** | ⭐⭐⭐⭐⭐  | 零继承，全面组合                                    | —                                     |
| **面向接口编程** | ⭐⭐⭐⭐½   | `DocumentOpener` + `ZipOutputPolicy` + `OutputSink` | 格式事实 / adapter 能力一致性校验缺失 |
| **测试覆盖**     | ⭐⭐⭐⭐½   | 新增 session / catalog / document 边界测试          | 无覆盖率报告，仅单浏览器              |
| **构建与部署**   | ⭐⭐⭐⭐½   | 成熟 CI, 缓存策略, Release 优化                     | —                                     |
| **文档完整度**   | ⭐⭐⭐⭐    | README 已覆盖 API/CI/Benchmark，架构文档更完整      | 缺 FAQ、部署 runbook                  |
| **依赖健康度**   | ⭐⭐⭐⭐½   | 依赖栈整体健康（基于当前仓库状态的推断）            | 需持续关注 TS 6.x 生态兼容            |
| **代码规范**     | ⭐⭐⭐⭐½   | ESLint 10 + strict TS + 显式 Prettier 配置          | Rust fmt/clippy 未入根门禁            |
| **错误处理**     | ⭐⭐⭐      | Rust 统一 enum 优秀，worker 边界已拆分              | TS Worker 层缺重试/上下文             |
| **综合**         | **4.4 / 5** | 前端边界与文档完成了一轮有效收口                    | 错误模型与单一事实源仍待补齐          |

---

## 四、改进建议优先级

### P0 — 高优先级

1. **TS 错误处理健壮性**: Worker 错误添加上下文（层号、格式）+ 可重试错误的重试机制
2. **WASM 错误结构化**: 从自由字符串改为带错误码的结构化格式

### P1 — 中优先级

3. **消除 TS 硬编码注册表**: `KNOWN_FORMAT_INFOS` 完全由 WASM `list_supported_formats()` 驱动，并为 facts / adapter capability 建立一致性校验
4. **集成覆盖率报告**: c8/nyc 生成覆盖率 HTML，CI 门控覆盖率下降
5. **补齐 Rust 静态门禁**: 将 `cargo fmt --check` 与 `cargo clippy` 纳入根 CI

### P2 — 低优先级

6. **补全文档**: 故障排查指南、FAQ、自托管/静态部署 runbook
7. **多浏览器 E2E**: Playwright 扩展至 Firefox/Safari
8. **Anycubic Codec-Layout 配对约束**: 类型系统编码合法组合，防止运行时配对错误
9. **扩展 browser facade reference**: 细化 README 中的 API 表与 worker protocol / adapter capability 文档
