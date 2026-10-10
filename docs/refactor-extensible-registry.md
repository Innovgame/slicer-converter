# 格式注册表可扩展性重构 — 技术规范

> **状态**：Draft  
> **范围**：`rust/src/formats/mod.rs`、各格式 `mod.rs`、`rust/src/lib.rs`  
> **目标**：将格式注册从 5 处硬编码改为单一信息源，支撑 10-30 格式无痛扩展

---

## 1. 问题陈述

### 1.1 当前架构

新增一个格式需要修改 **5 处**代码：

```
① rust/src/formats/mod.rs  Format enum 加变体
② rust/src/formats/mod.rs  Format::as_str() match 加分支
③ rust/src/formats/mod.rs  get_format() match 加分支
④ rust/src/formats/mod.rs  get_format_by_name() match 加分支
⑤ rust/src/formats/mod.rs  detect_format() 加 magic/ext 检测逻辑
```

**风险**：遗漏任一处导致编译错误或运行时 bug；代码审查成本随格式数量线性增加。

### 1.2 目标状态

新增一个格式只需：

```
① rust/src/formats/xxx/mod.rs  实现 SlicerFormat trait（含 try_detect + extensions）
② rust/src/formats/mod.rs      FORMATS 数组加一行
```

---

## 2. 设计方案

### 2.1 核心数据结构：FormatDesc

```rust
/// 格式描述符 — 注册表中的单个条目
pub struct FormatDesc {
    /// 格式主名称（如 "ctb"），用于 WASM API 返回值和日志。
    /// 必须与 aliases[0] 相同。
    pub name: &'static str,
    /// 所有可识别的别名（含主名称在 [0] 位），用于 get_format_by_name() 查找。
    /// 当多个格式声称同一别名时，数组中较早的 FormatDesc 优先（先注册先命中）。
    pub aliases: &'static [&'static str],
    /// 实现了 SlicerFormat trait 的静态实例
    pub format: &'static (dyn SlicerFormat + Sync),
}
```

### 2.2 格式注册表：常量数组

```rust
/// 所有已注册格式 — 格式添加/删除的唯一修改点
pub const FORMATS: &[FormatDesc] = &[
    FormatDesc {
        name: "ctb",
        aliases: &["ctb", "cbddlp", "photon"],
        format: &ctb::CtbFormat,
    },
    FormatDesc {
        name: "goo",
        aliases: &["goo", "prz"],
        format: &goo::GooFormat,
    },
    FormatDesc {
        name: "sl1",
        aliases: &["sl1", "sl1s"],
        format: &sl1::Sl1Format,
    },
];
```

> **aliases vs extensions 语义说明**：
>
> - `FormatDesc.aliases`：用于 WASM API 的 `get_format_by_name()` 查找（如 JS 传入 `"cbddlp"`）
> - `SlicerFormat::extensions()`：用于 `detect_format()` 做文件扩展名匹配
> - 当前版本中两者相同。未来可独立配置（如某格式有 API 别名但无对应扩展名）。
> - 大小写处理统一使用 `to_lowercase()` 进行比较。

### 2.3 Trait 增强：try_detect + extensions

```rust
pub trait SlicerFormat: Sync {
    // ── 现有方法（不变） ──
    fn format_name(&self) -> &'static str;
    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError>;
    fn decode_layer(&self, data: &[u8], ctx: &LayerDecodeContext) -> Result<Vec<u8>, ConvertError>;
    fn encode_layer(&self, pixels: &[u8], ctx: &LayerEncodeContext) -> Result<Vec<u8>, ConvertError>;
    fn build_header(&self, metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError>;
    fn finalize_header(&self, header: &[u8], _layer_sizes: &[u32]) -> Result<Vec<u8>, ConvertError> {
        Ok(header.to_vec())
    }
    fn build_footer(&self) -> Vec<u8> {
        vec![]
    }

    // ── 新增方法 ──

    /// 此格式支持的文件扩展名（不含点号，小写），用于扩展名检测
    fn extensions(&self) -> &'static [&'static str];

    /// 尝试从文件头字节检测是否为此格式。
    /// - 返回 true 表示确认匹配
    /// - 返回 false 表示不匹配
    /// - 实现必须仅做 O(1) 操作（magic 字节比较），不做完整解析
    fn try_detect(&self, _data: &[u8]) -> bool {
        false  // 默认：不支持 magic 检测，依赖扩展名
    }
}
```

