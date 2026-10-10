use crate::{common::ConvertError, formats::common::FormatSpec};

use super::{
    fallback,
    registry::{TranscodeRegistry, DEFAULT_REGISTRY},
    FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscodeStats,
    TranscodeRouteDescriptor,
};

/// Layer transcode engine.
///
/// The engine attempts route-specific fast paths first. If every strategy
/// cleanly declines, it falls back to decoding the source layer to grayscale and
/// encoding it with the target profile.
///
/// Fast-path lookup uses [`TranscodeRouteDescriptor::route_key`]; the full
/// descriptor is still built here so stats/tests can keep route diagnostics
/// aligned with the concrete source and target specs.
///
/// 逐层转码引擎。engine 先尝试 route-specific 快路径；如果所有策略都正常拒绝，则回退到
/// 源层解码为灰度再用目标 profile 编码。
/// 快路径 lookup 使用 [`TranscodeRouteDescriptor::route_key`]；这里仍构建完整 descriptor，
/// 让 stats/tests 的 route diagnostics 与具体 source/target specs 保持一致。
pub struct TranscodeEngine {
    registry: &'static dyn TranscodeRegistry,
}

impl TranscodeEngine {
    /// Creates an engine backed by the given registry.
    ///
    /// 创建由指定 registry 支撑的 engine。
    pub const fn new(registry: &'static dyn TranscodeRegistry) -> Self {
        Self { registry }
    }

    /// Transcodes one layer and discards decision stats.
    ///
    /// 转码单层并丢弃决策统计。
    #[allow(clippy::too_many_arguments)]
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

    /// Transcodes one layer and returns fast-path/fallback stats.
    ///
    /// A fast-path error is surfaced immediately; only [`FastPathOutcome::Fallback`]
    /// means the engine should try the next strategy or generic fallback.
    ///
    /// 转码单层并返回快路径/fallback 统计。
    /// 快路径返回 error 会立即向上传播；只有 [`FastPathOutcome::Fallback`] 表示 engine 应尝试
    /// 下一个策略或通用 fallback。
    #[allow(clippy::too_many_arguments)]
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

/// Returns the process-wide default transcode engine.
///
/// 返回进程级默认 transcode engine。
pub fn default_engine() -> &'static TranscodeEngine {
    &DEFAULT_ENGINE
}
