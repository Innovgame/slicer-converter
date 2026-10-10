# 切片文件格式转换器 — Rust + WebAssembly 技术方案

> **状态**：Implemented (v0.1)  
> **语言**：Rust → wasm32-unknown-unknown  
> **目标格式**：CTB (Chitubox) / SL1 (Prusa) / GOO (Elegoo) / PRZ (Phrozen)  
> **使用场景**：独立 Web 应用，浏览器端导入一种切片格式并导出另一种

## 0. 实现快照 (2026-04)

- 已完成：CTB/SL1/GOO/PRZ 头解析、层编解码、双向互转、浏览器 Worker Pool 并行
- 已完成：CTB/GOO 缩略图读写 (RGB15 RLE / RGB565 BE)
- 已完成：PRZ 作为 Goo-family 独立格式接入（禁用 color-diff compression）
- 已完成：CBDDLP AA 位平面解码 + `merge_bitplanes()` 合并
- 已完成：CTB `encryption_key` XOR 对称加解密
- 已完成：File.slice() 流式层读取（CTB/GOO/PRZ 仅加载 Header 区）
- 已完成：WASM SIMD128（编译目标启用 + 位平面 SIMD 合并）
- 测试现状：`pnpm test` 通过 68 项（31 unit + 32 cross-format + 5 sample）

---

## 1. 目标与边界

### 1.1 In-Scope

| 能力      | 说明                                                          |
| --------- | ------------------------------------------------------------- |
| 格式解码  | CTB (v1-v2, CBDDLP)、SL1 (ZIP+PNG+INI)、GOO / PRZ ✅          |
| 格式编码  | 同上四种格式 ✅                                               |
| 格式互转  | 任意两种格式的双向转换（4×3 = 12 个方向） ✅                  |
| 缩略图    | CTB RGB15 RLE / GOO RGB565 BE 预览图解码与编码 ✅             |
| 加密支持  | CTB EncryptionKey XOR 对称加解密 ✅                           |
| CBDDLP AA | 位平面解码 + 合并（支持 AA Level 1-8） ✅                     |
| 流式处理  | File.slice() 按需读取层数据，Header 区域独立解析 ✅           |
| 多线程    | Web Worker Pool (最多 8 worker) + Transferable ArrayBuffer ✅ |
| SIMD 加速 | WASM SIMD 128-bit 位平面合并 + 自动向量化 (png/flate2) ✅     |
| JS 集成   | 通过 wasm-bindgen 暴露 7 个 API 函数 ✅                       |

### 1.2 Out-of-Scope

- 图像编辑/修复操作（Operation 体系）
- 3D 模型切片（Slicer 不在范围）
- GCode 高级生成（仅做必要的 GCode 段转写）
- CTB v3/v4/v5 格式（仅支持 v1-v2 + CBDDLP）
- 35+ 格式全量支持（仅 4 种目标格式）

---

## 2. 架构概览

```
┌──────────────────────────────────────────────────────┐
│                     浏览器 (JS 层)                     │
│                                                        │
│  ┌─────────────────────────────────────────────────┐  │
│  │  File API                                        │  │
│  │  ├─ File.slice() 分片读取源文件                    │  │
│  │  ├─ Transferable ArrayBuffer → Worker             │  │
│  │  └─ Blob([...parts]) 构建输出文件 → download       │  │
│  └────────────────┬────────────────────────────────┘  │
│                   │                                    │
│  ┌────────────────▼────────────────────────────────┐  │
│  │  Worker Pool (N = navigator.hardwareConcurrency)  │  │
│  │  ├─ Worker 1: WASM 实例 (层 0, 4, 8, ...)        │  │
│  │  ├─ Worker 2: WASM 实例 (层 1, 5, 9, ...)        │  │
│  │  ├─ Worker 3: WASM 实例 (层 2, 6, 10, ...)       │  │
│  │  └─ Worker 4: WASM 实例 (层 3, 7, 11, ...)       │  │
│  └────────────────────────────────────────────────┘  │
│                                                        │
│  ┌────────────────────────────────────────────────┐   │
│  │  WASM 模块 (~300 KB gzip)                       │   │
│  │  ┌─────────┐ ┌────────────┐ ┌──────────┐       │   │
│  │  │ Decoder │→│ LayerImage │→│ Encoder  │       │   │
│  │  │ CTB/SL1 │ │ (中间表示)  │ │ CTB/SL1  │       │   │
│  │  │ /GOO/PRZ│ │            │ │ /GOO/PRZ │       │   │
│  │  └─────────┘ └────────────┘ └──────────┘       │   │
│  │  峰值内存: ~25-50 MB / Worker                    │   │
│  └────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────┘
```

### 2.1 数据流

```
Phase 1 — Header 解析 (Main Thread, 同步):
  JS: file.slice(0, headerSize) → ArrayBuffer
  WASM: parse_header(bytes, extensionHint) → FileMetadata { format, layers, resolution, ... }

Phase 2 — 层级并行转换 (Worker Pool):
  FOR EACH layer IN 0..layer_count:
    JS: file.slice(layer.offset, layer.end) → ArrayBuffer
    JS: worker.postMessage({ index, data }, [data])  // Transferable: 零拷贝
    Worker WASM: decode_layer(data, index, format, width, height, anti_alias_level, encryption_key) → pixel_buffer (raw grayscale)
    Worker WASM: encode_layer(pixel_buffer, index, target_format) → encoded_bytes
    Worker: postMessage({ index, encoded }, [encoded.buffer])  // Transferable 返回

Phase 3 — 文件组装 (Main Thread, 同步):
  WASM: build_header(metadata, target_format) → header_bytes
  JS: new Blob([header, layer0, layer1, ..., footer])
  JS: URL.createObjectURL(blob) → 下载链接
```