### 2.4 查找函数（替代 get_format / get_format_by_name）

```rust
/// 按别名查找格式（WASM API 使用）
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

### 2.5 格式检测（替代 detect_format）

```rust
/// 检测文件格式：先 magic 字节，再扩展名
pub fn detect_format(data: &[u8], extension_hint: Option<&str>) -> Result<&'static dyn SlicerFormat, ConvertError> {
    // Phase 1: magic 字节检测（高优先级）
    for desc in FORMATS {
        if desc.format.try_detect(data) {
            return Ok(desc.format);
        }
    }

    // Phase 2: 扩展名匹配
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

> **注意**：SL1 的 `try_detect()` 使用 ZIP magic (`PK`)，会匹配所有 ZIP 文件。
> 当前三种格式互不冲突（CTB/GOO 有独特 magic，SL1 是唯一 ZIP 格式）。
> 未来若添加其他 ZIP 格式（如 ChituboxZip），需在 `try_detect()` 中
> 进一步检查 ZIP 内容（如 config.ini 存在性），或提升扩展名匹配优先级。

### 2.6 各格式 try_detect 实现

**CTB**：

```rust
fn try_detect(&self, data: &[u8]) -> bool {
    if data.len() < 4 { return false; }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    matches!(magic, MAGIC_CBDDLP | MAGIC_CTB | MAGIC_CTB_V4 | MAGIC_CTB_V4_GKTWO)
}

fn extensions(&self) -> &'static [&'static str] {
    &["ctb", "cbddlp", "photon"]
}
```

**GOO**：

```rust
fn try_detect(&self, data: &[u8]) -> bool {
    data.len() >= 12 && &data[4..12] == MAGIC_BYTES
}

fn extensions(&self) -> &'static [&'static str] {
    &["goo", "prz"]
}
```

**SL1**：

```rust
fn try_detect(&self, data: &[u8]) -> bool {
    // ZIP magic (PK\x03\x04)
    data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4B
}

fn extensions(&self) -> &'static [&'static str] {
    &["sl1", "sl1s"]
}
```

---

## 3. 删除项

| 删除                 | 理由                                       |
| -------------------- | ------------------------------------------ |
| `Format` enum        | 被 `FormatDesc` 替代；lib.rs 不再需要 enum |
| `Format::as_str()`   | 被 `FormatDesc.name` 替代                  |
| `get_format(Format)` | 被 `get_format_by_name(&str)` 统一         |

---

## 4. lib.rs 变更

### 4.1 parse_header

```rust
// 之前
let format = formats::detect_format(data, ext)?;  // 返回 Format enum
let fmt = formats::get_format(format);             // enum → dyn SlicerFormat

// 之后
let fmt = formats::detect_format(data, ext)?;      // 直接返回 dyn SlicerFormat
```

注意：`ParseResult.format` 字段需要格式名字符串，改用 `fmt.format_name()`。

### 4.2 其他 6 个函数

`decode_layer` / `encode_layer` / `build_header` / `build_footer` / `finalize_header`：
已经使用 `get_format_by_name()`，无需修改。

`merge_bitplanes`：CTB 专用，无变化。

---

## 5. 测试影响

### 5.1 现有测试迁移

**需要修改的测试**（共 10 处，`Format` enum 比较改为字符串比较）：

`fixture_files.rs`（3 处）：

```rust
// 之前
let format = formats::detect_format(&data, Some("goo")).unwrap();
assert_eq!(format, formats::Format::Goo);

// 之后
let fmt = formats::detect_format(&data, Some("goo")).unwrap();
assert_eq!(fmt.format_name(), "goo");
```

`cross_format.rs`（7 处）：

```rust
// 之前
let format = slicer_converter::formats::detect_format(&data, None).unwrap();
assert_eq!(format, slicer_converter::formats::Format::Ctb);

// 之后
let fmt = slicer_converter::formats::detect_format(&data, None).unwrap();
assert_eq!(fmt.format_name(), "ctb");
```

**不受影响的测试**：

