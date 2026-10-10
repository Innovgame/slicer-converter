# slicer-converter v3 断裂式架构演进规划

> 版本：v3 (breaking — 不考虑向后兼容、老版本回退、渐进灰度)
> 起草日期：2026-04-18
> 适用范围：Rust 内核 + WASM 边界 + 前端 React/TS 应用全栈
> 设计取向：**易拓展性 / 性能 / 面向对象封装 / 多态 / 组合优于继承 / 面向接口编程**
> 取舍前提：接口可以**直接断裂式替换**，旧实现一次性删除，不保留 shim/alias
>
> 状态：保留为 2026-04-18 的审计/草案参考；当前执行与后续维护以 [`docs/architecture-evolution-plan.md`](./architecture-evolution-plan.md) 为准。
> 保留原因：其中的痛点清单、测试与性能守护思路仍有参考价值，但其目标架构与阶段设计存在较重的概念堆叠，不作为当前实施基线。

---

## 0. 文档定位

本文档是 v2 架构改进收口（参见 [architecture-improvement-plan.md](architecture-improvement-plan.md)）之后的**下一代规划**。v2 解决了"实现走样、测试薄弱、命名分散"的存量债务；v3 在 v2 干净基线上做**结构性跃迁**——把过去因兼容性而妥协的设计一次性改齐。

v2 与 v3 的关系：

- v2 = **同形态优化**（不破坏 API，重构内部）
- v3 = **断裂式重塑**（重新定义 API、模块边界、调用契约）

执行时机：v2 全部 milestones ✅ 已完成；当前 master `ec0468b` + 本会话补做的 8 commit 形成稳定基线，可作为 v3 起点。

---

## 1. 设计原则与判据

### 1.1 六维优先级

| 维度             | 操作化判据（"达成"的可观测信号）                                                                         |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| **易拓展性**     | 新增格式/编解码/Sink/UI 组件 ≤ 1 个新文件，零中央编辑（仅 `register*()` 调用）                           |
| **性能**         | 关键路径无非必要 clone；批转码无 per-layer 分配峰值；JS↔WASM 边界传零拷贝 view                           |
| **面向对象封装** | 单一 hook/struct ≤ 300 行；无外部直接读 `_internal` 字段；状态变更必经方法                               |
| **多态**         | 格式/编解码/Sink dispatch 全部走 trait/接口，禁止 `match enum` 或 `=== "string"` 业务分支                |
| **组合优于继承** | Anycubic 的 `<C, L>` 编译期展开 → 运行时 `dyn Codec + dyn Layout` 注入；FileMetadata 拆为 concern slices |
| **面向接口编程** | TS 端 import 全部对接 `interface`/`type`，禁止 import concrete class；WASM 边界由 `WasmApi` trait 统一   |

### 1.2 拒绝清单（明确**不做**的事）

- ❌ 不做向后兼容垫片。旧 `decode_layer(...)` 旧签名一次性删除。
- ❌ 不做渐进式灰度。`pnpm typecheck` 红→绿是单次切换。
- ❌ 不做格式弃用周期。某格式如果在 v3 重构中暴露设计冲突，直接重构其行为，不为旧文件保留 fallback。
- ❌ 不引入运行时反射框架（如 InversifyJS）。依赖注入靠**显式构造函数 + 模块化 register**。
- ❌ 不做"功能旗标"开关。要么全切，要么不切。

---

## 2. 现状痛点摘要（来自审计）

> 详细位置见审计报告附录 §A。此处只列**进入 v3 必须处理**的高杠杆痛点。