---

## 3. 模块设计

### 3.1 Monorepo / Crate 结构

```
slicer-converter/
├── package.json
├── pnpm-workspace.yaml
├── rust/
│   ├── Cargo.toml              # 唯一 Cargo manifest
│   ├── .cargo/
│   │   └── config.toml         # WASM target flags (+bulk-memory,+simd128)
│   ├── src/
│   │   ├── lib.rs              # wasm-bindgen 入口 (7 个导出函数, trait dispatch)
│   │   ├── common/
│   │   │   ├── mod.rs
│   │   │   ├── layer.rs        # LayerOffset / ParseResult
│   │   │   ├── metadata.rs     # FileMetadata + Thumbnail 通用参数
│   │   │   └── error.rs        # ConvertError 错误类型
│   │   ├── formats/
│   │   │   ├── mod.rs          # 格式检测 + SlicerFormat trait + 格式注册表
│   │   │   ├── ctb/
│   │   │   │   ├── mod.rs      # CtbFormat trait impl + SIMD merge_bitplanes
│   │   │   │   ├── header.rs   # CtbHeader/PrintParams/LayerDef/LayerDefEx
│   │   │   │   ├── rle.rs      # CBDDLP 7-bit RLE + CTB 变长 RLE (SWAR 优化)
│   │   │   │   ├── preview.rs  # RGB15 RLE 预览图编解码
│   │   │   │   ├── crypto.rs   # XOR 对称加解密
│   │   │   │   ├── decoder.rs  # CTB → FileMetadata + LayerOffsets
│   │   │   │   └── encoder.rs  # FileMetadata → CTB v2 二进制
│   │   │   ├── sl1/
│   │   │   │   ├── mod.rs      # Sl1Format trait impl
│   │   │   │   ├── config.rs   # INI 解析/生成 (手写解析，无外部依赖)
│   │   │   │   ├── decoder.rs  # ZIP+PNG → 灰度像素
│   │   │   │   └── encoder.rs  # 灰度像素 → ZIP+PNG
│   │   │   ├── goo/
│   │   │   │   ├── mod.rs      # GooFormat trait impl
│   │   │   │   ├── header.rs   # GooHeader (大端字节序) + LayerDef
│   │   │   │   ├── rle.rs      # GOO RLE (差值编码 + stride)
│   │   │   │   ├── preview.rs  # RGB565 BE 预览图编解码
│   │   │   │   ├── decoder.rs  # GOO-family → FileMetadata + LayerOffsets
│   │   │   │   └── encoder.rs  # FileMetadata → GOO-family 二进制块
│   │   │   └── prz/
│   │   │       └── mod.rs      # PrzFormat trait impl（复用 Goo 容器，禁用 diff compression）
│   ├── examples/
│   │   ├── generate_fixtures.rs # 生成测试 fixture 文件
│   │   └── bench_rle.rs        # RLE 性能基准测试
│   └── tests/
│       ├── cross_format.rs     # 跨格式转换测试
│       └── fixture_files.rs    # fixture 文件端到端测试
├── packages/
│   └── slicer-wasm/            # wasm-pack 产物与包封装
└── apps/
  └── slicer-app/             # Vite 浏览器测试页
    ├── src/main.ts         # 主界面逻辑
    ├── src/worker.ts       # 并行层解码/编码 Worker
    └── tests/e2e/          # Playwright 主链路测试，上传 rust/tests/fixtures 样例
```

### 3.2 核心类型定义

```rust
// common/layer.rs
/// 单层的中间表示 — 格式无关
pub struct LayerImage {
    /// 灰度像素数据 (0=透明, 255=曝光)，row-major
    pub pixels: Vec<u8>,
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
}

// common/metadata.rs
/// 文件级元数据 — 格式间转换时映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    // --- 打印机参数 ---
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub display_width_mm: f32,
    pub display_height_mm: f32,
    pub machine_z_mm: f32,
    pub machine_name: String,
    #[serde(default)]
    pub mirror_x: bool,
    #[serde(default)]
    pub mirror_y: bool,

    // --- 切片参数 ---
    pub layer_height_mm: f32,
    pub layer_count: u32,
    pub bottom_layer_count: u32,
    pub transition_layer_count: u32,
    /// Anti-alias level (1 = none, >1 = CBDDLP bitplane count)
    #[serde(default = "default_aa_level")]
    pub anti_alias_level: u32,
    /// CTB encryption key (0 = unencrypted)
    #[serde(default)]
    pub encryption_key: u32,

    // --- 曝光参数 ---
    pub exposure_time: f32,
    pub bottom_exposure_time: f32,
    pub light_off_delay: f32,
    pub bottom_light_off_delay: f32,
    pub light_pwm: u16,
    pub bottom_light_pwm: u16,

    // --- 运动参数 ---
    pub lift_height: f32,
    pub lift_speed: f32,
    pub lift_height2: f32,
    pub lift_speed2: f32,
    pub retract_speed: f32,
    pub retract_height2: f32,
    pub retract_speed2: f32,
    pub bottom_lift_height: f32,
    pub bottom_lift_speed: f32,

    // --- 等待时间 (GOO 特有，其他格式映射) ---
    pub wait_time_before_cure: f32,
    pub wait_time_after_cure: f32,
    pub wait_time_after_lift: f32,

    // --- 统计 ---
    pub print_time_sec: u32,
    pub volume_ml: f32,
    pub weight_g: f32,
    pub cost: f32,

    // --- 缩略图 ---
    pub thumbnails: Vec<Thumbnail>,

    // --- 每层覆盖参数 (可选) ---
    pub per_layer_settings: Vec<PerLayerSettings>,
}

/// 每层可覆盖的参数
pub struct PerLayerSettings {
    pub position_z: f32,
    pub exposure_time: Option<f32>,
    pub lift_height: Option<f32>,
    pub lift_speed: Option<f32>,
    pub retract_speed: Option<f32>,
    pub light_pwm: Option<u16>,
    // ... 其他可选覆盖
}

/// 缩略图
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    pub pixels_rgba: Vec<u8>,  // RGBA 格式
}
```

