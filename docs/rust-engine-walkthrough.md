# `engine.rs` 逐行讲解：Rust 语法 + 转码业务 + TS 类比

本文面向 Rust 新手，专门解释 [rust/src/transcode/engine.rs](../rust/src/transcode/engine.rs)。这个文件很短，但它是 Rust core 里“逐层转码”的调度中心。

推荐先读：

- [docs/rust-quickstart.md](./rust-quickstart.md)
- [docs/rust-syntax-notes.md](./rust-syntax-notes.md)
- [rust/src/transcode/engine.rs](../rust/src/transcode/engine.rs)

---

## 1. 先用一句话理解它

`TranscodeEngine` 做的事情是：

```text
给我源 FormatSpec、目标 FormatSpec、一层源数据和一些上下文；
我先从源/目标 FormatSpec 派生 route，再查有没有当前 route 的 fast path；
有 fast path 且能处理，就直接产出目标层 bytes；
没有命中，就走 fallback：source spec decode -> target spec encode。
```

如果用 TypeScript 心智看，它像一个 service：

```typescript
class TranscodeEngine {
    constructor(private registry: TranscodeRegistry) {}

    transcodeLayerIntoWithStats(srcFmt, tgtFmt, src, req, scratch, out) {
        const route = TranscodeRouteDescriptor.fromSpecs(srcFmt, tgtFmt);
        for (const transcoder of this.registry.lookupRoute(route.routeKey)) {
            const outcome = transcoder.tryTranscodeLayerInto(src, req, scratch, out);
            if (outcome === "applied") {
                return stats;
            }
        }

        fallbackTranscodeLayerWithFormats(srcFmt, tgtFmt, src, req, scratch, out);
        return stats;
    }
}
```

Rust 版本的额外复杂度主要来自：

- `Result` / `?`：错误处理
- `&dyn Trait`：registry 仍用运行时多态
- `&FormatSpec`：源/目标格式 profile 的只读借用
- `&[u8]` / `&mut Vec<u8>`：借用和 buffer 复用
- `static` / `'static`：全局注册表和全局 engine

---

## 2. 完整源码

当前 `rust/src/transcode/engine.rs`：

```rust
use crate::{common::ConvertError, formats::common::FormatSpec};

use super::{
    fallback,
    registry::{TranscodeRegistry, DEFAULT_REGISTRY},
    FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscodeStats,
    TranscodeRouteDescriptor,
};

pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}

impl TranscodeEngine {
    pub const fn new(registry: &'static dyn TranscodeRegistry) -> Self {
        Self { registry }
    }

    pub fn transcode_layer_into(
        &self,
        src_fmt: &FormatSpec,
        tgt_fmt: &FormatSpec,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        self.transcode_layer_into_with_stats(src_fmt, tgt_fmt, src, req, scratch, out)
            .map(|_| ())
    }

    pub fn transcode_layer_into_with_stats(
        &self,
        src_fmt: &FormatSpec,
        tgt_fmt: &FormatSpec,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<LayerTranscodeStats, ConvertError> {
        let mut stats = LayerTranscodeStats::default();
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
    }
}

static DEFAULT_ENGINE: TranscodeEngine = TranscodeEngine::new(&DEFAULT_REGISTRY);

pub fn default_engine() -> &'static TranscodeEngine {
    &DEFAULT_ENGINE
}
```

---

## 3. imports：这个文件依赖谁

```rust
use crate::{common::ConvertError, formats::common::FormatSpec};
```

意思是从当前 crate 根部导入两个东西：

| 名字           | 来源                              | 作用                              |
| -------------- | --------------------------------- | --------------------------------- |
| `ConvertError` | `rust/src/common/error.rs`        | Rust core 统一错误类型            |
| `FormatSpec`   | `rust/src/formats/common/spec.rs` | 单个格式 profile 的事实和执行内核 |

TypeScript 类比：

