# Rust 语法总结：结合 slicer-converter 源码和 TypeScript 对照

本文不是完整 Rust 教程，而是整理 `slicer-converter` 源码里最常见、最应该优先掌握的语法。

如果你已经熟悉 TypeScript，可以把本文当成“Rust -> TS 心智桥”。每一节都会给出：

- Rust 在本项目里的真实写法
- TypeScript 里的近似类比
- 两者最重要的差异

推荐配合这些文件阅读：

- `rust/src/main.rs`
- `rust/src/formats/mod.rs`
- `rust/src/transcode/engine.rs`
- `rust/src/transcode/request.rs`
- `rust/src/common/metadata.rs`

---

## 0. 速查表

这些类比只是帮助建立直觉，不是完全等价。Rust 最大的不同点是：所有权、借用、生命周期和错误处理都由类型系统强约束。

| Rust 写法              | TypeScript 类比                                     | 关键差异                                  |
| ---------------------- | --------------------------------------------------- | ----------------------------------------- | ------------ | ------------------------------- |
| `mod` / `use`          | `export` / `import`                                 | Rust 模块默认私有，且和文件树关系更紧     |
| `pub`                  | `export` / `public`                                 | 不写 `pub` 就不能被外部模块访问           |
| `struct`               | `interface` / `type` / class 的字段                 | Rust `struct` 只定义数据，方法写在 `impl` |
| `impl Type`            | `class` methods                                     | Rust 数据和方法可以分开写                 |
| `trait`                | `interface`                                         | Rust trait 可表达可替换策略和注册表能力   |
| `impl Trait for Type`  | `class Foo implements Bar`                          | Rust 可以给类型补实现，调用层依赖 trait   |
| `&dyn Trait`           | `Interface` 类型参数                                | Rust 明确表示“借用 + 运行时动态分发”      |
| `enum`                 | discriminated union                                 | Rust enum 分支能直接携带不同数据          |
| `match`                | `switch`                                            | Rust 会检查分支覆盖，模式匹配更强         |
| `Option<T>`            | `T \| undefined`                                    | Rust 强制处理 `Some` / `None`             |
| `Result<T, E>`         | `{ ok: true; value: T } \| { ok: false; error: E }` | Rust 用返回类型表达失败，不靠异常         |
| `?`                    | `if (!result.ok) return result`                     | 失败时提前返回当前函数                    |
| `Vec<u8>`              | `Uint8Array` / `number[]`                           | Rust `Vec` 拥有内存且可增长               |
| `&[u8]`                | `Readonly<Uint8Array>`                              | Rust 是借用视图，不拥有数据               |
| `&mut Vec<u8>`         | 可变 out parameter                                  | Rust 同一时间只允许一个可变借用           |
| `                      | x                                                   | ...`                                      | `(x) => ...` | Rust 闭包受所有权和借用规则约束 |
| `derive`               | decorator / codegen                                 | Rust 编译期生成 trait 实现                |
| `println!` / `format!` | 函数或模板字符串                                    | 带 `!` 的是宏，不是普通函数               |

---

## 1. `mod` / `pub` / `use`

### Rust 写法

`rust/src/lib.rs`：

```rust
pub mod common;
pub mod formats;
pub mod transcode;
```

含义：

```text
当前 crate 暴露 common、formats、transcode 三个模块
```

`rust/src/transcode/engine.rs`：

```rust
use crate::{common::ConvertError, formats::common::FormatSpec};

use super::{
    fallback,
    registry::{TranscodeRegistry, DEFAULT_REGISTRY},
    FastPathOutcome,
};
```

读法：

- `crate::` 从当前 Rust 包根部开始找
- `super::` 从父模块开始找
- `self::` 从当前模块开始找
- `use` 把路径里的名字引入当前文件
- `pub` 让模块、函数、类型或字段对外可见

可以先用 TS 路径建立直觉：

```text
crate::common::ConvertError      ≈ 从 package 根入口开始找
super::registry::DEFAULT_REGISTRY ≈ 从父目录/父模块开始找
self::engine::TranscodeEngine     ≈ 从当前目录/当前模块开始找
```

以 `rust/src/transcode/engine.rs` 为例，它位于 `transcode` 模块下面：

```rust
use crate::{common::ConvertError, formats::common::FormatSpec};

use super::{
    fallback,
    registry::{TranscodeRegistry, DEFAULT_REGISTRY},
    FastPathOutcome,
};
```

这里的 `crate::common::ConvertError` 是从 crate 根部，也就是 `lib.rs` 声明出来的模块树开始找：