### 3.3 Trait 抽象 (已实现)

```rust
// formats/mod.rs

/// 层解码上下文 — 各格式按需使用的参数
#[derive(Debug, Clone, Default)]
pub struct LayerDecodeContext {
    pub layer_index: u32,
    pub width: u32,
    pub height: u32,
    pub anti_alias_level: u32,  // CTB CBDDLP 用
    pub encryption_key: u32,    // CTB 加密用
}

/// 层编码上下文
#[derive(Debug, Clone, Default)]
pub struct LayerEncodeContext {
    pub layer_index: u32,
    pub width: u32,
    pub height: u32,
  pub layer_metadata: Option<LayerEncodeMetadata>,
}

/// 统一切片格式 trait — 每种格式实现此 trait
pub trait SlicerFormat {
    fn format_name(&self) -> &'static str;
    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError>;
    fn decode_layer(&self, data: &[u8], ctx: &LayerDecodeContext) -> Result<Vec<u8>, ConvertError>;
    fn encode_layer(&self, pixels: &[u8], ctx: &LayerEncodeContext) -> Result<Vec<u8>, ConvertError>;
    fn build_header(&self, metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError>;
    fn finalize_header(&self, header: &[u8], layer_sizes: &[u32]) -> Result<Vec<u8>, ConvertError>;
    fn build_footer(&self) -> Vec<u8>;
}

/// 格式注册表 — 按 enum 或字符串查找
pub fn get_format(format: Format) -> &'static dyn SlicerFormat;
pub fn get_format_by_name(name: &str) -> Result<&'static dyn SlicerFormat, ConvertError>;
// name 支持别名: "ctb"|"cbddlp"|"photon", "goo", "prz", "sl1"|"sl1s"
```

**实现**：`CtbFormat`、`GooFormat`、`PrzFormat`、`Sl1Format` 四个零大小结构体，
各自委托至模块内已有的 `decode_layer_data` / `encode_layer_data` / `build_header_bytes` 等函数。

`lib.rs` 通过 `get_format()` / `get_format_by_name()` 统一分发，
新增格式只需：(1) 实现 `SlicerFormat` trait，(2) 在注册表新增一行。

---

## 4. 格式字节级规范

### 4.1 CTB (Chitubox) 格式

#### 4.1.1 Magic Numbers

| 变体        | Magic (LE)   | 值            |
| ----------- | ------------ | ------------- |
| CBDDLP      | `0x12FD0019` | 318,570,521   |
| CTB         | `0x12FD0086` | 318,570,630   |
| CTBv4       | `0x12FD0106` | 318,570,758   |
| CTBv4_GKtwo | `0xFF220810` | 4,280,420,368 |

#### 4.1.2 Header (128 bytes, Little-Endian)

```
Offset  Size  Type    Field
0x00    4     u32     magic
0x04    4     u32     version (default: 5)
0x08    4     f32     bed_size_x (mm)
0x0C    4     f32     bed_size_y (mm)
0x10    4     f32     bed_size_z (mm)
0x14    4     u32     _reserved1
0x18    4     u32     _reserved2
0x1C    4     f32     total_height_mm
0x20    4     f32     layer_height_mm
0x24    4     f32     exposure_seconds
0x28    4     f32     bottom_exposure_seconds
0x2C    4     f32     light_off_delay
0x30    4     u32     bottom_layer_count
0x34    4     u32     resolution_x
0x38    4     u32     resolution_y
0x3C    4     u32     preview_large_offset
0x40    4     u32     layer_table_offset
0x44    4     u32     layer_count
0x48    4     u32     preview_small_offset
0x4C    4     u32     print_time_sec
0x50    4     u32     projector_type (0=normal, 1=mirrored)
0x54    4     u32     print_params_offset
0x58    4     u32     print_params_size
0x5C    4     u32     anti_alias_level
0x60    2     u16     light_pwm
0x62    2     u16     bottom_light_pwm
0x64    4     u32     encryption_key (0=无加密)
0x68    4     u32     slicer_info_offset
0x6C    4     u32     slicer_info_size
```

#### 4.1.3 PrintParameters (60 bytes, v2+)

```
Offset  Size  Type    Field
0x00    4     f32     bottom_lift_height
0x04    4     f32     bottom_lift_speed
0x08    4     f32     lift_height
0x0C    4     f32     lift_speed
0x10    4     f32     retract_speed
0x14    4     f32     volume_ml
0x18    4     f32     weight_g
0x1C    4     f32     cost_dollars
0x20    4     f32     bottom_light_off_delay
0x24    4     f32     light_off_delay
0x28    4     u32     bottom_layer_count
0x2C    16    pad     _reserved[4]
```

#### 4.1.4 LayerDef (36 bytes per entry)

```
Offset  Size  Type    Field
0x00    4     f32     position_z
0x04    4     f32     exposure_time
0x08    4     f32     light_off_seconds
0x0C    4     u32     data_address
0x10    4     u32     data_size
0x14    4     u32     page_number
0x18    4     u32     table_size (36 or 84)
0x1C    4     u32     _reserved1
0x20    4     u32     _reserved2
```

