use crate::{
    common::ConvertError,
    transcode::{FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder},
};

use super::goo_family_to_ctb;

/// Fast-path strategy for GOO profile layers targeting CTB.
///
/// The implementation delegates to the shared GOO-family transcoder because
/// this route only depends on the layer RLE payload, not the surrounding
/// container profile.
///
/// GOO profile layer 转 CTB 的快路径策略。实现委托给共享 GOO-family transcoder，因为这条
/// route 只依赖 layer RLE payload，不依赖外围 container profile。
pub(crate) struct GooToCtbTranscoder;

/// Process-wide GOO to CTB strategy instance used by direct tests and registry entries.
///
/// 供直接测试与 registry entry 使用的进程级 GOO to CTB 策略实例。
pub(crate) static GOO_TO_CTB_TRANSCODER: GooToCtbTranscoder = GooToCtbTranscoder;

/// Exported strategy slice for tests that inspect this route module directly.
///
/// 供直接检查此 route module 的测试使用的策略列表。
pub static TRANSCODERS: [&'static dyn LayerTranscoder; 1] = [&GOO_TO_CTB_TRANSCODER];

impl LayerTranscoder for GooToCtbTranscoder {
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

#[cfg(test)]
mod tests {
    use super::GooToCtbTranscoder;
    use crate::{
        formats::{ctb, goo, LayerEncodeContext, LayerOptions},
        transcode::{
            FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder,
        },
    };

    fn request<'a>(
        layer_index: u32,
        width: u32,
        height: u32,
        options: &'a LayerOptions,
    ) -> LayerTranscodeRequest<'a> {
        LayerTranscodeRequest {
            layer_index,
            width,
            height,
            display_width_mm: 0.0,
            display_height_mm: 0.0,
            decode_options: options,
            encode_ctx: LayerEncodeContext {
                layer_index,
                width,
                height,
                layer_metadata: None,
            },
        }
    }

    #[test]
    fn goo_to_ctb_fast_path_applies_and_matches_ctb_encoder_for_black_white_runs() {
        let width = 8;
        let height = 2;
        let pixels = vec![
            0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255,
        ];
        let goo_layer =
            goo::encode_layer_data_with_options(&pixels, width, height, true).expect("encode GOO");
        let expected =
            ctb::encode_layer_data(&pixels, 0, width, height).expect("encode CTB fallback target");
        let options = LayerOptions::default();
        let req = request(0, width, height, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let outcome = GooToCtbTranscoder
            .try_transcode_layer_into(&goo_layer, &req, &mut scratch, &mut out)
            .expect("fast path should not error");

        assert_eq!(outcome, FastPathOutcome::Applied);
        assert_eq!(out, expected);
    }

    #[test]
    fn goo_to_ctb_fast_path_applies_and_matches_ctb_encoder_for_negative_diff_run() {
        let options = LayerOptions::default();
        let req = request(0, 5, 1, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();
        let goo_with_diff_run = vec![goo::LAYER_MAGIC, 0x41, 20, 0xB1, 4, 0];
        let expected_pixels = vec![20, 19, 18, 17, 16];
        let expected =
            ctb::encode_layer_data(&expected_pixels, 0, 5, 1).expect("encode CTB expected layer");

        let outcome = GooToCtbTranscoder
            .try_transcode_layer_into(&goo_with_diff_run, &req, &mut scratch, &mut out)
            .expect("fast path should not error");

        assert_eq!(outcome, FastPathOutcome::Applied);
        assert_eq!(out, expected);
    }

    #[test]
    fn goo_to_ctb_fast_path_applies_and_matches_ctb_encoder_for_positive_diff_run() {
        let options = LayerOptions::default();
        let req = request(0, 5, 1, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();
        let goo_with_diff_run = vec![goo::LAYER_MAGIC, 0x41, 10, 0x92, 4, 0];
        let expected_pixels = vec![10, 12, 14, 16, 18];
        let expected =
            ctb::encode_layer_data(&expected_pixels, 0, 5, 1).expect("encode CTB expected layer");

        let outcome = GooToCtbTranscoder
            .try_transcode_layer_into(&goo_with_diff_run, &req, &mut scratch, &mut out)
            .expect("fast path should not error");

        assert_eq!(outcome, FastPathOutcome::Applied);
        assert_eq!(out, expected);
    }
}