```text
crate
├─ common
├─ formats
└─ transcode
```

所以 `crate::common::ConvertError` 类似 TS 里从包根入口导入某个公共模块。

而 `super::registry::DEFAULT_REGISTRY` 是从 `engine.rs` 的父模块 `transcode` 开始找。对应的父模块入口是 `rust/src/transcode/mod.rs`：

```rust
pub mod engine;
pub mod fallback;
pub mod registry;
pub mod request;

pub use request::{
    FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscodeStats,
};
```

所以在 `engine.rs` 里：

```rust
super::registry::DEFAULT_REGISTRY
```

可以理解成：

```text
从父模块 transcode 出发，找到 transcode::registry::DEFAULT_REGISTRY
```

`self::` 用得少一点，表示“从当前模块出发”。如果在 `transcode/mod.rs` 里写：

```rust
pub use self::engine::TranscodeEngine;
```

含义和本项目里的：

```rust
pub use engine::TranscodeEngine;
```

基本一样，都是从当前 `transcode` 模块下面找 `engine`。写 `self::` 时只是把“当前模块”说得更明确。

`pub use` 可以理解成 Rust 的重新导出，类似 TS 的：

```typescript
export { TranscodeEngine } from "./engine";
```

本项目的 `rust/src/transcode/mod.rs` 里同时有 `pub mod` 和 `pub use`：

```rust
pub mod engine;
pub mod fallback;
pub mod paths;
pub mod registry;
pub mod request;

pub use engine::{default_engine, TranscodeEngine};
pub use registry::{default_registry, TranscodeRegistry};
pub use request::{
    FastPathOutcome, FormatPair, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscodeStats,
    LayerTranscoder, TranscodeStats,
};
```

这两类语句作用不同：

- `pub mod engine;`：把 `engine` 这个子模块纳入模块树，并且让外部可以访问 `transcode::engine`。
- `pub use engine::TranscodeEngine;`：把 `engine` 里的 `TranscodeEngine` 重新挂到 `transcode` 这一层，外部可以直接写 `transcode::TranscodeEngine`。

如果没有 `pub use engine::TranscodeEngine;`，调用方通常要写更长的路径：

```rust
use slicer_converter_core::transcode::engine::TranscodeEngine;
```

有了重新导出后，可以写成：

```rust
use slicer_converter_core::transcode::TranscodeEngine;
```

所以 `pub mod` 解决的是“模块是否存在、是否可见”，`pub use` 解决的是“把内部名字整理到更舒服的对外入口”。

`pub` 需要逐层写，不会因为外层公开，里面所有东西就自动公开。可以按“门一层一层打开”理解：

```rust
pub mod transcode;
```

这只表示外部可以访问 `transcode` 这个模块入口，不表示 `transcode` 下面所有类型、函数、字段都自动公开。

例如 `rust/src/transcode/engine.rs`：

```rust
pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}
```

这里 `TranscodeEngine` 类型本身是公开的，所以外部可以使用这个类型；但字段 `registry` 没有 `pub`，所以外部不能直接访问：

```rust
// 不允许：registry 字段是私有的
engine.registry
```

如果希望字段也能被外部读写，字段本身也要写 `pub`。比如 `rust/src/formats/mod.rs` 里的 `LayerDecodeContext`：

```rust
pub struct LayerDecodeContext {
    pub layer_index: u32,
    pub width: u32,
    pub height: u32,
    pub options: LayerOptions,
}
```

这里是三层都打开了：

```text
pub mod formats           外部能进入 formats 模块
pub struct LayerDecodeContext  外部能使用这个结构体类型
pub width                 外部能访问这个字段
```

所以 Rust 的 `pub` 更像精确的 API 边界控制：模块公开、类型公开、字段公开、函数公开是几件分开的事。

除了完全公开的 `pub`，Rust 还可以限制公开范围：

```rust
pub(crate) static DEFAULT_REGISTRY: StaticTranscodeRegistry =
    StaticTranscodeRegistry::new(paths::REGISTRY_ENTRIES);
```

`pub(crate)` 的意思是“只在当前 crate 内公开”。本项目的 `DEFAULT_REGISTRY` 在 `rust/src/transcode/registry.rs` 里是 `pub(crate)`，所以同一个 Rust 包内部可以用它，但 crate 外部不能把它当公开 API 直接依赖。

这很适合内部实现细节：内部模块需要共享，但不希望暴露给外部调用者。

还有更窄的 `pub(super)`：

```rust
pub(super) unsafe fn scan_u8_run_end_scalar(...) -> usize {
    // ...
}
```

