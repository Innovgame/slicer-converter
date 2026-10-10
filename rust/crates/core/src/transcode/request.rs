use crate::{
    common::ConvertError,
    formats::{LayerEncodeContext, LayerOptions},
};

/// Per-layer request passed to fast-path transcoders.
///
/// This bundles the geometry, decode options, and target encode context so a
/// strategy can decide whether it can transform the encoded payload directly.
///
/// 传给快路径 transcoder 的逐层请求。它聚合几何信息、decode options 与目标 encode context，
/// 让策略判断是否可以直接转换 encoded payload。
#[derive(Debug, Clone)]
pub struct LayerTranscodeRequest<'a> {
    /// Zero-based logical layer index.
    ///
    /// 从 0 开始的逻辑 layer index。
    pub layer_index: u32,
    /// Layer width in pixels.
    ///
    /// layer 像素宽度。
    pub width: u32,
    /// Layer height in pixels.
    ///
    /// layer 像素高度。
    pub height: u32,
    /// Physical display width in millimeters.
    ///
    /// 物理显示宽度，单位毫米。
    pub display_width_mm: f32,
    /// Physical display height in millimeters.
    ///
    /// 物理显示高度，单位毫米。
    pub display_height_mm: f32,
    /// Source decode options.
    ///
    /// 源格式 decode options。
    pub decode_options: &'a LayerOptions,
    /// Target encode context.
    ///
    /// 目标格式 encode context。
    pub encode_ctx: LayerEncodeContext<'a>,
}

/// Reusable scratch buffers for one layer transcode operation.
///
/// Fast paths may use `aux_buf`; fallback uses `pixel_buf` for decoded grayscale
/// pixels. Callers keep the allocation across layers.
///
/// 单层转码操作的可复用 scratch buffers。快路径可使用 `aux_buf`；fallback 使用 `pixel_buf`
/// 保存解码后的灰度像素。调用方可跨层保留分配。
#[derive(Debug, Default)]
pub struct LayerTranscodeScratch {
    /// Reusable grayscale pixel buffer.
    ///
    /// 可复用灰度像素 buffer。
    pub pixel_buf: Vec<u8>,
    /// Reusable auxiliary buffer for fast paths.
    ///
    /// 快路径使用的可复用辅助 buffer。
    pub aux_buf: Vec<u8>,
}

impl LayerTranscodeScratch {
    /// Creates scratch buffers with initial pixel capacity.
    ///
    /// 创建带初始像素容量的 scratch buffers。
    pub fn new(pixel_capacity: usize) -> Self {
        Self {
            pixel_buf: Vec::with_capacity(pixel_capacity),
            aux_buf: Vec::new(),
        }
    }
}

/// Outcome returned by a fast-path transcoder.
///
/// `Fallback` is a clean decline, not an error; the engine may try the next
/// strategy or the generic decode/encode fallback.
///
/// 快路径 transcoder 返回的结果。`Fallback` 是正常拒绝，不是错误；engine 可以尝试下一个策略，
/// 或使用通用 decode/encode fallback。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastPathOutcome {
    /// Strategy wrote the output payload.
    ///
    /// 策略已写出目标 payload。
    Applied,
    /// Strategy declined this input.
    ///
    /// 策略拒绝当前输入。
    Fallback,
}

/// Strategy that may transcode one encoded layer without full decode/encode.
///
/// Implementations must leave `out` containing the target encoded payload only
/// when returning `Applied`.
///
/// 可尝试绕过完整 decode/encode 的单层转码策略。实现只有在返回 `Applied` 时，才应让 `out`
/// 包含目标 encoded payload。
pub trait LayerTranscoder: Sync {
    /// Attempts to transcode a source encoded layer into target encoded bytes.
    ///
    /// 尝试将源 encoded layer 转为目标 encoded bytes。
    fn try_transcode_layer_into(
        &self,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<FastPathOutcome, ConvertError>;
}