```typescript
import { ConvertError } from "../common/error";
import type { FormatSpec } from "../formats/common/spec";
```

下一段：

```rust
use super::{
    fallback,
    registry::{TranscodeRegistry, DEFAULT_REGISTRY},
    FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscodeStats,
};
```

`super` 表示父模块，也就是 `transcode` 模块。这里引入的是转码层内部组件：

| 名字                       | 作用                                               |
| -------------------------- | -------------------------------------------------- |
| `fallback`                 | 通用转码路径：decode -> encode                     |
| `TranscodeRegistry`        | fast path 注册表接口                               |
| `DEFAULT_REGISTRY`         | 默认 fast path 注册表实例                          |
| `FastPathOutcome`          | fast path 尝试结果：`Applied` 或 `Fallback`        |
| `LayerTranscodeRequest`    | 一层转码所需的请求上下文                           |
| `LayerTranscodeScratch`    | 可复用临时 buffer                                  |
| `LayerTranscodeStats`      | 单层转码统计                                       |
| `TranscodeRouteDescriptor` | 从源/目标 `FormatSpec` 派生出来的转换级 route 事实 |

这里能看出一个分层：

```text
formats/FormatSpec 负责“单格式 profile 能力”
transcode/engine 负责“跨格式编排”
transcode/execute 负责“单层执行 facade：组装 request、复用 scratch、调用 engine”
transcode/registry 负责“查找 pairwise fast path”
transcode/fallback 负责“没有 fast path 时兜底”
```

---

## 4. `TranscodeEngine` 结构体

```rust
pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}
```

它只有一个字段：`registry`。

逐段拆开：

```text
pub struct TranscodeEngine
```

定义一个公开结构体。

```text
registry:
```

字段名。

```text
&'static dyn TranscodeRegistry
```

字段类型，拆开：

| 片段                    | 含义                                                        |
| ----------------------- | ----------------------------------------------------------- |
| `&`                     | 借用，不拥有这个 registry                                   |
| `'static`               | 引用指向的对象活得足够久，通常是全局静态对象                |
| `dyn TranscodeRegistry` | 一个实现了 `TranscodeRegistry` trait 的对象，运行时动态分发 |

TypeScript 类比：

```typescript
class TranscodeEngine {
    constructor(private registry: TranscodeRegistry) {}
}
```

但 Rust 写得更精确：它明确说 `registry` 是借来的、全局可用的、通过 trait object 调用。

---

## 5. `impl TranscodeEngine`

```rust
impl TranscodeEngine {
    pub const fn new(registry: &'static dyn TranscodeRegistry) -> Self {
        Self { registry }
    }
}
```

`impl TranscodeEngine` 表示“给 `TranscodeEngine` 实现方法”。

### `new`

```rust
pub const fn new(registry: &'static dyn TranscodeRegistry) -> Self
```

读法：

```text
公开的构造函数 new
参数是一个全局生命周期的 TranscodeRegistry trait object 引用
返回 Self，也就是 TranscodeEngine
```

函数体：

```rust
Self { registry }
```

结构体初始化简写，等价于：

```rust
TranscodeEngine {
    registry: registry,
}
```

为什么是 `const fn`？

因为后面要初始化全局静态对象：

```rust
static DEFAULT_ENGINE: TranscodeEngine = TranscodeEngine::new(&DEFAULT_REGISTRY);
```

普通函数不能随便用于 `static` 初始化，`const fn` 可以在编译期常量上下文使用。

---

## 6. 简化 API：`transcode_layer_into`

```rust
pub fn transcode_layer_into(
    &self,
    src_fmt: &FormatSpec,
    tgt_fmt: &FormatSpec,
    src: &[u8],
    req: &LayerTranscodeRequest<'_>,
    scratch: &mut LayerTranscodeScratch,
    out: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    self.transcode_layer_into_with_stats(src_fmt, tgt_fmt, src, req, scratch, out)
        .map(|_| ())
}
```