`pub(super)` 的意思是“只对父模块公开”。本项目的 `rust/src/common/simd/fallback.rs` 里有这种写法，表示这些 SIMD fallback 函数只给父模块 `simd` 这一层调度使用，不对更外层模块开放。

可以粗略记成：

```text
pub            对外公开，作为公共 API
pub(crate)     当前 crate 内公开，外部 crate 不可见
pub(super)     只对父模块公开
不写 pub        只在当前模块私有
```

TS 里常见的 `export` 基本是“从当前文件导出”；Rust 的可见性更细，可以把 API 边界卡在 crate、父模块、当前模块这些层级上。

### TypeScript 类比

```typescript
export * from "./common";
export * from "./formats";
export * from "./transcode";

import { ConvertError } from "../common/error";
import { FormatSpec } from "../formats";
```

### 关键差异

TS 里文件天然就是模块；Rust 里需要通过 `mod` 把文件/目录纳入模块树。

更具体一点：

- TS 中，`formats/index.ts` 是一种约定俗成的入口文件。别人可以 `import ... from "./formats"`，再由 `index.ts` 统一转发。
- Rust 中，`formats/mod.rs` 也很像目录模块的入口文件，但它不是单靠文件名自动生效。父模块必须先声明 `mod formats;` 或 `pub mod formats;`，Rust 才会把 `formats` 纳入模块树。

本项目里：

```rust
// rust/src/lib.rs
pub mod formats;
```

这句会让 Rust 去找 `rust/src/formats.rs` 或 `rust/src/formats/mod.rs`。因为项目里存在 `rust/src/formats/mod.rs`，所以它就成为 `formats` 模块的入口。

然后 `rust/src/formats/mod.rs` 里面继续声明：

```rust
pub mod ctb;
pub mod goo;
pub mod prz;
```

这些子模块才会继续被纳入 `formats` 下面。也就是说，Rust 的模块关系不是“文件存在就算模块”，而是“父模块用 `mod` 点名后，文件才进入模块树”。

补充：`mod.rs` 是 Rust 传统目录模块写法，确实可以类比 TS 的 `index.ts`。现在 Rust 也支持更扁平的写法：用 `formats.rs` 作为入口，再把子模块放在 `formats/ctb.rs`、`formats/goo.rs` 等文件里。本项目使用的是传统 `formats/mod.rs` 写法。

`lib.rs` 和 `main.rs` 可以理解成 Rust crate 的两种入口：

- `lib.rs` 是库入口，类似 TS package 的 `src/index.ts`。它负责组织并暴露可复用能力，比如 `pub mod formats;`、`pub mod transcode;`。本项目的 `lib.rs` 还放了 wasm binding 函数，供 JS/wasm 调用。
- `main.rs` 是可执行程序入口，类似 TS CLI/app 的 `src/main.ts`。真正执行命令行流程的是这里的 `fn main()`。

本项目里 `main.rs` 会像外部使用者一样，通过 crate 名引用库入口暴露的能力：

```rust
use slicer_converter_core::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::{self, LayerEncodeContext},
    transcode,
};
```

所以可以把关系记成：

```text
rust/src/lib.rs   = slicer-converter 的可复用库入口
rust/src/main.rs  = slicer-converter CLI 的启动入口
```

这也是 Rust 项目常见的拆法：核心逻辑放 `lib.rs`，CLI 只负责解析参数、调用库、打印结果。这样测试、benchmark、wasm 和命令行都能复用同一套核心能力。

Rust 默认私有，不写 `pub` 的东西外部不能访问。TS 里如果从文件导出就是公开，class 字段也常常默认更宽松。

---

## 2. `struct` 和结构体初始化

### Rust 写法

`rust/src/transcode/engine.rs`：

```rust
pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}
```

初始化：

```rust
Self { registry }
```

等价于：

```rust
TranscodeEngine {
    registry: registry,
}
```

字段名和变量名一样时，Rust 允许简写。

### TypeScript 类比

```typescript
type TranscodeEngine = {
    registry: TranscodeRegistry;
};

const engine: TranscodeEngine = { registry };
```

也可以类比 class 的数据字段：

```typescript
class TranscodeEngine {
    constructor(private registry: TranscodeRegistry) {}
}
```

### 关键差异

Rust 的 `struct` 主要定义数据形状；方法不写在 `struct` 里面，而是放在 `impl` 块里。

字段也默认私有。这里的 `registry` 没有 `pub`，所以外部不能直接访问 `engine.registry`。

---

## 3. `impl` 给类型实现方法

### Rust 写法

