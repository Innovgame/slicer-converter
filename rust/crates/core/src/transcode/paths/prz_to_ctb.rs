use crate::{
    common::ConvertError,
    transcode::{FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder},
};

use super::goo_family_to_ctb;

/// Fast-path strategy for PRZ profile layers targeting CTB.
///
/// PRZ reuses the GOO-family layer RLE payload for this conversion, so the
/// strategy delegates to the shared implementation.
///
/// PRZ profile layer 转 CTB 的快路径策略。PRZ 在该转换中复用 GOO-family layer RLE
/// payload，因此策略委托给共享实现。
pub(crate) struct PrzToCtbTranscoder;

/// Process-wide PRZ to CTB strategy instance used by direct tests and registry entries.
///
/// 供直接测试与 registry entry 使用的进程级 PRZ to CTB 策略实例。
pub(crate) static PRZ_TO_CTB_TRANSCODER: PrzToCtbTranscoder = PrzToCtbTranscoder;

/// Exported strategy slice for tests that inspect this route module directly.
///
/// 供直接检查此 route module 的测试使用的策略列表。
pub static TRANSCODERS: [&'static dyn LayerTranscoder; 1] = [&PRZ_TO_CTB_TRANSCODER];

impl LayerTranscoder for PrzToCtbTranscoder {
    fn try_transcode_layer_into(
        &self,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<FastPathOutcome, ConvertError> {
        goo_family_to_ctb::try_transcode_layer_into(src, req, scratch, out)
    }
}