这个函数的含义是：

```text
转码一层，把结果写入 out；
如果成功，不关心统计信息，只返回 Ok(())；
如果失败，返回 ConvertError。
```

### 参数逐个看

| 参数      | Rust 类型                    | 业务含义                               |
| --------- | ---------------------------- | -------------------------------------- |
| `&self`   | 只读借用当前 engine          | 调用 engine 方法                       |
| `src_fmt` | `&FormatSpec`                | 源格式 spec，例如 GOO                  |
| `tgt_fmt` | `&FormatSpec`                | 目标格式 spec，例如 CTB                |
| `src`     | `&[u8]`                      | 当前层的源格式 bytes                   |
| `req`     | `&LayerTranscodeRequest<'_>` | 当前层的执行参数，不携带 route 事实    |
| `scratch` | `&mut LayerTranscodeScratch` | 可复用临时 buffer                      |
| `out`     | `&mut Vec<u8>`               | 输出 buffer，函数把目标层 bytes 写进去 |

TypeScript 类比：

```typescript
function transcodeLayerInto(
    srcFmt: FormatSpec,
    tgtFmt: FormatSpec,
    src: Uint8Array,
    req: LayerTranscodeRequest,
    scratch: LayerTranscodeScratch,
    out: Uint8Array[]
): Result<void, ConvertError> {
    return transcodeLayerIntoWithStats(srcFmt, tgtFmt, src, req, scratch, out).map(() => undefined);
}
```

### `.map(|_| ())`

这一句是新手容易卡住的点：

```rust
self.transcode_layer_into_with_stats(...)
    .map(|_| ())
```

`transcode_layer_into_with_stats` 返回：

```rust
Result<LayerTranscodeStats, ConvertError>
```

但当前函数想返回：

```rust
Result<(), ConvertError>
```

所以 `.map(|_| ())` 表示：

```text
如果成功，丢掉 LayerTranscodeStats，改成 ()
如果失败，ConvertError 原样保留
```

TypeScript 类比：

```typescript
const result = transcodeLayerIntoWithStats(...);
if (!result.ok) {
    return result;
}
return { ok: true, value: undefined };
```

---

## 7. 真正核心：`transcode_layer_into_with_stats`

```rust
pub fn transcode_layer_into_with_stats(
    &self,
    src_fmt: &FormatSpec,
    tgt_fmt: &FormatSpec,
    src: &[u8],
    req: &LayerTranscodeRequest<'_>,
    scratch: &mut LayerTranscodeScratch,
    out: &mut Vec<u8>,
) -> Result<LayerTranscodeStats, ConvertError> {
    ...
}
```

它和上一个函数参数完全一样，但返回值不同：

```rust
Result<LayerTranscodeStats, ConvertError>
```

意思是：

```text
成功时返回这层用了几个策略、有没有 fast path、有没有 fallback；
失败时返回 ConvertError。
```

这就是带统计信息的版本。CLI 的 `--timing` 和 benchmark 这类需求会关心 stats。

---

## 8. 初始化统计

```rust
let mut stats = LayerTranscodeStats::default();
```

读法：

```text
创建一份默认统计值，并允许后面修改它
```

`mut` 表示变量可变。如果没有 `mut`，下面这些修改会编译失败：

```rust
stats.strategy_attempts += 1;
stats.fast_path_applied = true;
stats.strategy_declines += 1;
stats.fallback_used = true;
```