```rust
impl TranscodeEngine {
    pub const fn new(registry: &'static dyn TranscodeRegistry) -> Self {
        Self { registry }
    }
}
```

读法：

```text
给 TranscodeEngine 实现一个 new 方法
传入 registry
返回 Self，也就是 TranscodeEngine
```

`Self` 在 `impl TranscodeEngine` 里面就是 `TranscodeEngine`。

### TypeScript 类比

```typescript
class TranscodeEngine {
    constructor(private registry: TranscodeRegistry) {}

    static new(registry: TranscodeRegistry): TranscodeEngine {
        return new TranscodeEngine(registry);
    }
}
```

### 关键差异

TS 的 class 把字段、构造函数、方法写在一起。Rust 常见模式是：

```text
struct 定义数据
impl 定义方法
```

`const fn` 表示这个函数可以在编译期常量上下文调用。这个项目里它用于初始化全局静态对象：

```rust
static DEFAULT_ENGINE: TranscodeEngine = TranscodeEngine::new(&DEFAULT_REGISTRY);
```

---

## 4. `trait` 和 `impl Trait for Type`

### Rust 写法

现在的格式内核不再让每个格式实现一个大而全的格式 trait，而是让 `FormatSpec`
持有多个更小的策略 trait。`rust/src/formats/common/composed.rs` 里可以看到：

```rust
pub trait HeaderLayout: Sync {
    fn try_detect(&self, data: &[u8], hint: Option<&str>) -> bool;

    fn parse_header(
        &self,
        data: &[u8],
        spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError>;
}

pub trait LayerCodec: Sync {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError>;

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError>;
}
```

某个具体策略会这样接入：

```rust
impl LayerCodec for GooFamilyLayerCodec {
    // GOO family 自己的 decode_layer_into / encode_layer_into ...
}
```

读法：

```text
GooFamilyLayerCodec 实现了 LayerCodec 这组能力
FormatSpec 组合 HeaderLayout、LayerCodec、CryptoPolicy、FooterPolicy 等策略
调用层依赖 FormatSpec，不需要知道每个格式内部用了哪些具体策略
```

### TypeScript 类比

```typescript
interface LayerCodec {
    decodeLayerInto(data: Uint8Array, ctx: LayerDecodeContext, output: number[]): Result<void, ConvertError>;
    encodeLayerInto(pixels: Uint8Array, ctx: LayerEncodeContext, output: number[]): Result<void, ConvertError>;
}

class GooFamilyLayerCodec implements LayerCodec {
    decodeLayerInto(data: Uint8Array, ctx: LayerDecodeContext, output: number[]) {
        // ...
    }
}
```

### 关键差异

Rust `trait` 很像 TS `interface`，但能力更强：

- trait 可以有默认方法实现
- trait 可以参与动态分发，例如 `&dyn LayerCodec`
- trait 也可以作为泛型约束

在这个项目里，更重要的是“组合优于单个大接口”：`FormatSpec` 是格式事实源，
小 trait 是可替换策略。

---

## 5. `dyn Trait`

### Rust 写法

项目里仍然常见 `dyn Trait`，例如 `rust/src/transcode/engine.rs` 里的注册表：

```rust
pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}
```

`dyn TranscodeRegistry` 表示 trait object：

```text
运行时才知道具体类型，但它一定实现了 TranscodeRegistry
```

为什么这里需要 `dyn`？

因为 `TranscodeEngine` 只需要“能查 fast path 的注册表”，不需要关心具体注册表类型：

```rust
static DEFAULT_ENGINE: TranscodeEngine = TranscodeEngine::new(&DEFAULT_REGISTRY);
```

当前默认实现用真实注册表，测试或 benchmark 也可以传入空注册表或测试注册表。

和 `impl Trait` 对比时，可以抓住一个核心差异：

```text
dyn Trait   值背后真的可能是不同具体类型，运行时才知道是哪种
impl Trait  其实还是某一个确定类型，只是把具体类型名省略了
```

用 TS 心智类比，`dyn Trait` 更像一个接口变量：

```typescript
let registry: TranscodeRegistry;

if (useReal) {
    registry = new StaticTranscodeRegistry();
} else {
    registry = new EmptyTranscodeRegistry();
}

registry.lookup(pair);
```

`registry` 这个位置可能放真实注册表，也可能放空注册表。调用方只按 `TranscodeRegistry` 接口使用它。

Rust 里的对应写法是：

```rust
let registry: &dyn TranscodeRegistry = &DEFAULT_REGISTRY;
```

这里 `&dyn TranscodeRegistry` 是“接口对象引用”，它背后可以指向不同的具体 registry 实现。