#### 4.1.5 LayerDefEx (v3+, 额外 48 bytes 紧随 LayerDef)

```
Offset  Size  Type    Field
0x00    4     u32     total_size
0x04    4     f32     lift_height
0x08    4     f32     lift_speed
0x0C    4     f32     lift_height2
0x10    4     f32     lift_speed2
0x14    4     f32     retract_speed
0x18    4     f32     retract_height2
0x1C    4     f32     retract_speed2
0x20    4     f32     rest_time_before_lift
0x24    4     f32     rest_time_after_lift
0x28    4     f32     rest_time_after_retract
0x2C    4     f32     light_pwm
```

#### 4.1.6 RLE 编码 — CBDDLP (7-bit)

```
每字节:
  bit 7     = 颜色标记 (0=黑, 1=白)
  bit [6:0] = 重复次数 (1-125, 上限 RLE8_ENCODING_LIMIT=0x7D)

反走样: 生成 AntiAliasLevel 个位平面
  位平面 i 的阈值 = 256 / aa_level * i - 1
  最终像素 = Σ(位平面值) * (256 / aa_level)
```

#### 4.1.7 RLE 编码 — CTB (变长)

```
字节 0:
  bit 7     = run 标记 (1=run+stride, 0=单像素)
  bit [6:1] = 灰度值 (原始8位 >> 1, 即7位精度)

如果 run 标记 = 1, 后续字节编码 stride:
  0x00-0x7F:     1字节 stride (7 bit, max 127)
  0x80-0xBF:     2字节 stride (14 bit, max 16383)
  0xC0-0xDF:     3字节 stride (20 bit, max 1048575)
  0xE0-0xFF:     4字节 stride (28 bit, max 268435455)

Stride 高位编码方式:
  1-byte: stride & 0x7F
  2-byte: ((stride >> 8) | 0x80), (stride & 0xFF)
  3-byte: ((stride >> 16) | 0xC0), ((stride >> 8) & 0xFF), (stride & 0xFF)
  4-byte: ((stride >> 24) | 0xE0), ((stride >> 16) & 0xFF), ((stride >> 8) & 0xFF), (stride & 0xFF)
```

#### 4.1.8 加密 (已实现 — `ctb/crypto.rs`)

```
init = seed * 0x2d83cdac + 0xd8a83423
key  = (layer_index * 0x1e1530cd + 0xec3d47cd) * init

FOR i in 0..rle_data.len():
  shift = (i & 3) * 8
  k = (key >> shift) & 0xFF
  encrypted[i] = data[i] ^ k
  IF (i + 1) & 3 == 0:
    key += init
```

#### 4.1.9 预览图 — RGB15 RLE

```
每像素 2 字节 (Little-Endian):
  bit [15:11] = R (5 bit)
  bit [10:6]  = G (5 bit, 注意不是6)
  bit [5:1]   = B (5 bit)
  bit 0       = run 标记

如果 bit 0 = 1: 下一个 u16 的低 12 位 = 重复次数 (max 4095)
```

#### 4.1.10 文件布局

```
[0x0000] Header (128B)
[var]    Large Preview (32B header + RLE RGB15 data)
[var]    Small Preview (32B header + RLE RGB15 data)
[var]    PrintParameters (60B, v2+)
[var]    SlicerInfo (variable, v2+ CTB)
[var]    MachineName (variable string)
[var]    PrintParametersV4 (variable, v4+)
[var]    Disclaimer (320B, v4+)
[var]    ResinParameters (variable, v5+)
[var]    Layer Table (36B or 84B × layer_count)
[var]    LayerDefEx (48B × layer_count, v3+)
[var]    Layer RLE Data (variable per layer)
```

---

### 4.2 GOO (Elegoo) 格式

> `PRZ` 复用同一 Goo-family 容器与 header/layer/footer 布局，
> 与 `GOO` 的主要差异是编码时禁用 color-diff compression，并依赖扩展名提示完成格式区分。

#### 4.2.1 常量

```
Magic: [0x07, 0x00, 0x00, 0x00, 0x44, 0x4C, 0x50, 0x00]
Version: "V3.0"
Delimiter: [0x0D, 0x0A]
Layer Magic: 0x55
```

#### 4.2.2 Header (~488 bytes, 混合大小端)

```
Offset  Size    Type       Endian  Field
0x00    4       string     BE      version ("V3.0")
0x04    8       bytes      BE      magic
0x0C    32      string     LE      software_name (null-terminated)
0x2C    24      string     LE      software_version
0x44    24      string     LE      file_create_time
0x5C    32      string     LE      machine_name
0x7C    32      string     LE      machine_type
0x9C    32      string     LE      profile_name
0xBC    2       u16        BE      anti_aliasing_level
0xBE    2       u16        BE      grey_level
0xC0    2       u16        BE      blur_level
0xC2    26912   bytes      BE      small_preview_565 (116×116 RGB565)
0x69E4  2       bytes      BE      delimiter [0x0D, 0x0A]
0x69E6  168200  bytes      BE      big_preview_565 (290×290 RGB565)
(+168200)                          再跟 2B delimiter
...     4       u32        BE      layer_count
...     2       u16        BE      resolution_x
...     2       u16        BE      resolution_y
...     1       bool       BE      mirror_x
...     1       bool       BE      mirror_y
...     4       f32        BE      display_width_mm
...     4       f32        BE      display_height_mm
...     4       f32        BE      machine_z_mm
...     4       f32        BE      layer_height_mm
...     4       f32        BE      exposure_time
...     1       u8         BE      delay_mode (0=LightOff, 1=WaitTime)
...     4       f32        BE      light_off_delay
...     ~56     f32 × 14   BE      各种 lift/retract 参数
...     2+2     u16 × 2    BE      bottom_light_pwm, light_pwm
...     1       bool       BE      per_layer_settings
...     4       u32        BE      print_time_sec
...     4       f32        BE      volume_mm3
...     4       f32        BE      material_grams
...     4       f32        BE      material_cost
...     8       string     BE      currency_symbol
...     4       u32        BE      layer_def_address
...     1       u8         BE      gray_scale_level (0=4bit, 1=8bit)
...     2       u16        BE      transition_layer_count
```