| ID  | 位置                                                                                               | 跨轴杠杆       | 痛点摘要                                                                   |
| --- | -------------------------------------------------------------------------------------------------- | -------------- | -------------------------------------------------------------------------- |
| H1  | `apps/slicer-app/src/app/hooks/use-converter-controller.ts` (1100+ 行)                             | 封装/多态/接口 | 神 hook，10 useState + 11 useRef，6 个嵌套函数共享 ref，无法独立测试子能力 |
| H2  | `rust/src/formats/anycubic/mod.rs` (21 个 `anycubic_format!` 宏 + `<C, L>` 泛型)                   | 拓展/多态/组合 | 编译期 monomorphization 爆炸；新增 codec/layout 维度时 N×M 增长            |
| H3  | `rust/src/lib.rs` decode/encode `Vec::clone()` 链 + 分散的 `thread_local` pool                     | 性能/封装      | 高分辨率多层批转每层多走一次内存拷贝；缓冲池硬编码不可换策略               |
| H4  | `rust/src/lib.rs` 14 个 `wasm_bindgen` 函数（`Vec<u8>` / `String JSON` 边界）                      | 接口/拓展      | WASM 边界无 trait 抽象，TS 端依赖具体函数签名+魔法字节布局                 |
| H5  | `rust/src/formats/mod.rs` `LayerOptions` 包含 `anti_alias_level` / `encryption_key` 等格式特化字段 | 封装/接口      | 违反 ISP，调用方必须理解每个格式的特化参数                                 |
| H6  | `apps/slicer-app/src/core/formats/*-plugin.ts` 当前的 `FormatPlugin` 钩子（v2 已落地）多为可选     | 拓展/多态      | 钩子可选性导致行为分散在 plugin + WASM + worker 三处                       |
| H7  | `apps/slicer-app/src/core/file-loader.ts` `ParsedFileState` 用 nullable 字段表达多源               | 封装/接口      | `fileBytes \| zipEntries \| zipSource` 应为 sealed enum                    |
| H8  | `targetFormat === "anycubic-manual"` 类的字符串比较 dispatch                                       | 多态/接口      | 散落于 anycubic.ts、use-converter-controller.ts、format-registry.ts        |
| H9  | `FileMetadata` 50+ 字段 packed struct                                                              | 封装           | 字段对各格式语义不同，无类型层路由                                         |
| H10 | Worker handler list 与 `DispatchableRequest` 联合手工对齐                                          | 多态/拓展      | 新增请求类型需双向编辑                                                     |

---

## 3. v3 目标架构（Block Diagram）

### 3.1 分层

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5: Presentation (React Components, pure, props-in)        │
│   - app/components/* (presentation only, zero side effects)     │
├─────────────────────────────────────────────────────────────────┤
│ Layer 4: Application Controllers (interface-driven hooks)       │
│   - PreviewController       (interface: IPreviewController)     │
│   - ConversionController    (interface: IConversionController)  │
│   - BenchmarkController     (interface: IBenchmarkController)   │
│   - SessionController       (源文件状态机, 持有 FileSource enum) │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3: Domain Services (concrete, behind interfaces)          │
│   - FormatRegistry         (IFormatRegistry)                    │
│   - PreviewService         (IPreviewService)                    │
│   - ConversionPipeline     (IConversionPipeline)                │
│   - WorkerScheduler        (IWorkerScheduler)                   │
│   - SinkFactory            (ISinkFactory)                       │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2: WASM Facade (single seam to Rust)                      │
│   - IWasmApi 接口 + WasmApiClient 实现 (only file allowed       │
│     to call into wasm-bindgen exports)                          │
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: Rust Core                                              │
│   - SlicerFormat trait (existing, polish)                       │
│   - LayerCodec / LayoutStrategy trait objects                   │
│   - BufferPool trait + 实现 (replace thread_local)              │
│   - WasmApiV3 (替换 lib.rs，统一进出口 + 零拷贝 view)           │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 关键接口契约（伪代码）

#### Rust 端

```rust
// 格式 trait（已存在，仅 polish）
pub trait SlicerFormat: Send + Sync {
    fn identity(&self) -> &FormatIdentity;
    fn decode_layer(&self, ctx: &LayerDecodeContext, dst: &mut [u8]) -> Result<DecodedLayer>;
    fn encode_layer(&self, ctx: &LayerEncodeContext, dst: &mut Vec<u8>) -> Result<EncodedLayer>;
    fn build_header(&self, plan: &EncodePlan, dst: &mut Vec<u8>) -> Result<HeaderLayout>;
    fn finalize_header(&self, layout: &HeaderLayout, layers: &[LayerEntry], dst: &mut Vec<u8>);
    fn try_detect(&self, head: &[u8]) -> Option<DetectionConfidence>;
}

// 解决 H2：Anycubic 改为 trait object
pub trait AnycubicLayerCodec: Send + Sync { ... }
pub trait AnycubicLayoutStrategy: Send + Sync { ... }

pub struct AnycubicFormat {
    identity: FormatIdentity,
    codec: Arc<dyn AnycubicLayerCodec>,
    layout: Arc<dyn AnycubicLayoutStrategy>,
    profile: AnycubicProfile,
}
// 注册 21 个变种 = 21 行 Builder 调用，无宏

// 解决 H3：BufferPool trait
pub trait BufferPool: Send + Sync {
    fn acquire(&self, hint: usize) -> PooledBuffer<'_>;
}
pub struct ThreadLocalPool;       // 当前默认
pub struct UnpooledBuffers;       // 测试 / 单层场景
// 未来可加 ArenaPool、OffHeapPool

// 解决 H5：LayerOptions 拆分为 per-format 参数包
pub struct CtbLayerOptions   { encryption_key: u32, anti_alias_level: u32 }
pub struct GooLayerOptions   { /* 空 */ }
pub struct VdtLayerOptions   { /* 空 */ }
pub trait FormatLayerOptions { fn into_decode_ctx(self) -> LayerDecodeContext; }
// SlicerFormat::Options 关联类型携带，编译期保证不混用