`impl Trait` 更像“返回某个确定类型，但不把类型名写出来”：

```rust
fn make_registry() -> impl TranscodeRegistry {
    StaticTranscodeRegistry::new(...)
}
```

这个函数虽然返回 `impl TranscodeRegistry`，但每次返回的实际类型必须是同一种具体类型。下面这种“一会儿返回真实注册表，一会儿返回空注册表”的写法通常不行：

```rust
fn make_registry(use_real: bool) -> impl TranscodeRegistry {
    if use_real {
        StaticTranscodeRegistry::new(...)
    } else {
        EmptyTranscodeRegistry
    }
}
```

因为两个分支返回了不同具体类型。如果确实需要“可能是不同实现”，就要用 `dyn Trait`：

```rust
fn pick_registry(use_real: bool) -> &'static dyn TranscodeRegistry {
    if use_real {
        &DEFAULT_REGISTRY
    } else {
        &EMPTY_REGISTRY
    }
}
```

一句话记：

```text
impl Trait = 编译器知道具体类型，只是你省略了类型名
dyn Trait  = 代码只知道接口，具体类型藏在运行时对象后面
```

### TypeScript 类比

```typescript
function runWithRegistry(registry: TranscodeRegistry) {
    // 只依赖接口，不关心具体 class
    registry.lookup(pair);
}
```

### 关键差异

Rust 写成 `&dyn TranscodeRegistry` 时，里面有三层意思：

```text
&        借用，不拥有对象
dyn      运行时动态分发
Trait    这个对象满足该接口
```

TS 的接口变量通常不需要显式区分“静态分发/动态分发”，Rust 会把这件事写出来。

---

## 6. 所有权、借用、可变借用

### Rust 写法

先掌握这个表：

| 写法      | 含义                     |
| --------- | ------------------------ |
| `T`       | 拥有一个值               |
| `&T`      | 只读借用                 |
| `&mut T`  | 可变借用                 |
| `Vec<u8>` | 拥有一段可增长的字节数组 |
| `&[u8]`   | 借用一段只读字节切片     |

项目里常见：

```rust
fn decode_layer(&self, data: &[u8], ctx: &LayerDecodeContext) -> Result<Vec<u8>, ConvertError>;
```

读法：

```text
data 是借来的输入 bytes，不复制
ctx 是借来的解码上下文，不复制
成功时返回新的 Vec<u8>
```

再看 `engine.rs`：

```rust
out: &mut Vec<u8>,
```

读法：

```text
调用者把一个可复用的输出 buffer 借给函数
函数可以 clear/extend/write 它
```

### TypeScript 类比

```typescript
function decodeLayer(data: Readonly<Uint8Array>, ctx: LayerDecodeContext): Uint8Array {
    // data 只是读，不应该修改
}

function encodeInto(out: Uint8Array[]) {
    // 通过参数把结果写出去
}
```

### 关键差异

TS 里“不要改这个对象”很多时候是约定，或者靠 `Readonly<T>` 辅助。Rust 里是编译器强制：

- `&T` 只能读
- `&mut T` 可以写
- 同一时间不能既有可变借用又有其他借用
- 借用不能比被借用的数据活得更久

这就是 Rust 新手最容易不习惯、但也是 Rust 最重要的安全来源。

---

## 7. `Vec<u8>` 和 `&[u8]`

这个项目大量处理文件 bytes。

### Rust 写法

```rust
let source_bytes = fs::read(input)?;
```

`fs::read` 返回整个文件的 bytes。它是 `Vec<u8>`，拥有数据。

只读使用时：

```rust
formats::detect_format_spec(&source_bytes, Some(input_extension))
```

`&source_bytes` 会自动借成 `&[u8]`。函数只是读取，不拿走所有权。

### TypeScript 类比

```typescript
const sourceBytes = new Uint8Array(await file.arrayBuffer());
detectFormat(sourceBytes, "goo");
```

### 关键差异

在 TS 里，你通常把 `Uint8Array` 传来传去。Rust 里更常区分：

- `Vec<u8>`：我拥有这段 bytes
- `&[u8]`：我只是临时读一下这段 bytes

切片文件可能很大，这样能避免无意义复制。

---

## 8. `Result` / `Ok` / `Err`

### Rust 写法

Rust 常用 `Result` 表示可能失败：

```rust
Result<T, E>
```

含义：

```text
成功：Ok(T)
失败：Err(E)
```

例子：

```rust
fn run() -> Result<(), String>
```

读法：

```text
run 成功时不返回有意义的数据，只返回 ()
失败时返回 String 错误信息
```

