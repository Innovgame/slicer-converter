use crate::{
    common::ConvertError,
    formats::{common::FormatSpec, LayerDecodeContext, LayerEncodeContext},
};

use super::{default_engine, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscodeStats};

/// Borrowed buffers supplied by high-level layer execution.
///
/// The function temporarily moves these buffers into `LayerTranscodeScratch` and
/// restores them before returning, including on errors.
///
/// 高层 layer execution 提供的借用 buffers。函数会临时将这些 buffer 移入
/// `LayerTranscodeScratch`，并在返回前恢复，即使发生错误也是如此。
pub struct LayerExecutionBuffers<'a> {
    /// Reusable decoded pixel buffer.
    ///
    /// 可复用 decoded pixel buffer。
    pub pixel_buf: &'a mut Vec<u8>,
    /// Reusable auxiliary buffer.
    ///
    /// 可复用辅助 buffer。
    pub aux_buf: &'a mut Vec<u8>,
    /// Target encoded payload output.
    ///
    /// 目标 encoded payload 输出。
    pub out: &'a mut Vec<u8>,
}

/// Executes one layer transcode through the default engine and returns stats.
///
/// This is the high-level helper used by callers that already own reusable
/// buffers. It preserves those allocations by moving them into scratch for the
/// duration of the engine call and moving them back before returning.
///
/// 执行一次默认 engine 单层转码并返回统计。
/// 这是面向已持有 reusable buffers 的高层 helper。它会在 engine 调用期间把这些分配移入
/// scratch，并在返回前移回，从而保留 allocation。
pub fn execute_layer_into_with_stats(
    src_fmt: &FormatSpec,
    tgt_fmt: &FormatSpec,
    src: &[u8],
    decode_ctx: &LayerDecodeContext,
    encode_ctx: &LayerEncodeContext<'_>,
    buffers: LayerExecutionBuffers<'_>,
) -> Result<LayerTranscodeStats, ConvertError> {
    let req = LayerTranscodeRequest {
        layer_index: decode_ctx.layer_index,
        width: decode_ctx.width,
        height: decode_ctx.height,
        display_width_mm: decode_ctx.display_width_mm,
        display_height_mm: decode_ctx.display_height_mm,
        decode_options: &decode_ctx.options,
        encode_ctx: *encode_ctx,
    };
    let mut scratch = LayerTranscodeScratch {
        pixel_buf: std::mem::take(buffers.pixel_buf),
        aux_buf: std::mem::take(buffers.aux_buf),
    };
    let result = default_engine().transcode_layer_into_with_stats(
        src_fmt,
        tgt_fmt,
        src,
        &req,
        &mut scratch,
        buffers.out,
    );
    *buffers.pixel_buf = scratch.pixel_buf;
    *buffers.aux_buf = scratch.aux_buf;
    result
}

/// Executes one layer transcode through the default engine.
///
/// Convenience wrapper around [`execute_layer_into_with_stats`] for callers that
/// do not need decision counters.
///
/// 执行一次默认 engine 单层转码。
/// [`execute_layer_into_with_stats`] 的 convenience wrapper，适合不需要决策计数的调用方。
pub fn execute_layer_into(
    src_fmt: &FormatSpec,
    tgt_fmt: &FormatSpec,
    src: &[u8],
    decode_ctx: &LayerDecodeContext,
    encode_ctx: &LayerEncodeContext<'_>,
    buffers: LayerExecutionBuffers<'_>,
) -> Result<(), ConvertError> {
    execute_layer_into_with_stats(src_fmt, tgt_fmt, src, decode_ctx, encode_ctx, buffers)
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use crate::{
        formats::{self, goo, LayerDecodeContext, LayerEncodeContext},
        transcode::{execute_layer_into_with_stats, LayerExecutionBuffers},
    };

    fn format_spec(name: &str) -> &'static formats::common::FormatSpec {
        formats::global_registry()
            .spec_by_name(name)
            .unwrap_or_else(|| panic!("{name} spec should be registered"))
    }

    #[test]
    fn execute_layer_into_with_stats_returns_stats_and_restores_buffers() {
        let goo = format_spec("goo");
        let ctb = format_spec("ctb");
        let decode_ctx = LayerDecodeContext::new(0, 8, 1);
        let encode_ctx = LayerEncodeContext {
            layer_index: 0,
            width: 8,
            height: 1,
            layer_metadata: None,
        };
        let mut pixel_buf = vec![0xAA; 4];
        let mut aux_buf = vec![0xBB; 3];
        let original_pixel_capacity = pixel_buf.capacity();
        let original_aux_capacity = aux_buf.capacity();
        let mut out = Vec::new();
        let pixels = vec![0, 0, 32, 32, 128, 128, 255, 255];
        let source = goo::encode_layer_data(&pixels, 8, 1).expect("encode GOO");

        let stats = execute_layer_into_with_stats(
            goo,
            ctb,
            &source,
            &decode_ctx,
            &encode_ctx,
            LayerExecutionBuffers {
                pixel_buf: &mut pixel_buf,
                aux_buf: &mut aux_buf,
                out: &mut out,
            },
        )
        .expect("fast path should transcode");

        assert!(stats.fast_path_applied);
        assert!(!stats.fallback_used);
        assert_eq!(stats.strategy_attempts, 1);
        assert_eq!(pixel_buf, vec![0xAA; 4]);
        assert!(pixel_buf.capacity() >= original_pixel_capacity);
        assert!(aux_buf.capacity() >= original_aux_capacity);
        assert!(!out.is_empty());
    }

    #[test]
    fn execute_layer_into_with_stats_restores_buffers_after_error() {
        let ctb = format_spec("ctb");
        let goo = format_spec("goo");
        let decode_ctx = LayerDecodeContext::new(0, 8, 1);
        let encode_ctx = LayerEncodeContext {
            layer_index: 0,
            width: 8,
            height: 1,
            layer_metadata: None,
        };
        let mut pixel_buf = vec![0xAA; 4];
        let mut aux_buf = vec![0xBB; 3];
        let original_pixel_capacity = pixel_buf.capacity();
        let original_aux_capacity = aux_buf.capacity();
        let mut out = Vec::new();

        let result = execute_layer_into_with_stats(
            ctb,
            goo,
            &[0x80],
            &decode_ctx,
            &encode_ctx,
            LayerExecutionBuffers {
                pixel_buf: &mut pixel_buf,
                aux_buf: &mut aux_buf,
                out: &mut out,
            },
        );

        assert!(result.is_err());
        assert!(pixel_buf.capacity() >= original_pixel_capacity);
        assert!(aux_buf.capacity() >= original_aux_capacity);

        pixel_buf.extend_from_slice(&[0x11, 0x22]);
        aux_buf.extend_from_slice(&[0x33, 0x44]);
        assert!(pixel_buf.ends_with(&[0x11, 0x22]));
        assert!(aux_buf.ends_with(&[0x33, 0x44]));
    }
}