#### 4.2.3 LayerDef (74 bytes + RLE data, Big-Endian)

```
Offset  Size  Type    Field
0x00    2     u16     pause_flag (0=继续, 1=暂停)
0x02    4     f32     pause_position_z
0x06    4     f32     position_z
0x0A    4     f32     exposure_time
0x0E    4     f32     light_off_delay
0x12    4     f32     wait_time_after_cure
0x16    4     f32     wait_time_after_lift
0x1A    4     f32     wait_time_before_cure
0x1E    4     f32     lift_height
0x22    4     f32     lift_speed
0x26    4     f32     lift_height2
0x2A    4     f32     lift_speed2
0x2E    4     f32     retract_height
0x32    4     f32     retract_speed
0x36    4     f32     retract_height2
0x3A    4     f32     retract_speed2
0x3E    2     u16     light_pwm
0x40    2     bytes   delimiter [0x0D, 0x0A]
0x42    4     u32     data_length
0x46    var   bytes   RLE data (data_length bytes)
```

#### 4.2.4 RLE 编码 (GOO)

```
字节 0: 0x55 (magic)

后续字节编码:
  bit [7:6] = chunk 类型
    0b00 = 全黑 (0x00) + stride
    0b01 = 灰度值 (下一字节为灰度) + stride
    0b10 = 差值编码 (相对前一像素)
    0b11 = 全白 (0xFF) + stride

  对于类型 00/01/11 (非差值):
    bit [5:4] = stride 长度类型
      0b00 = 4-bit stride (bit[3:0], max 15)
      0b01 = 12-bit stride (1 extra byte, max 4095)
      0b10 = 20-bit stride (2 extra bytes, max 1048575)
      0b11 = 28-bit stride (3 extra bytes, max 268435455)
    bit [3:0] = stride 低 4 位

  对于类型 10 (差值):
    bit [5:4] = 差值模式
      0b00 = 正差值, 单像素
      0b01 = 正差值 + run (下一字节 = 重复次数)
      0b10 = 负差值, 单像素
      0b11 = 负差值 + run
    bit [3:0] = 差值大小 (0-15)

末尾字节: Fletcher checksum
  checksum = (~Σ(所有数据字节)) & 0xFF
```

#### 4.2.5 RGB565 预览图

```
每像素 2 字节 (Big-Endian):
  bit [15:11] = R (5 bit)
  bit [10:5]  = G (6 bit)
  bit [4:0]   = B (5 bit)

小预览: 116 × 116 = 26,912 bytes
大预览: 290 × 290 = 168,200 bytes
```

#### 4.2.6 文件布局

```
[0x0000]              Header (~488B)
[layer_def_address]   LayerDef[0] (74B + RLE data)
[...]                 LayerDef[1] (74B + RLE data)
...
[...]                 LayerDef[N-1]
[EOF - 11]            Footer: 3B padding + 8B magic
```

---

### 4.3 SL1 (Prusa) 格式

#### 4.3.1 ZIP 归档结构

```
archive.sl1 (ZIP)
├── config.ini              # 主配置文件
├── prusaslicer.ini         # 切片器/打印机/材料参数
├── <filename>.thumbnails/
│   ├── thumbnail400x400.png
│   └── thumbnail800x480.png
├── <filename>00001.png     # Layer 0
├── <filename>00002.png     # Layer 1
└── ...
```

#### 4.3.2 config.ini 关键字段

```ini
action = print
job_dir = <filename>
exp_time = <float>               # 普通层曝光 (sec)
exp_time_first = <float>         # 底层曝光 (sec)
layer_height = <float>           # 层高 (mm)
num_fade = <u16>                 # 底层数
num_fast = <u32>                 # 普通层数
num_slow = <u16>                 # 过渡层数
print_time = <float>             # 打印时间 (sec)
printer_model = SL1              # 打印机型号
used_material = <float>          # 树脂用量 (mm³)
material_name = <string>         # 材料名称
```

#### 4.3.3 prusaslicer.ini 关键字段

```ini
# Printer
display_width = <float>          # 显示屏宽 (mm)
display_height = <float>         # 显示屏高 (mm)
display_pixels_x = <u32>         # X 分辨率
display_pixels_y = <u32>         # Y 分辨率
max_print_height = <float>       # 最大 Z (mm)
display_mirror_x = <0|1>
display_mirror_y = <0|1>

# Material
exposure_time = <float>
initial_exposure_time = <float>
material_density = <float>
bottle_cost = <float>
bottle_volume = <float>
```

#### 4.3.4 额外参数注入 (通过 notes 字段)

```
printer_notes 或 material_notes 中的格式:
  KeyName_value
例如:
  BottomLiftHeight_5.0
  LiftHeight_8.0
  WaitTimeBeforeCure_1.0
```

#### 4.3.5 层图像格式

```
PNG, 8 位灰度
0 = 未曝光 (透明)
255 = 完全曝光
文件名: <job_dir>NNNNN.png (5 位补零)
```

---