另一个例子：

```rust
fn decode_layer(...) -> Result<Vec<u8>, ConvertError>
```

读法：

```text
成功时返回 Vec<u8>
失败时返回 ConvertError
```

### TypeScript 类比

```typescript
type Result<T, E> =
    | { ok: true; value: T }
    | { ok: false; error: E };

function decodeLayer(...): Result<Uint8Array, ConvertError> {
    // ...
}
```

### 关键差异

TS 代码经常用 `throw` 表达失败。Rust 这里更倾向于把失败写进返回类型。

你看到：

```rust
Result<Vec<u8>, ConvertError>
```

就应该马上意识到：调用方必须处理失败。

---

## 9. `?` 错误传播

### Rust 写法

```rust
let value = may_fail()?;
```

等价理解：

```text
如果 may_fail() 返回 Ok(v)，就把 v 放进 value
如果返回 Err(e)，当前函数立刻返回 Err(e)
```

项目里的例子：

```rust
match parse_args(std::env::args().skip(1))? {
    CliAction::Help => ...
    CliAction::Run(args) => ...
}
```

如果 `parse_args` 失败，`run()` 会直接返回错误，不再进入 `match`。

### TypeScript 类比

如果用 TS 手写 `Result`，大概是：

```typescript
const parsed = parseArgs(args);
if (!parsed.ok) {
    return parsed;
}

const action = parsed.value;
```

Rust 写成：

```rust
let action = parse_args(args)?;
```

### 关键差异

`?` 不是忽略错误，而是把错误往上传。它要求当前函数本身也返回 `Result` 或兼容类型。

---

## 10. `Option` / `Some` / `None`

### Rust 写法

Rust 不用 `null` 表达“可能没有”，而是用：

```rust
Option<T>
```

两种情况：

```rust
Some(value)
None
```

项目里的例子：

```rust
let input_extension = extension_hint(input)
    .ok_or_else(|| format!("input path '{}' has no file extension", input.display()))?;
```

读法：

```text
extension_hint(input) 返回 Option
Some(ext) 表示有扩展名
None 表示没有扩展名
ok_or_else 把 None 转成 Err
? 遇到 Err 就提前返回
```

### TypeScript 类比

```typescript
function extensionHint(path: Path): string | undefined {
    // ...
}

const inputExtension = extensionHint(input);
if (inputExtension === undefined) {
    return { ok: false, error: `input path has no file extension` };
}
```

### 关键差异

TS 的 `undefined` 很容易被忘记处理。Rust 的 `Option<T>` 会逼你显式处理：

- `Some(value)`
- `None`

`ok_or_else` 的作用就是把：

```text
Option<T>
```

转换成：

```text
Result<T, E>
```

这样就能接上 `?`。

---

## 11. `match` 模式匹配

### Rust 写法

`main.rs`：

```rust
match run() {
    Ok(()) => {}
    Err(message) => {
        eprintln!("{message}");
        process::exit(1);
    }
}
```

`engine.rs`：

```rust
match transcoder.try_transcode_layer_into(src, req, scratch, out)? {
    FastPathOutcome::Applied => {
        stats.fast_path_applied = true;
        return Ok(stats);
    }
    FastPathOutcome::Fallback => {
        stats.strategy_declines += 1;
    }
}
```

### TypeScript 类比

```typescript
switch (outcome.kind) {
    case "applied":
        stats.fastPathApplied = true;
        return ok(stats);
    case "fallback":
        stats.strategyDeclines += 1;
        break;
}
```

### 关键差异

Rust 的 `match` 比 TS `switch` 更强：

- 可以匹配 enum 分支
- 可以解构分支携带的数据
- 编译器能检查你有没有漏掉分支

所以 Rust 很多地方用 `match` 替代一串 `if/else`。

---

## 12. `enum` 枚举

### Rust 写法

`main.rs`：

```rust
enum CliAction {
    Run(CliArgs),
    Help,
}
```

读法：

```text
CLI 解析结果只有两类：
Run，并携带 CliArgs
Help，不携带额外数据
```

### TypeScript 类比

```typescript
type CliAction = { kind: "run"; args: CliArgs } | { kind: "help" };
```

### 关键差异

Rust enum 分支可以直接携带数据：

```rust
Run(CliArgs)
```

这相当于 TS discriminated union 里的：

```typescript
{
    kind: "run";
    args: CliArgs;
}
```

但 Rust 的 `match` 会和 enum 配合得更紧，分支遗漏时编译器会提醒。

---

## 13. 闭包 `|x| ...`

### Rust 写法