// 解决 H4：WASM 单一接口
#[wasm_bindgen]
pub struct WasmApiV3 { /* ... */ }

#[wasm_bindgen]
impl WasmApiV3 {
    pub fn list_formats(&self) -> JsValue;          // 直接返回结构化对象，非 JSON 字符串
    pub fn parse_header(&self, view: &Uint8Array) -> ParsedHeaderHandle;  // 句柄
    pub fn decode_layer(&self, handle: &LayerHandle, dst: &mut Uint8Array);
    pub fn encode_layer(&self, plan: &EncodePlanHandle, dst: &mut Uint8Array);
    // 句柄模式：JS 持有不透明 ID，避免每次序列化整个状态
}
```

#### TypeScript 端

```typescript
// 解决 H4：单一 WASM 边界 interface
export interface IWasmApi {
  listFormats(): readonly FormatDescriptor[];
  parseHeader(view: ArrayBufferView): ParsedHeader;
  decodeLayer(req: LayerDecodeRequest): DecodedLayer;     // 内部走句柄
  encodeLayer(req: LayerEncodeRequest): EncodedLayer;
  // 全部签名在 IWasmApi 中声明；wasm-api-client.ts 是唯一实现
}

// 解决 H1：拆分神 hook
export interface IPreviewController {
  readonly state: PreviewState;
  selectLayer(index: number): void;
  invalidate(): void;
  dispose(): void;
}
export interface IConversionController {
  readonly state: ConversionState;     // discriminated union
  start(plan: ConversionPlan): Promise<ConversionResult>;
  cancel(): void;
}
export interface IBenchmarkController { ... }

// 解决 H7：FileSource 用 sealed enum
export type FileSource =
  | { kind: "raw"; bytes: Uint8Array }
  | { kind: "zipBytes"; entries: ZipEntryView[] }
  | { kind: "zipStream"; reader: AsyncIterable<ZipEntryChunk> };

// 解决 H8：Anycubic 特化走 Strategy 而非字符串分支
export interface IAnycubicExportStrategy {
  resolveTarget(source: FormatDescriptor, hint: AnycubicHint): FormatDescriptor;
  buildLayout(plan: EncodePlan): AnycubicLayoutPlan;
}
// 由 anycubic-plugin.ts 实现并注入 ConversionPipeline