## 5. WASM 接口设计

### 5.1 wasm-bindgen 导出 API (7 个函数)

所有函数内部通过 `get_format()` / `get_format_by_name()` 查找对应的 `SlicerFormat` trait
实现进行分发，无 match 硬编码。

```rust
use wasm_bindgen::prelude::*;

/// 解析文件头部，返回 JSON 格式的元数据 + 层偏移表
/// 自动检测格式 (CTB magic / Goo-family magic)，共享容器场景会结合 extension_hint 区分 GOO/PRZ
#[wasm_bindgen]
pub fn parse_header(data: &[u8], extension_hint: Option<String>) -> Result<JsValue, JsError> {
    // 返回 { metadata: FileMetadata, layers: LayerOffset[], format: string }
}

/// 解码单层数据为灰度像素 (Uint8Array)
/// CTB 加密文件传入 encryption_key (从 metadata 获取), 非加密传 0
#[wasm_bindgen]
pub fn decode_layer(
    data: &[u8],          // 该层的原始字节 (RLE/PNG/ZIP entry)
    layer_index: u32,
    format: &str,         // "ctb" | "sl1" | "goo" | "prz"
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,  // CTB 加密密钥，0=无加密
) -> Result<Vec<u8>, JsError> {
    // 返回 width * height 的灰度像素数组
}

/// 编码单层灰度像素为目标格式
#[wasm_bindgen]
pub fn encode_layer(
    pixels: &[u8],        // 灰度像素 (width * height)
    layer_index: u32,
  target_format: &str,  // "ctb" | "sl1" | "goo" | "prz"
    width: u32,
    height: u32,
  layer_metadata_json: Option<String>, // GOO/PRZ 输出完整 layer block 时使用
) -> Result<Vec<u8>, JsError> {
    // 返回 RLE/PNG 编码后的字节
}

/// 构建目标格式的文件头
#[wasm_bindgen]
pub fn build_header(
    metadata_json: &str,     // JSON 格式的 FileMetadata
    target_format: &str,
) -> Result<Vec<u8>, JsError> {
  // 返回 header bytes (CTB: 128B+previews+params+table, GOO/PRZ: ~488B+previews, SL1: INI as bytes)
}

/// 构建目标格式的文件尾
#[wasm_bindgen]
pub fn build_footer(target_format: &str) -> Result<Vec<u8>, JsError> {
  // GOO/PRZ: 11B footer; CTB/SL1: empty
}

/// 回填文件头中的层数据偏移/大小 (所有层编码完成后调用)
#[wasm_bindgen]
pub fn finalize_header(
    header_bytes: &[u8],
    target_format: &str,
    layer_sizes: &[u32],     // 每层编码后的字节数
) -> Result<Vec<u8>, JsError> {
    // CTB: 修补 LayerDef 表中的 data_address/data_size
  // GOO/PRZ/SL1: 原样返回
}

/// 合并 CBDDLP 抗锯齿位平面 (JS 端拼接后传入)
#[wasm_bindgen]
pub fn merge_bitplanes(
    planes_flat: &[u8],      // 所有位平面拼接 (plane_count × pixel_count)
    plane_count: u32,
    pixel_count: u32,
) -> Result<Vec<u8>, JsError> {
    // 像素加权合并: weight = 256 / plane_count, saturating add
}
```

### 5.2 JS 侧使用示例

```javascript
import init, { parse_header, decode_layer, encode_layer, build_header } from "./pkg";

await init(); // 加载 WASM

// 1. 用户选择文件
const file = document.getElementById("fileInput").files[0];

// 2. 读取并解析 header (CTB/GOO/PRZ: 先读 512KB, 不足再扩展; SL1: 全量)
const ext = file.name.split(".").pop().toLowerCase();
const headerChunk = new Uint8Array(await file.slice(0, 512 * 1024).arrayBuffer());
const { metadata, layers, format } = parse_header(headerChunk, ext);

// 3. 创建 Worker Pool
const WORKERS = navigator.hardwareConcurrency || 4;
const pool = Array.from({ length: WORKERS }, () => new Worker("worker.ts", { type: "module" }));

// 4. 分发层任务
const encoded = new Array(layers.length);
let next = 0;

for (const worker of pool) {
    worker.onmessage = async (e) => {
        encoded[e.data.index] = e.data.result;
        if (next < layers.length) dispatch(worker);
        else if (encoded.every(Boolean)) assemble();
    };
    dispatch(worker);
}

function dispatch(worker) {
    const i = next++;
    file.slice(layers[i].offset, layers[i].offset + layers[i].size)
        .arrayBuffer()
        .then((buf) => {
            worker.postMessage(
                {
                    index: i,
                    data: buf,
                    sourceFormat: format,
                    targetFormat: "prz",
                    width: metadata.resolution_x,
                    height: metadata.resolution_y,
                    antiAliasLevel: metadata.anti_alias_level || 1,
                    encryptionKey: metadata.encryption_key || 0
                },
                [buf] // Transferable
            );
        });
}

function assemble() {
    const header = build_header(JSON.stringify(metadata), "prz");
    const footer = build_footer("prz");
    const blob = new Blob([header, ...encoded, footer], { type: "application/octet-stream" });
    const url = URL.createObjectURL(blob);
    // 触发下载...
}
```

---

## 6. SIMD 加速策略

### 6.1 自动获取 (库级别)

| 库                     | SIMD 加速 | 说明                                |
| ---------------------- | --------- | ----------------------------------- |
| `png` crate            | ✅ 自动   | PNG filter 重建 (Paeth/Sub/Average) |
| `flate2` (miniz_oxide) | ✅ 自动   | deflate 压缩/解压                   |