`main.rs`：

```rust
.map_err(|error| {
    format!(
        "conversion failed for '{}' -> '{}': {error}",
        args.input.display(),
        args.output.display()
    )
})
```

常见形式：

```rust
|x| x + 1
|error| format!("failed: {error}")
|| expensive_default()
```

最后一个 `||` 表示没有参数。

### TypeScript 类比

```typescript
const plusOne = (x: number) => x + 1;
const toMessage = (error: Error) => `failed: ${error.message}`;
const expensiveDefault = () => computeDefault();
```

### 关键差异

Rust 闭包和 TS arrow function 很像，但捕获外部变量时会受所有权和借用规则影响。

你可以先按 arrow function 理解，等遇到 borrow checker 报错时，再回头补闭包捕获规则。

---

## 14. `map` / `map_err` / `ok_or_else`

这些是 `Result` / `Option` 上常见的转换方法。

### `map`

Rust：

```rust
self.transcode_layer_into_with_stats(...)
    .map(|_| ())
```

读法：

```text
如果成功，把成功值转换成 ()
如果失败，错误保持不变
```

TS 类比：

```typescript
if (result.ok) {
    return { ok: true, value: undefined };
}
return result;
```

### `map_err`

Rust：

```rust
convert_file_with_options(...)
    .map_err(|error| format!("conversion failed: {error}"))
```

读法：

```text
如果失败，把错误转换成另一个错误
成功值保持不变
```

TS 类比：

```typescript
if (!result.ok) {
    return { ok: false, error: `conversion failed: ${result.error}` };
}
return result;
```

### `ok_or_else`

Rust：

```rust
option.ok_or_else(|| "missing value".to_string())?
```

读法：

```text
Some(value) -> Ok(value)
None -> Err(...)
```

TS 类比：

```typescript
if (value === undefined) {
    return { ok: false, error: "missing value" };
}
return { ok: true, value };
```

### 关键差异

Rust 很多错误处理不是通过 `try/catch`，而是通过这些方法组合：

```text
Option -> Result -> ? 提前返回
Result -> map_err 加上下文 -> ? 提前返回
```

---

## 15. 生命周期：`'static` 和 `'_`

生命周期描述“引用能活多久”。新手先不用深入，只需要能读懂常见写法。

### `'static`

Rust：

```rust
registry: &'static dyn TranscodeRegistry
```

读法：

```text
registry 是一个引用，指向活得足够久的全局对象
```

对应：

```rust
static DEFAULT_ENGINE: TranscodeEngine = TranscodeEngine::new(&DEFAULT_REGISTRY);
```

### `'_`

Rust：

```rust
ctx: &LayerEncodeContext<'_>
```

`'_` 表示让编译器推断生命周期。

为什么 `LayerEncodeContext` 要带生命周期？因为它里面可能借用了 metadata：

```rust
pub struct LayerEncodeContext<'a> {
    pub layer_metadata: Option<LayerEncodeMetadata<'a>>,
}
```

### TypeScript 类比

TS 没有生命周期标注。你可能会写：

```typescript
type LayerEncodeContext = {
    layerMetadata?: LayerEncodeMetadata;
};
```

### 关键差异

Rust 要额外表达：

```text
这个 layer_metadata 是借来的
它不能比来源活得更久
```

生命周期不是业务字段，而是编译器用来证明引用安全的标记。

---

## 16. `derive`

### Rust 写法

```rust
#[derive(Debug, Clone, Default)]
pub struct LayerDecodeContext {
    ...
}
```

`derive` 让编译器自动生成某些能力：

| derive        | 作用                                 |
| ------------- | ------------------------------------ |
| `Debug`       | 可以用 `{:?}` 打印调试信息           |
| `Clone`       | 可以复制一份值                       |
| `Copy`        | 小类型可以按位复制，复制后原值仍可用 |
| `Default`     | 可以创建默认值                       |
| `Serialize`   | 可以序列化，例如转 JSON              |
| `Deserialize` | 可以反序列化，例如从 JSON 读回来     |

