# 可扩展注册表重构 — 执行计划

> **前置文档**：[refactor-extensible-registry.md](refactor-extensible-registry.md)（技术规范）  
> **目标**：按 spec 实施重构，全程保持编译通过，最终 60+ 测试全绿  
> **预计改动**：8 个文件，~145 行净变更

## 执行结果（2026-04-13）

- 已完成：Step 1-8 全部落地
- 验证结果：`cargo test --manifest-path rust/Cargo.toml` 63/63 通过（29 unit + 29 cross-format + 5 sample）
- 验证结果：`pnpm build:wasm` 通过（产物输出到 `packages/slicer-wasm/lib`）
- 对应提交：`6bb9e98`（refactor: extensible format registry (FormatDesc + FORMATS const array)）

---

## 执行策略

采用 **自底向上** 顺序：先扩展 trait，再补各格式实现，最后改注册表和调用方。  
每步之后 `cargo check --manifest-path rust/Cargo.toml` 验证编译，全部完成后 `cargo test --manifest-path rust/Cargo.toml` 验证行为。

⚠ **关键约束**：步骤 1-3 是纯增量（不删不改），步骤 4 是核心替换（一次性完成），步骤 5-7 是适配和清理。

---

## Step 1：扩展 SlicerFormat trait（`rust/src/formats/mod.rs`）

**操作**：在 trait 定义中新增 2 个方法

```rust
// 添加到 pub trait SlicerFormat 末尾（build_footer 之后）

/// 此格式支持的文件扩展名（不含点号，小写）
fn extensions(&self) -> &'static [&'static str];

/// 尝试从文件头字节检测是否为此格式（O(1) magic 比较）
fn try_detect(&self, _data: &[u8]) -> bool {
    false
}
```

**同时**：在 trait 声明上添加 `Sync` 约束

```rust
pub trait SlicerFormat: Sync {
```

**验证**：`cargo check --manifest-path rust/Cargo.toml` — 预期编译失败（3 个 impl 缺少 `extensions()`），进入 Step 2

---

## Step 2：各格式实现新方法（3 个文件并行改）

### 2a. `rust/src/formats/ctb/mod.rs` — CtbFormat impl

```rust
fn extensions(&self) -> &'static [&'static str] {
    &["ctb", "cbddlp", "photon"]
}

fn try_detect(&self, data: &[u8]) -> bool {
    if data.len() < 4 { return false; }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    matches!(magic, MAGIC_CBDDLP | MAGIC_CTB | MAGIC_CTB_V4 | MAGIC_CTB_V4_GKTWO)
}
```

### 2b. `rust/src/formats/goo/mod.rs` — GooFormat impl

```rust
fn extensions(&self) -> &'static [&'static str] {
    &["goo", "prz"]
}

fn try_detect(&self, data: &[u8]) -> bool {
    data.len() >= 12 && &data[4..12] == MAGIC_BYTES
}
```

### 2c. `rust/src/formats/sl1/mod.rs` — Sl1Format impl

```rust
fn extensions(&self) -> &'static [&'static str] {
    &["sl1", "sl1s"]
}

fn try_detect(&self, data: &[u8]) -> bool {
    data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4B
}
```

**验证**：`cargo check` — 应编译通过（新方法有实现，旧代码未改）

---

## Step 3：添加 FormatDesc 结构 + FORMATS 数组（`rust/src/formats/mod.rs`）

**操作**：在 trait 定义之后、现有 `get_format()` 之前插入

```rust
/// 格式描述符 — 注册表条目
pub struct FormatDesc {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub format: &'static (dyn SlicerFormat + Sync),
}

/// 所有已注册格式
pub const FORMATS: &[FormatDesc] = &[
    FormatDesc {
        name: "ctb",
        aliases: &["ctb", "cbddlp", "photon"],
        format: &ctb::CtbFormat as &(dyn SlicerFormat + Sync),
    },
    FormatDesc {
        name: "goo",
        aliases: &["goo", "prz"],
        format: &goo::GooFormat as &(dyn SlicerFormat + Sync),
    },
    FormatDesc {
        name: "sl1",
        aliases: &["sl1", "sl1s"],
        format: &sl1::Sl1Format as &(dyn SlicerFormat + Sync),
    },
];
```

**验证**：`cargo check` — 编译通过（FormatDesc/FORMATS 目前未被使用，旧代码仍在）

---

## Step 4：替换注册表函数（`rust/src/formats/mod.rs`）— 核心步骤

**4a. 重写 `get_format_by_name()`**

```rust
pub fn get_format_by_name(name: &str) -> Result<&'static dyn SlicerFormat, ConvertError> {
    let name_lower = name.to_ascii_lowercase();
    for desc in FORMATS {
        if desc.aliases.iter().any(|a| *a == name_lower) {
            return Ok(desc.format);
        }
    }
    Err(ConvertError::UnknownFormat)
}
```

**4b. 重写 `detect_format()` — 返回值从 `Format` 改为 `&'static dyn SlicerFormat`**