- `cross_format.rs` 中 ctb/goo/sl1 直接调用 `decode_layer_data` / `encode_layer_data` 等函数 → 无变化
- `ctb/mod.rs` 单元测试 → 直接调用模块函数，无变化

### 5.2 新增测试

```rust
#[test]
fn test_format_registry_completeness() {
    // 每个注册格式都能通过别名查到
    for desc in formats::FORMATS {
        for alias in desc.aliases {
            let found = formats::get_format_by_name(alias).unwrap();
            assert_eq!(found.format_name(), desc.name);
        }
    }
}

#[test]
fn test_detect_by_magic() {
    // CTB magic
    let ctb_header = [0x19, 0x00, 0xFD, 0x12]; // MAGIC_CBDDLP LE
    let fmt = formats::detect_format(&ctb_header, None).unwrap();
    assert_eq!(fmt.format_name(), "ctb");
}

#[test]
fn test_detect_by_extension() {
    let empty: &[u8] = &[];
    let fmt = formats::detect_format(empty, Some("sl1")).unwrap();
    assert_eq!(fmt.format_name(), "sl1");
}
```

---

## 6. 性能分析

| 操作                        | 当前              | 重构后               | 差异                           |
| --------------------------- | ----------------- | -------------------- | ------------------------------ |
| `get_format_by_name("ctb")` | match 跳转表 O(1) | 线性扫描 3 别名 O(N) | ~2ns（3格式）→ ~20ns（30格式） |
| `detect_format()`           | 3 个 if-else      | 遍历 3 个 try_detect | ~5ns 差异                      |
| `decode_layer`              | trait 虚调用      | 完全不变             | 零                             |
| WASM 二进制                 | ~280KB            | ~282KB               | +0.7%                          |

**结论**：查找路径从 O(1) 变为 O(N)，但 N ≤ 30 时仍在纳秒级，对比层解码的毫秒级操作完全可忽略。

---

## 7. 不做（YAGNI 清单）

以下方案经评估后决定不实施：

| 被否决方案                                        | 理由                                              |
| ------------------------------------------------- | ------------------------------------------------- |
| 声明式宏 `register_formats!()`                    | 宏调试困难；常量数组已足够简单                    |
| 格式特定错误类型 `ConvertError::Ctb(CtbError)`    | `Box<dyn Error>` 在 WASM 中堆分配开销；零当前需求 |
| `FileMetadata.format_specific: serde_json::Value` | 堆分配、类型安全丧失、零用例                      |
| `LayerContext` 泛型化                             | 当前 5 字段 struct 无膨胀风险；添加新字段成本极低 |
| `list_supported_formats()` WASM 导出              | 延后至 JS 端确认需要动态查询                      |
| `DetectResult::Maybe(f32)` 置信度                 | 当前 3 格式 magic 不冲突；未来真冲突时增量添加    |

---

## 8. 文件变更清单

| 文件                          | 操作                                                                                   | 行数估算 |
| ----------------------------- | -------------------------------------------------------------------------------------- | -------- |
| `rust/src/formats/mod.rs`     | **重写**注册表部分（删 Format enum + 3 个 match，加 FormatDesc + FORMATS + 新 detect） | ±60 行   |
| `rust/src/formats/ctb/mod.rs` | **加** `extensions()` + `try_detect()` 到 CtbFormat impl                               | +10 行   |
| `rust/src/formats/goo/mod.rs` | **加** `extensions()` + `try_detect()` 到 GooFormat impl                               | +8 行    |
| `rust/src/formats/sl1/mod.rs` | **加** `extensions()` + `try_detect()` 到 Sl1Format impl                               | +8 行    |
| `rust/src/lib.rs`             | **改** parse_header 中 detect_format 调用方式                                          | ±5 行    |
| `rust/tests/fixture_files.rs` | **改** 3 处 Format enum 比较 → format_name() 字符串比较                                | ±6 行    |
| `rust/tests/cross_format.rs`  | **改** 7 处 Format enum 比较 → format_name() 字符串比较                                | ±14 行   |
| `rust/tests/` 新测试          | **加** 3 个注册表/检测测试                                                             | +30 行   |
| **合计**                      |                                                                                        | ~+145 行 |