编译配置 `rust/.cargo/config.toml`:

```toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "target-feature=+bulk-memory,+simd128"]
```

### 6.2 手写 SIMD — CBDDLP 位平面叠加 (已实现)

位于 `rust/src/formats/ctb/mod.rs` 的 `merge_bitplanes()` 函数中：

```rust
#[cfg(target_arch = "wasm32")]
{
    use std::arch::wasm32::*;
    let weight_v = u8x16_splat(weight);
    let zero = u8x16_splat(0);
    let simd_len = pixel_count & !15; // 16 字节对齐

    for i in (0..simd_len).step_by(16) {
        let mut acc = zero;
        for plane in planes.iter() {
            let px = unsafe { v128_load(plane[i..].as_ptr() as *const v128) };
            let mask = u8x16_ne(px, zero);
            let weighted = v128_and(mask, weight_v);
            acc = u8x16_add_sat(acc, weighted);
        }
        unsafe { v128_store(output[i..].as_mut_ptr() as *mut v128, acc) };
    }
    // 尾部标量处理 simd_len..pixel_count
}

#[cfg(not(target_arch = "wasm32"))]
{
    // 标量回退 (用于原生测试)
    for (i, out) in output.iter_mut().enumerate() {
        let sum: u16 = planes.iter()
            .filter(|p| i < p.len() && p[i] > 0)
            .count() as u16 * weight as u16;
        *out = sum.min(255) as u8;
    }
}
```

#### 6.2.2 灰度 → RGB565 (未实现 — 预览图较小，收益不大)

预览图尺寸 (116×116 / 290×290) 较小，标量转换已够用。如需实现：

```rust
/// SIMD 灰度转 RGB565 (8 像素/次)
pub fn gray_to_rgb565_simd(gray: &[u8], out: &mut [u8]) {
    for i in (0..gray.len()).step_by(8) {
        let g = v128_load64_zero(gray[i..].as_ptr() as *const v128);
        let g16 = u16x8_extend_low_u8x16(g);
        let r = u16x8_shr(g16, 3);
        let g_part = u16x8_shr(g16, 2);
        let b = u16x8_shr(g16, 3);
        let rgb = u16x8_or(u16x8_or(u16x8_shl(r, 11), u16x8_shl(g_part, 5)), b);
        v128_store(out[i * 2..].as_mut_ptr() as *mut v128, rgb);
    }
}
```

---

## 7. 多线程架构

### 7.1 Worker 通信协议

```typescript
// worker.ts → main thread
interface WorkerMessage {
    type: "ready" | "layer-done" | "error";
    index?: number;
    result?: ArrayBuffer; // Transferable
    error?: string;
}

// main thread → worker.ts
interface TaskMessage {
    type: "decode-encode";
    index: number;
    data: ArrayBuffer; // Transferable
    sourceFormat: "ctb" | "sl1" | "goo" | "prz";
    targetFormat: "ctb" | "sl1" | "goo" | "prz";
    width: number;
    height: number;
    antiAliasLevel?: number;
    encryptionKey?: number;
}
```

### 7.2 Worker 实现

```typescript
// worker.ts
import { decode_layer, encode_layer, initSlicerConverter } from "@innovgame/slicer-converter";

let ready = false;

initSlicerConverter().then(() => {
    ready = true;
    postMessage({ type: "ready" });
});

onmessage = (e) => {
    if (!ready) return;
    const { index, data, sourceFormat, targetFormat, width, height, antiAliasLevel, encryptionKey } = e.data;

    try {
        const pixels = decode_layer(
            new Uint8Array(data),
            index,
            sourceFormat,
            width,
            height,
            antiAliasLevel || 1,
            encryptionKey || 0
        );
        const encoded = encode_layer(pixels, index, targetFormat, width, height);
        const buffer = encoded.buffer;
        postMessage({ type: "layer-done", index, result: buffer }, [buffer]);
    } catch (err) {
        postMessage({ type: "error", index, error: err.message });
    }
};
```

### 7.3 进度报告

```javascript
// main thread
let completed = 0;
const total = layers.length;

worker.onmessage = (e) => {
    if (e.data.type === "layer-done") {
        completed++;
        progressCallback(completed / total); // 0.0 → 1.0
    }
};
```

---

## 8. 依赖清单

### 8.1 Cargo.toml (实际)

```toml
[package]
name = "slicer-converter"
version = "0.1.0"
edition = "2021"

[package.metadata.wasm-pack.profile.release]
wasm-opt = false                  # wasm-opt 与 SIMD128 兼容性问题，暂禁用

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"        # JS ↔ Rust 结构体序列化
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
binrw = "0.15"                    # 声明式二进制解析
png = "0.18"                      # PNG 编解码 (含 SIMD filter)
flate2 = "1"                      # deflate/zlib
zip = { version = "8", default-features = false, features = ["deflate"] }
thiserror = "2"                   # 错误类型

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"                   # 最小包体
```

> **注**：SL1 INI 解析采用手写解析器（`sl1/config.rs`），不依赖外部 configparser crate。

### 8.2 包体实测

| 指标                 | 值                      |
| -------------------- | ----------------------- |
| WASM 二进制大小      | ~540 KB (未 gzip)       |
| gzip 后估计          | ~180-200 KB             |
| 编译 target features | `+bulk-memory,+simd128` |

---

## 9. 格式间参数映射