`LayerTranscodeStats` 在 `request.rs` 里定义：

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayerTranscodeStats {
    pub strategy_attempts: u32,
    pub strategy_declines: u32,
    pub fast_path_applied: bool,
    pub fallback_used: bool,
}
```

TypeScript 类比：

```typescript
let stats = {
    strategyAttempts: 0,
    strategyDeclines: 0,
    fastPathApplied: false,
    fallbackUsed: false
};
```

---

## 9. 派生 route 并尝试 fast path

```rust
let route = TranscodeRouteDescriptor::from_specs(src_fmt, tgt_fmt);
for transcoder in self.registry.lookup_route(route.route_key) {
    ...
}
```

`TranscodeRouteDescriptor::from_specs(src_fmt, tgt_fmt)` 是转换级 value object。它从源/目标 `FormatSpec` 里读取格式事实，包括：

- `route_key`：用于 registry dispatch 的稳定策略键
- `source_family_key` / `target_family_key`
- `source_profile_key` / `target_profile_key`
- `source_header_key` / `target_header_key`
- `source_archive_key` / `target_archive_key`

`route.route_key` 是更细的策略路由键。它基于 source/target container、layer codec、pixel model 和 crypto key 派生，让 `goo` / `prz` 这类共享 layer codec 语义的路径可以复用同一类 fast path。

`LayerTranscodeRequest` 只保留单层执行参数：

```rust
pub struct LayerTranscodeRequest<'a> {
    pub layer_index: u32,
    pub width: u32,
    pub height: u32,
    pub display_width_mm: f32,
    pub display_height_mm: f32,
    pub decode_options: &'a LayerOptions,
    pub encode_ctx: LayerEncodeContext<'a>,
}
```

`registry.lookup_route(route.route_key)` 返回当前 route 注册过的所有 fast path 策略：

```rust
fn lookup_route(&self, route: TranscodeRouteKey) -> &[&'static dyn LayerTranscoder];
```

TypeScript 类比：

```typescript
const route = TranscodeRouteDescriptor.fromSpecs(srcFmt, tgtFmt);
for (const transcoder of registry.lookupRoute(route.routeKey)) {
    // 逐个尝试 fast path
}
```

如果当前 route 没有注册 fast path，`lookup_route` 返回空数组，`for` 循环直接跳过，后面走 fallback。

---

## 10. 记录尝试次数

```rust
stats.strategy_attempts += 1;
```

每尝试一个 fast path 策略，就把尝试次数加一。

这不是业务转换必需的，而是观测和 benchmark 需要：

```text
这一层尝试了多少策略？
有多少策略说自己不适用？
最后是 fast path 还是 fallback？
```

---

## 11. 调用 fast path 策略

```rust
match transcoder.try_transcode_layer_into(src, req, scratch, out)? {
    ...
}
```

`transcoder` 实现的是 `LayerTranscoder` trait：

```rust
pub trait LayerTranscoder: Sync {
    fn try_transcode_layer_into(
        &self,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<FastPathOutcome, ConvertError>;
}
```

这个函数返回：

```rust
Result<FastPathOutcome, ConvertError>
```

三种情况：

| 情况                            | 含义                         |
| ------------------------------- | ---------------------------- |
| `Ok(FastPathOutcome::Applied)`  | fast path 成功处理了这一层   |
| `Ok(FastPathOutcome::Fallback)` | 这个策略看了请求后决定不处理 |
| `Err(ConvertError)`             | 尝试过程中真的出错           |

### 这里的 `?`

```rust
transcoder.try_transcode_layer_into(src, req, scratch, out)?
```

如果返回 `Err(error)`，整个 `transcode_layer_into_with_stats` 立刻返回这个错误。

如果返回 `Ok(outcome)`，`?` 会把 `outcome` 取出来交给 `match`。

TypeScript 类比：

```typescript
const result = transcoder.tryTranscodeLayerInto(src, req, scratch, out);
if (!result.ok) {
    return result;
}

switch (
    result.value
    // ...
) {
}
```

---

## 12. `FastPathOutcome::Applied`

```rust
FastPathOutcome::Applied => {
    stats.fast_path_applied = true;
    return Ok(stats);
}
```

含义：

```text
当前 fast path 已经成功把目标层 bytes 写进 out；
标记这层命中了 fast path；
直接返回统计信息；
不再尝试后续策略；
也不走 fallback。
```

为什么能直接返回？

因为 `out: &mut Vec<u8>` 已经被 fast path 填好了。

TypeScript 类比：

```typescript
case "applied":
    stats.fastPathApplied = true;
    return { ok: true, value: stats };
```

---

## 13. `FastPathOutcome::Fallback`

```rust
FastPathOutcome::Fallback => {
    stats.strategy_declines += 1;
}
```

含义：

```text
这个 fast path 策略不适合当前 layer/request；
记录一次 decline；
继续尝试下一个策略。
```

注意：`Fallback` 不是错误。

它更像策略在说：

```text
我看了一眼，这层我处理不了，但可以让别人试试；
如果没人能处理，最后走通用 fallback。
```

TypeScript 类比：

```typescript
case "fallback":
    stats.strategyDeclines += 1;
    break;
```

---

## 14. 所有 fast path 都没命中：走 fallback

```rust
fallback::transcode_layer_with_formats(src_fmt, tgt_fmt, src, req, scratch, out)?;
stats.fallback_used = true;
Ok(stats)
```

如果 `for` 循环结束还没有 `return Ok(stats)`，说明：

```text
没有 fast path
或者所有 fast path 都返回了 Fallback
```

于是调用通用路径：

```rust
fallback::transcode_layer_with_formats(...)
```

`fallback.rs` 里是：

```rust
scratch.pixel_buf.clear();
let decode_ctx = LayerDecodeContext::new(req.layer_index, req.width, req.height)
    .with_display_size_mm(req.display_width_mm, req.display_height_mm)
    .with_options(req.decode_options.clone());
src_fmt.decode_layer_into(src, &decode_ctx, &mut scratch.pixel_buf)?;
tgt_fmt.encode_layer_into(&scratch.pixel_buf, &req.encode_ctx, out)
```

翻译成业务语言：

```text
1. 清空可复用 pixel buffer
2. 构造 decode context
3. 用源 FormatSpec 把当前层 bytes 解码成灰度像素
4. 用目标 FormatSpec 把灰度像素编码成目标层 bytes
5. 输出写入 out
```

TypeScript 类比：

```typescript
scratch.pixelBuf.length = 0;
const pixels = srcFmt.decodeLayer(src, decodeCtx);
const encoded = tgtFmt.encodeLayer(pixels, encodeCtx);
out.replaceWith(encoded);
```

真正 Rust 代码用 `decode_layer_into` / `encode_layer_into`，是为了复用 buffer，减少分配。

---

## 15. 默认全局 engine

```rust
static DEFAULT_ENGINE: TranscodeEngine = TranscodeEngine::new(&DEFAULT_REGISTRY);
```

这定义了一个全局默认 engine。

它里面使用的 registry 是：

```rust
DEFAULT_REGISTRY
```

来自 `registry.rs`：

```rust
pub(crate) static DEFAULT_REGISTRY: StaticTranscodeRegistry =
    StaticTranscodeRegistry::new(paths::REGISTRY_ENTRIES);
```

也就是说：

```text
paths::REGISTRY_ENTRIES 里注册了有哪些 pairwise fast path；
DEFAULT_REGISTRY 用这些 entries 创建静态注册表；
DEFAULT_ENGINE 用 DEFAULT_REGISTRY 创建全局转码引擎。
```

TypeScript 类比：

```typescript
const DEFAULT_REGISTRY = new StaticTranscodeRegistry(REGISTRY_ENTRIES);
const DEFAULT_ENGINE = new TranscodeEngine(DEFAULT_REGISTRY);
```

---

## 16. 对外暴露 `default_engine`

```rust
pub fn default_engine() -> &'static TranscodeEngine {
    &DEFAULT_ENGINE
}
```

读法：

```text
返回全局默认 engine 的只读引用
```

返回类型：

```rust
&'static TranscodeEngine
```

表示：

```text
这是一个引用；
它指向全局静态 engine；
调用方只是借用它，不拥有它。
```

TypeScript 类比：

```typescript
export function defaultEngine(): TranscodeEngine {
    return DEFAULT_ENGINE;
}
```

Rust 版本更精确地表达了所有权：调用方拿到的是引用，不会复制或释放全局 engine。

---

## 17. 一层转码的完整控制流

把上面串起来：

```text
调用 default_engine().transcode_layer_into(...)
-> 如果调用简化 API，内部转到 transcode_layer_into_with_stats(...)
-> 创建 LayerTranscodeStats::default()
-> TranscodeRouteDescriptor::from_specs(src_fmt, tgt_fmt)
-> registry.lookup_route(route.route_key)
-> 逐个 try_transcode_layer_into(...)
   -> Applied: 写 out，return Ok(stats)
   -> Fallback: 记录 decline，继续
   -> Err: ? 直接返回错误
-> 没有策略命中
-> fallback::transcode_layer_with_formats(...)
   -> src_fmt.decode_layer_into(...)
   -> tgt_fmt.encode_layer_into(...)
-> 标记 fallback_used
-> Ok(stats)
```

用更短的业务公式：

```text
pairwise fast path 优先；
通用 decode -> encode 兜底；
所有结果写入调用方提供的 out buffer；
所有错误用 Result 往上传。
```

---

## 18. 读这个文件时最该抓住的语法

| 语法                     | 例子                                        | 你要读出的意思                |
| ------------------------ | ------------------------------------------- | ----------------------------- | ------ | --- | ---- | ------------------------------------- |
| `&FormatSpec`            | `src_fmt: &FormatSpec`                      | 借用一个格式 profile 执行对象 |
| `&dyn TranscodeRegistry` | `registry: &'static dyn TranscodeRegistry`  | 借用一个注册表接口对象        |
| `&[u8]`                  | `src: &[u8]`                                | 借用一段只读 bytes            |
| `&mut Vec<u8>`           | `out: &mut Vec<u8>`                         | 借用一个可写输出 buffer       |
| `Result<T, E>`           | `Result<LayerTranscodeStats, ConvertError>` | 成功返回 stats，失败返回错误  |
| `?`                      | `try_transcode_layer_into(...)?`            | 失败就提前返回                |
| `match`                  | `match outcome { ... }`                     | 穷举 fast path 的结果         |
| `static`                 | `DEFAULT_ENGINE`                            | 全局静态对象                  |
| `'static`                | `&'static TranscodeEngine`                  | 引用指向全局长生命周期对象    |
| `.map(                   | \_                                          | ())`                          | `.map( | \_  | ())` | 简化 API：成功值丢弃，只保留成功/失败 |

---

## 19. 下一步怎么读

读完 `engine.rs` 后，建议按这个顺序继续：

1. `rust/src/transcode/request.rs`
   看 `LayerTranscodeRequest`、`LayerTranscodeScratch`、`LayerTranscoder` 是怎么定义的。
2. `rust/src/transcode/execute.rs`
   看 CLI/WASM 共用的单层执行 facade 如何组装 request、搬运 scratch buffer 并调用 engine。
3. `rust/src/transcode/route.rs`
   看 `TranscodeRouteDescriptor` 和 `TranscodeRouteKey` 如何从 `FormatSpec` 派生 route 事实。
4. `rust/src/transcode/registry.rs`
   看 `lookup_route(route.route_key)` 怎么找到 fast path。
5. `rust/src/transcode/fallback.rs`
   看通用 decode -> encode 具体怎么写。
6. `rust/src/transcode/paths/goo_to_ctb.rs`
   看一条真实 fast path 如何实现 `LayerTranscoder`。
7. `rust/src/main.rs`
   看 CLI 怎么调用 `default_engine()`。
8. `rust/src/lib.rs`
   看 WASM API 怎么调用 `default_engine()`。