metadata 里大量使用：

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
```

### TypeScript 类比

TS 的对象天然能做一部分事情：

```typescript
type LayerDecodeContext = {
    layerIndex: number;
    width: number;
    height: number;
};
```

你可以直接 `console.log(ctx)`，也可以 `JSON.stringify(ctx)`。

### 关键差异

Rust 更显式。一个类型能不能 debug 打印、clone、默认构造、serde 序列化，都要通过 trait 表达。

`derive` 就是编译器帮你自动实现这些 trait。

---

## 17. 宏：带 `!` 的调用

### Rust 写法

常见宏：

```rust
println!("hello");
eprintln!("{message}");
format!("failed: {error}");
vec![1, 2, 3];
```

本项目也会用宏组织重复样板，例如测试和底层 codec 中的批量 case：

```rust
macro_rules! roundtrip_cases {
    ($($name:ident: $pixels:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let pixels = $pixels;
                // ...
            }
        )+
    };
}
```

看到 `!` 时，可以先理解为：

```text
这是宏，不是普通函数；它可能在编译期展开成更多 Rust 代码
```

### TypeScript 类比

宏更接近“编译期代码生成”。粗略类比：

- Babel transform
- decorator metadata
- codegen 脚本
- 模板生成代码

### 关键差异

Rust 宏是语言内的一等工具，适合减少重复测试、重复声明或重复小模板。

---

## 18. `for` 循环和迭代器

### Rust 写法

`engine.rs`：

```rust
for transcoder in self.registry.lookup(req.pair) {
    ...
}
```

读法：

```text
遍历当前格式对可用的所有 fast path 策略
```

### TypeScript 类比

```typescript
for (const transcoder of registry.lookup(req.pair)) {
    // ...
}
```

### 关键差异

Rust 的 iterator 很常用，很多集合操作会返回惰性 iterator，需要 `for`、`collect`、`find_map` 等消费。

---

## 19. `return` 和最后表达式返回

### Rust 写法

Rust 可以显式 `return`：

```rust
return Ok(stats);
```

也可以让函数最后一个表达式作为返回值：

```rust
Ok(stats)
```

`engine.rs`：

```rust
FastPathOutcome::Applied => {
    stats.fast_path_applied = true;
    return Ok(stats);
}
```

函数末尾：

```rust
stats.fallback_used = true;
Ok(stats)
```

### TypeScript 类比

TS 必须显式 `return`：

```typescript
return ok(stats);
```

### 关键差异

Rust 最后表达式返回时没有分号：

```rust
Ok(stats)
```

如果写成：

```rust
Ok(stats);
```

它就变成一条语句，返回值会变成 `()`。

---

## 20. `Default::default()` 和 `mut`

### Rust 写法

`engine.rs`：

```rust
let mut stats = LayerTranscodeStats::default();
```

读法：

```text
创建一个 LayerTranscodeStats 的默认值
stats 是可变变量
```

之后才能修改：

```rust
stats.strategy_attempts += 1;
stats.fast_path_applied = true;
```

### TypeScript 类比

```typescript
let stats = createDefaultLayerTranscodeStats();
stats.strategyAttempts += 1;
stats.fastPathApplied = true;
```

### 关键差异

Rust 变量默认不可变。要修改绑定，必须写 `mut`。

`Default::default()` 类似一个统一的默认构造约定。只要类型实现了 `Default`，就能用：

```rust
LayerTranscodeStats::default()
```

---

## 21. 当前项目里最该优先掌握的语法

按优先级：

1. `Result` / `Ok` / `Err` / `?`
2. `Option` / `Some` / `None`
3. `&T` / `&mut T` / `Vec<u8>` / `&[u8]`
4. `struct` / `impl`
5. `trait` / `impl Trait for Type` / `dyn Trait`
6. `enum` / `match`
7. `mod` / `pub` / `use`
8. closure：`|x| ...`
9. `derive`
10. macro：`println!` / `format!` / `vec!` / `macro_rules!`

---

## 22. 用一句话读 `engine.rs`

`rust/src/transcode/engine.rs` 可以这样读：

```text
TranscodeEngine 持有一个 fast path 注册表；
转码一层时先查当前格式对有没有专门策略；
策略命中就直接写入 out 并返回 stats；
策略不适用就继续尝试；
全部不适用时走 fallback；
fallback 通过源/目标 FormatSpec 做 decode -> encode。
```

如果换成 TS 心智，大概是：

```typescript
function transcodeLayerWithStats(...) {
    const stats = createDefaultStats();

    for (const transcoder of registry.lookup(req.pair)) {
        const outcome = transcoder.tryTranscodeLayerInto(...);
        if (outcome.kind === "applied") {
            stats.fastPathApplied = true;
            return ok(stats);
        }
        stats.strategyDeclines += 1;
    }

    fallbackTranscodeLayerWithFormats(...);
    stats.fallbackUsed = true;
    return ok(stats);
}
```

这不是源码等价翻译，只是帮助理解控制流。真正 Rust 代码多出来的 `Result`、`?`、`&mut Vec<u8>`，正是 Rust 相比 TS 更强调的安全和性能边界。
