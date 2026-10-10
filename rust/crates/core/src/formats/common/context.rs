use crate::common::LayerEncodeMetadata;

/// Format-specific per-layer options bag.
///
/// 格式特化的逐层选项聚合结构。通用 `LayerDecodeContext` 只携带几何/索引信息；
/// 任何只对特定格式有意义的参数（例如 CTB 的 AA 位面与加密密钥）都下沉到
/// `LayerOptions`。这样 trait 接口对所有格式对称，调用方统一透明透传该 bag。
///
/// 新增 format-specific 字段时：给 `LayerOptions` 加字段（带合理 `Default`），
/// 相关 decoder 自行消费，其他 decoder 天然忽略。
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayerOptions {
    /// CTB anti-alias level (1 = no AA, >1 = CBDDLP bitplane count).
    pub anti_alias_level: u32,
    /// CTB encryption key (0 = unencrypted).
    pub encryption_key: u32,
}

impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            anti_alias_level: 1,
            encryption_key: 0,
        }
    }
}

impl LayerOptions {
    /// Creates default layer options.
    ///
    /// 创建默认 layer options。
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the CTB anti-alias level.
    ///
    /// 设置 CTB anti-alias level。
    pub fn with_anti_alias_level(mut self, level: u32) -> Self {
        self.anti_alias_level = level;
        self
    }

    /// Sets the CTB encryption key.
    ///
    /// 设置 CTB encryption key。
    pub fn with_encryption_key(mut self, key: u32) -> Self {
        self.encryption_key = key;
        self
    }
}

/// Layer decode context — geometry + format-specific options bag.
///
/// 图层解码上下文：几何/索引通用字段 + format-specific `LayerOptions` bag。
/// trait 层面签名对所有格式对称；单个 decoder 按需读取 `options` 中的字段。
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct LayerDecodeContext {
    /// Zero-based logical layer index.
    ///
    /// 从 0 开始的逻辑 layer index。
    pub layer_index: u32,
    /// Source layer width in pixels.
    ///
    /// 源 layer 像素宽度。
    pub width: u32,
    /// Source layer height in pixels.
    ///
    /// 源 layer 像素高度。
    pub height: u32,
    /// Physical display width in millimeters when the codec needs it.
    ///
    /// codec 需要物理尺寸时使用的显示宽度，单位毫米。
    pub display_width_mm: f32,
    /// Physical display height in millimeters when the codec needs it.
    ///
    /// codec 需要物理尺寸时使用的显示高度，单位毫米。
    pub display_height_mm: f32,
    /// Format-specific options passed through unchanged by generic callers.
    ///
    /// 泛型调用方透明透传的格式特化选项。
    pub options: LayerOptions,
}

impl LayerDecodeContext {
    /// Creates a decode context with logical layer index and pixel dimensions.
    ///
    /// 使用逻辑 layer index 与像素尺寸创建 decode context。
    pub fn new(layer_index: u32, width: u32, height: u32) -> Self {
        Self {
            layer_index,
            width,
            height,
            display_width_mm: 0.0,
            display_height_mm: 0.0,
            options: LayerOptions::default(),
        }
    }

    /// Replaces the format-specific options bag.
    ///
    /// 替换 format-specific options bag。
    pub fn with_options(mut self, options: LayerOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the physical display size in millimeters.
    ///
    /// 设置物理显示尺寸，单位毫米。
    pub fn with_display_size_mm(mut self, width: f32, height: f32) -> Self {
        self.display_width_mm = width;
        self.display_height_mm = height;
        self
    }
}

/// Layer encode context — per-layer parameters for encoding.
///
/// 图层编码上下文——编码时需要的逐层参数。`layer_metadata` 是借用视图，由上层持有
/// backing `FileMetadata` 或 `OwnedLayerEncodeMetadata`。
#[derive(Debug, Clone, Copy, Default)]
pub struct LayerEncodeContext<'a> {
    /// Zero-based logical layer index.
    ///
    /// 从 0 开始的逻辑 layer index。
    pub layer_index: u32,
    /// Source grayscale layer width in pixels.
    ///
    /// 源灰度 layer 像素宽度。
    pub width: u32,
    /// Source grayscale layer height in pixels.
    ///
    /// 源灰度 layer 像素高度。
    pub height: u32,
    /// Optional borrowed per-layer metadata for formats with exposure/motion records.
    ///
    /// 可选的借用式逐层 metadata，供带 exposure/motion record 的格式使用。
    pub layer_metadata: Option<LayerEncodeMetadata<'a>>,
}