// 解决 H10：worker handler 自动派发
type WorkerRequestRegistry = {
  [K in WorkerRequestKind]: (req: WorkerRequest<K>) => Promise<WorkerResponse<K>>;
};
// 用 mapped type + WorkerRequestKind 联合 → 缺一个 handler 编译报错
```

### 3.3 核心模块边界

| 模块                        | 输入接口                                              | 输出接口                            | 不依赖                      |
| --------------------------- | ----------------------------------------------------- | ----------------------------------- | --------------------------- |
| `core/wasm-api/`            | (none)                                                | `IWasmApi`                          | 任何 UI / hook              |
| `core/format-registry/`     | `IWasmApi`, `FormatPlugin[]`                          | `IFormatRegistry`                   | DOM / React                 |
| `core/preview-service/`     | `IWasmApi`, `IFormatRegistry`, `IBufferCache`         | `IPreviewService`                   | DOM                         |
| `core/conversion-pipeline/` | `IFormatRegistry`, `IWorkerScheduler`, `ISinkFactory` | `IConversionPipeline`               | DOM                         |
| `core/sinks/`               | (none)                                                | `ISinkFactory` (含 capability 探测) | conversion-pipeline         |
| `core/worker-scheduler/`    | (none)                                                | `IWorkerScheduler`                  | conversion-pipeline         |
| `app/controllers/`          | 所有上面 services                                     | React hooks (`useXxxController`)    | 直接 import 具体 service 类 |
| `app/components/`           | controllers via props                                 | React 元素                          | services / WASM             |

**关键约束**：每一行 import 只能依赖**接口**，具体类只在 composition root（`app/composition.ts`）一处实例化。

---

## 4. 断裂式迁移阶段

> 不存在并行新旧路径。每个阶段是 PR-sized 单元，但 PR 内一次替换完。

### Phase v3.1 — Rust 内核重塑（解 H2 / H3 / H5）

| 子任务 | 动作                                                                                                                                                                         | 验收                                                                                |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 3.1.1  | 引入 `BufferPool` trait + `ThreadLocalPool` 默认实现，删除 `lib.rs` 中 5 个独立 `thread_local` 块                                                                            | `cargo test` 全绿；`cargo bench` codec_bench 持平或更优                             |
| 3.1.2  | `LayerOptions` 拆为 per-format `Options` 关联类型，删除中央 struct 中的 `encryption_key` / `anti_alias_level` 字段                                                           | GOO/SL1 不再传无意义字段；CTB/Anycubic 由 `Options` 携带                            |
| 3.1.3  | Anycubic `AnycubicFormat<C, L>` → `AnycubicFormat { codec: Arc<dyn ...>, layout: Arc<dyn ...> }`；删除 `anycubic_format!` 宏，21 个变种改为 `AnycubicFormatBuilder` 链式调用 | 二进制大小下降；`anycubic_*` 测试全绿；新增 codec/layout = 1 个 trait impl + 注册行 |
| 3.1.4  | 引入 `LayerDecodeContext::with_buffer_pool()`，所有 `decode_*` 函数接收 `&dyn BufferPool`                                                                                    | `clone()` 计数下降；新增 `unpooled` test 模式                                       |

### Phase v3.2 — WASM 边界统一（解 H4）

| 子任务 | 动作                                                                                                                  | 验收                                                            |
| ------ | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| 3.2.1  | 新建 `WasmApiV3` 单一 `#[wasm_bindgen] struct`，所有 14 个 free fn 收编为方法；旧 `decode_layer` 等导出**一次性删除** | `wasm_api_browser` 测试更新到新 API；TS 端只剩一处 import       |
| 3.2.2  | 引入句柄模式：`ParsedHeaderHandle`、`LayerHandle`、`EncodePlanHandle`，避免每次 JSON 序列化                           | `parse_header` JSON 字符串路径删除；微基准 `bench_inflate` 持平 |
| 3.2.3  | TS 端 `core/wasm-api/wasm-api-client.ts` 是**唯一**调用 `wasm-bindgen` 导出的文件；其余模块只 import `IWasmApi` 接口  | grep 全仓 `import.*slicer_wasm_bg` 仅 1 处                      |

### Phase v3.3 — TS 服务层重塑（解 H1 / H7 / H8）

| 子任务 | 动作                                                                                                                                                                                             | 验收                                                            |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| 3.3.1  | 拆分 `use-converter-controller.ts`（1100 行）为 `PreviewController` / `ConversionController` / `BenchmarkController` / `SessionController` 4 个独立 service + 4 个对应 thin hook；旧文件**删除** | 单文件 ≤ 300 行；每个 controller 有独立单测可不挂载 React       |
| 3.3.2  | `ParsedFileState` → `FileSource` discriminated union；调用方用 `switch (source.kind)` 代替 nullable 检查                                                                                         | 类型层禁止"raw + zipEntries 同时存在"等非法状态                 |
| 3.3.3  | `IAnycubicExportStrategy` 取代 `if (targetFormat === "anycubic-manual")` 的 4 处字符串比较                                                                                                       | grep `=== "anycubic-` 在业务代码中归零（仅注册表 key 比较保留） |
| 3.3.4  | `core/composition.ts` composition root：唯一可 `new XxxService()` 的地方；其余文件全部接口注入                                                                                                   | grep `new (Preview\|Conversion\|Benchmark)Service` 仅 1 处      |

### Phase v3.4 — Worker / Sink 多态收口（解 H6 / H10）