| 参数            | CTB                            | GOO                        | SL1                                 |
| --------------- | ------------------------------ | -------------------------- | ----------------------------------- |
| resolution_x    | `Header.ResolutionX`           | `Header.ResolutionX`       | `prusaslicer.ini: display_pixels_x` |
| resolution_y    | `Header.ResolutionY`           | `Header.ResolutionY`       | `prusaslicer.ini: display_pixels_y` |
| layer_height    | `Header.LayerHeightMillimeter` | `Header.LayerHeight`       | `config.ini: layer_height`          |
| exposure_time   | `Header.LayerExposureSeconds`  | `Header.ExposureTime`      | `config.ini: exp_time`              |
| bottom_exposure | `Header.BottomExposureSeconds` | (per-layer)                | `config.ini: exp_time_first`        |
| bottom_layers   | `Header.BottomLayersCount`     | (per-layer 推断)           | `config.ini: num_fade`              |
| lift_height     | `PrintParams.LiftHeight`       | `Header.LiftHeight`        | notes: `LiftHeight_<v>`             |
| lift_speed      | `PrintParams.LiftSpeed`        | `Header.LiftSpeed`         | notes: `LiftSpeed_<v>`              |
| 每层覆盖        | v3+ LayerDefEx                 | LayerDef (always)          | ❌ (不支持)                         |
| 反走样          | `Header.AntiAliasLevel`        | `Header.AntiAliasingLevel` | PNG 灰度即 AA                       |
| 灰度精度        | 7-bit (CTB) / 1-bit (CBDDLP)   | 8-bit (0xFF mode)          | 8-bit PNG                           |
| 缩略图          | RGB15 RLE                      | RGB565 raw                 | PNG                                 |

**映射策略**：

- CTB → GOO: 灰度 7-bit → 8-bit（左移 1 位），底层参数拆入每层 LayerDef
- GOO → CTB: 灰度 8-bit → 7-bit（右移 1 位），合并每层参数为全局默认
- SL1 → 任一: PNG 解码即得 8-bit 灰度，直接编码目标 RLE
- 任一 → SL1: 灰度 → PNG 编码，合并参数写 config.ini

---

## 10. 实现路线图

### Phase 1 — 基础框架 + CTB (MVP) ✅

- [x] 项目骨架: `rust/Cargo.toml` + wasm-pack 配置
- [x] `common/` 模块: LayerOffset, FileMetadata, ConvertError
- [x] CTB Header/PrintParams/LayerDef 结构体 (binrw 0.15)
- [x] CTB 变长 RLE 解码/编码 (SWAR 8-byte 并行优化，6.3x 加速)
- [x] CTB v2 整体 Decode/Encode
- [x] wasm-bindgen 入口 (parse_header, decode_layer, encode_layer, build_header, build_footer, finalize_header, merge_bitplanes)
- [x] CTB → CTB roundtrip 测试
- [x] 浏览器 Web 演示（现位于 `apps/slicer-app/`，含 Worker Pool + 进度条 + 拖拽上传）

### Phase 2 — SL1 + Goo-family ✅

- [x] SL1 解码: ZIP 解析 + INI 手写解析 + PNG 层解码
- [x] SL1 编码: ZIP 打包 + INI 生成 + PNG 编码
- [x] GOO Header 结构体 (大端 binrw)
- [x] GOO RLE 解码/编码 (差值 + stride)
- [x] GOO 整体 Decode/Encode
- [x] PRZ 独立格式接入（共享 Goo-family 容器，禁用 diff compression）
- [x] 32 个跨格式转换测试（12 方向 + 边界情况）

### Phase 3 — 多线程 + SIMD + 大文件 ✅

- [x] Worker Pool 实现（最多 8 workers，现位于 `apps/slicer-app/src/worker.ts`）
- [x] File.slice() 流式读取 (CTB/GOO/PRZ Header 区域独立解析，层数据按需加载)
- [x] Blob 分段组装输出 (零拷贝)
- [x] SIMD 位平面合并 (CBDDLP AA, u8x16 16像素/次)
- [x] `+simd128` 全局编译 flag (png/flate2 自动向量化)
- [x] 进度回调 (层级粒度)
- [x] RLE 性能基准测试 (`rust/examples/bench_rle.rs`)

### Phase 4 — 完善 ✅

- [x] CBDDLP 格式支持 (7-bit RLE + AA 位平面合并)
- [x] CTB XOR 对称加解密 (ctb/crypto.rs)
- [x] 缩略图: CTB RGB15 RLE + GOO RGB565 BE (双向转换 + 最近邻缩放)
- [x] 元数据扩展: mirror_x/y, anti_alias_level, encryption_key

### 待办 (P2)

- [ ] CTB v3/v4/v5 扩展参数 (LayerDefEx 48B)
- [ ] SIMD 灰度 ↔ RGB565 色空间转换
- [ ] 错误恢复 (损坏文件 partial decode)
- [ ] npm 包发布
- [ ] TypeScript 类型定义生成

---

## 11. 验收标准

| 项目       | 标准                                                                             | 现状                            |
| ---------- | -------------------------------------------------------------------------------- | ------------------------------- |
| 功能正确性 | CTB/SL1/GOO/PRZ 四格式 roundtrip 无损，交叉转换输出可被 UVtools / 浏览器链路接受 | ✅ 68 个测试全部通过            |
| 包体大小   | gzip 后 < 200 KB                                                                 | ✅ ~540 KB 未压缩，gzip ~180 KB |
| 单线程性能 | 4K 单层 RLE 编码 < 5ms                                                           | ✅ 2.11ms/层 (SWAR 优化)        |
| 内存占用   | 峰值 < 100 MB (单 Worker)                                                        | ✅ File.slice() 流式读取        |
| 浏览器兼容 | Chrome 91+, Firefox 89+, Safari 16.4+ (需 SIMD128 支持)                          | ✅                              |