```rust
pub fn detect_format(data: &[u8], extension_hint: Option<&str>) -> Result<&'static dyn SlicerFormat, ConvertError> {
    // Phase 1: magic
    for desc in FORMATS {
        if desc.format.try_detect(data) {
            return Ok(desc.format);
        }
    }
    // Phase 2: extension
    if let Some(ext) = extension_hint {
        let ext_clean = ext.to_lowercase();
        let ext_clean = ext_clean.trim_start_matches('.');
        for desc in FORMATS {
            if desc.format.extensions().iter().any(|e| *e == ext_clean) {
                return Ok(desc.format);
            }
        }
    }
    Err(ConvertError::UnknownFormat)
}
```

**4c. 删除**：`Format` enum、`Format::as_str()`、`get_format(Format)`

> ⚠ 此步结束后，`lib.rs` 和测试会编译失败。必须连续执行 Step 5 和 Step 6。

---

## Step 5：适配 `rust/src/lib.rs`

**`parse_header` 函数**：

```rust
// 删除这两行：
let format = formats::detect_format(data, extension_hint.as_deref())?;
let fmt = formats::get_format(format);

// 替换为：
let fmt = formats::detect_format(data, extension_hint.as_deref())
    .map_err(|e| JsError::new(&e.to_string()))?;
```

**`ParseResult.format` 字段**：

```rust
// 删除：
format: format.as_str().to_string(),

// 替换为：
format: fmt.format_name().to_string(),
```

**其他函数**：`decode_layer`、`encode_layer` 等已使用 `get_format_by_name()`，签名不变，无需修改。

**验证**：`cargo check` — lib.rs 编译通过

---

## Step 6：适配测试文件

### 6a. `rust/tests/fixture_files.rs`（3 处）

每处模式相同：

```rust
// 之前
let format = formats::detect_format(&data, Some("goo")).unwrap();
assert_eq!(format, formats::Format::Goo);

// 之后
let fmt = formats::detect_format(&data, Some("goo")).unwrap();
assert_eq!(fmt.format_name(), "goo");
```

### 6b. `rust/tests/cross_format.rs`（7 处）

3 处 `detect_*_format_by_magic` 测试：

```rust
// 之前
let format = slicer_converter::formats::detect_format(&data, None).unwrap();
assert_eq!(format, slicer_converter::formats::Format::Ctb);

// 之后
let fmt = slicer_converter::formats::detect_format(&data, None).unwrap();
assert_eq!(fmt.format_name(), "ctb");
```

4 处 `detect_format_by_extension_hint` 测试内的断言：

```rust
// 之前
slicer_converter::formats::Format::Ctb

// 之后
"ctb"  // 使用 format_name() 字符串比较
```

**验证**：`cargo check --tests` — 全部编译通过

---

## Step 7：添加新测试（`rust/tests/cross_format.rs`）

在 format detection 区块末尾添加 3 个测试：

```rust
#[test]
fn test_format_registry_completeness() {
    for desc in slicer_converter::formats::FORMATS {
        for alias in desc.aliases {
            let found = slicer_converter::formats::get_format_by_name(alias).unwrap();
            assert_eq!(found.format_name(), desc.name);
        }
    }
}

#[test]
fn test_detect_by_magic_via_registry() {
    let ctb_header = 0x12FD_0086u32.to_le_bytes();
    let fmt = slicer_converter::formats::detect_format(&ctb_header, None).unwrap();
    assert_eq!(fmt.format_name(), "ctb");
}

#[test]
fn test_detect_by_extension_only() {
    let empty: &[u8] = &[];
    let fmt = slicer_converter::formats::detect_format(empty, Some("sl1")).unwrap();
    assert_eq!(fmt.format_name(), "sl1");
}
```

**验证**：`cargo test --manifest-path rust/Cargo.toml` — 全部 63+ 测试通过（60 原有 + 3 新增）

---

## Step 8：清理和提交

1. 确认无 `Format::` 残留引用：`grep -r "Format::" rust/src/ rust/tests/ rust/examples/`
2. 确认 WASM 编译：`pnpm build:wasm`
3. 提交：`git add -A && git commit -m "refactor: extensible format registry (FormatDesc + FORMATS const array)"`

---

## 风险和回退

| 风险                                     | 缓解                                       |
| ---------------------------------------- | ------------------------------------------ |
| Step 4 删除 Format enum 后大面积编译失败 | Step 4/5/6 连续执行，不在中间 check        |
| SL1 try_detect 的 ZIP magic 误匹配       | 数组中 CTB/GOO 排在前面，magic 更强        |
| detect_format 返回类型变化破坏下游       | 仅 lib.rs 和测试在用，且已在 Step 5/6 适配 |

---

## 检查清单

- [x] Step 1: trait 加 `extensions()` + `try_detect()` + `Sync`
- [x] Step 2: ctb/goo/sl1 实现新方法
- [x] Step 3: 添加 `FormatDesc` + `FORMATS`
- [x] Step 4: 替换 `get_format_by_name`/`detect_format`，删除 `Format` enum
- [x] Step 5: 适配 `lib.rs`
- [x] Step 6: 适配测试文件（10 处）
- [x] Step 7: 添加 3 个新测试
- [x] Step 8: grep 残留 + wasm build + git commit