| 子任务 | 动作                                                                                                                                                      | 验收                                                                        |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| 3.4.1  | `WorkerRequestRegistry` mapped type 自动派发；删除手写 handler 列表                                                                                       | 新增 request 仅需扩 `WorkerRequestKind` 联合 + handler 函数；缺一个编译报错 |
| 3.4.2  | `FormatPlugin` 钩子从 optional 收紧为必需的 strategy slots（`HeaderStrategy`、`LayerStrategy`、`ZipLayoutStrategy`），缺省策略由 `defaultStrategies` 提供 | 行为不再分散在 plugin + WASM + worker；查 plugin 即知行为                   |
| 3.4.3  | `ISinkFactory.choose(capabilities)` 取代 `use-converter-controller` 内联 `openStreamingSink(...) ?? createBlobSink()`                                     | sink 选择策略可独立测试与替换（IndexedDB sink、S3 sink 等未来扩展点）       |

### Phase v3.5 — Presentation 纯化（解 H1 残留）

| 子任务 | 动作                                                                                                         | 验收                                                                         |
| ------ | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| 3.5.1  | `app/components/*` 全部改为 props-in、零副作用纯组件；hook 调用全部上提到 `app/pages/*`                      | 组件单测无需 mock service                                                    |
| 3.5.2  | 引入 `app/pages/converter-page.tsx` / `app/pages/benchmark-page.tsx`，作为 controllers ↔ components 的拼装层 | `converter-app.tsx` / `benchmark-app.tsx` 仅做 composition root + page mount |

---

## 5. 性能回归防御

| 基线指标（来自 v2 收口）                                 | v3 不可回退阈值                       |
| -------------------------------------------------------- | ------------------------------------- |
| GOO→VDT 1380 层 / 91 MB total 2.81 s                     | ≤ 2.95 s（+5%）                       |
| ZIP assemble 2.72 s                                      | ≤ 2.85 s                              |
| `finalizeOnlyWallMs` 30.3 ms                             | ≤ 50 ms                               |
| Peak heap 91.9 MB（输出 68.4 MB，1.34×）                 | ≤ 100 MB / ≤ 1.5×                     |
| WASM SIMD ctb_decode 633 MiB/s                           | ≥ 600 MiB/s                           |
| WASM SIMD ctb_encode 265 MiB/s                           | ≥ 250 MiB/s                           |
| WASM SIMD goo_encode 232 MiB/s                           | ≥ 220 MiB/s                           |
| PNG `Compression::Fast` + `Filter::Sub` + ZIP deflate(8) | **不变**（任何修改需重立基线 + 评审） |

每个 Phase 合并前必须跑 `pnpm --filter @slicer-converter/slicer-app run test:bench` 与 `cargo bench`，并把数字录入 `progress.md` 对应 phase 段落。

---

## 6. 测试策略调整

### 6.1 新增的测试形态

- **接口契约测试**：每个 `IXxx` interface 提供 `tests/contract/i-xxx.contract.ts`，所有实现复用同一组测试
- **句柄生命周期测试**：WasmApiV3 的句柄模式需要 `dispose` 测试（防内存泄漏）
- **Strategy 注入测试**：用假 `IAnycubicExportStrategy` 验证 dispatch 不再走字符串
- **Composition root 烟雾测试**：确认 `composition.ts` 装配后所有 service 可 `dispose()` 不抛

### 6.2 删除的测试形态

- ❌ 直接构造 `WorkerPool` 内部 `RefCell` 的白盒测试 → 改为通过 `IWorkerScheduler` 接口测
- ❌ 测试 `decode_layer` 的具体字节布局 → 改为测 `IWasmApi.decodeLayer` 的类型化输出

### 6.3 测试套件目标

| 套件      | v2 现状           | v3 目标                                          |
| --------- | ----------------- | ------------------------------------------------ |
| Rust unit | `cargo test` 全绿 | 持平                                             |
| TS unit   | 80 通过           | ≥ 100（新增 controller / service / contract 层） |
| TS e2e    | 16 通过           | 持平（无 UI 行为变更）                           |
| Bench     | 5 套基线          | 持平（指标维持）                                 |

---

## 7. 风险与不做之事

### 7.1 已识别风险

| 风险   | 触发条件                                                 | 缓解                                                                                                      |
| ------ | -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| R-V3-1 | Anycubic trait object 化引入虚函数开销                   | Phase 3.1.3 完成后立即跑 `cargo bench codec_bench`，超过 -3% 则改用 enum dispatch                         |
| R-V3-2 | 句柄模式增加 JS↔WASM 的 round-trip 次数                  | 保持单层 `decode_encode_layer` 热路径，优先通过 worker 并行、buffer 复用与 streamed output 降低端到端成本 |
| R-V3-3 | 服务层拆分后初始化顺序复杂                               | composition root 集中拼装 + 拓扑排序；提供 `dispose()` 链反向触发                                         |
| R-V3-4 | 删除 `LayerOptions` 中的格式特化字段会破坏现有 wasm 测试 | Phase 3.1.2 与 wasm 测试同 PR 更新                                                                        |

### 7.2 明确不做

- 不做插件热加载（运行时动态 register format from URL）
- 不做插件沙箱（不引入 Worker-per-format）
- 不做多语言后端切换（不引入 wasi 抽象层）
- 不做 React Server Components / SSR（前端是纯 SPA）
- 不引入 IoC 容器框架

---

## 8. 完成标准（DoD）

整个 v3 视为完成需同时满足：

1. ✅ 所有 Phase 3.1–3.5 子任务全部 ✅
2. ✅ `pnpm typecheck` / `cargo check` / `cargo clippy` 全绿
3. ✅ `pnpm -r test:unit` ≥ 100 通过
4. ✅ `pnpm --filter @slicer-converter/slicer-app run test:e2e` 16 通过
5. ✅ §5 性能阈值全部满足，bench 报告归档
6. ✅ grep 自检：
    - `grep -r 'thread_local!' rust/src/lib.rs` → 0 行
    - `grep -rn '=== "anycubic-' apps/slicer-app/src/` → 仅注册表 key 比较
    - `grep -rn 'anycubic_format!' rust/src/` → 0 行
    - `grep -rn 'import .* from .*slicer_wasm_bg' apps/slicer-app/` → 1 处
    - `wc -l apps/slicer-app/src/app/hooks/*.ts` → 每文件 ≤ 300
7. ✅ `docs/adding-a-new-format.md` 更新为 v3 strategy slot 描述
8. ✅ `progress.md` 记录每个 Phase 的 commit + bench 数字

---

## 9. 与既有计划的关系

| 计划                                               | 关系                                                               |
| -------------------------------------------------- | ------------------------------------------------------------------ |
| `architecture-improvement-plan.md`                 | v2，已收口，是 v3 起点。v3 不重复其内容。                          |
| `2026-04-15-slicer-app-react-wasm-refactor.md`     | v3 Phase 3.3 / 3.5 进一步拆分其遗留的 1100 行神 hook               |
| `2026-04-17-t3-1-tail-finalize-metric-and-sink.md` | v3 Phase 3.4.3 把其 `openStreamingSink` 升级为 `ISinkFactory` 多态 |
| `refactor-extensible-registry-plan.md`             | v3 Phase 3.4.2 在其 plugin 基础上收紧为强制 strategy slots         |
| `2026-04-14-performance-extensibility-roadmap.md`  | v3 §5 性能阈值与其指标基线一致                                     |

---

## 10. 附录 A：审计原始痛点定位

> 完整审计报告已被本规划吸收。以下保留**原文位置引用**便于审阅时定位。

- H1 神 hook：[apps/slicer-app/src/app/hooks/use-converter-controller.ts](../apps/slicer-app/src/app/hooks/use-converter-controller.ts)
- H2 Anycubic 宏展开：[rust/src/formats/anycubic/mod.rs](../rust/src/formats/anycubic/mod.rs) + `format.rs`
- H3 thread_local pools：[rust/src/lib.rs](../rust/src/lib.rs) (5 处 `thread_local!`)
- H4 WASM 14 函数：[rust/src/lib.rs](../rust/src/lib.rs) (`#[wasm_bindgen] pub fn`)
- H5 LayerOptions 中央化：[rust/src/formats/mod.rs](../rust/src/formats/mod.rs)
- H6 FormatPlugin 钩子可选：[apps/slicer-app/src/core/formats/plugin.ts](../apps/slicer-app/src/core/formats/plugin.ts)
- H7 ParsedFileState nullable：[apps/slicer-app/src/core/file-loader.ts](../apps/slicer-app/src/core/file-loader.ts)
- H8 字符串 dispatch：grep `=== "anycubic-"` in apps/slicer-app/src/
- H9 FileMetadata 50 字段：[rust/src/common/metadata.rs](../rust/src/common/metadata.rs)
- H10 Worker handler 列表：[apps/slicer-app/src/worker.ts](../apps/slicer-app/src/worker.ts)

---

**版本签名**：v3.0 draft, 2026-04-18
