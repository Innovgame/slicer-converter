use crate::{
    common::ConvertError,
    formats::{common::FormatSpec, LayerDecodeContext},
};

use super::{LayerTranscodeRequest, LayerTranscodeScratch};

/// Generic decode-to-grayscale then encode fallback.
///
/// This path is used when no registered fast path can handle the route. It is
/// deliberately simple and authoritative: source profile decodes into
/// `scratch.pixel_buf`, target profile encodes from that grayscale buffer.
///
/// 通用 fallback：先解码为灰度，再编码为目标格式。当没有已注册快路径能处理该 route 时使用。
/// 它刻意保持简单且权威：源 profile 解码到 `scratch.pixel_buf`，目标 profile 再从该灰度
/// buffer 编码。
pub fn transcode_layer_with_formats(
    src_fmt: &FormatSpec,
    tgt_fmt: &FormatSpec,
    src: &[u8],
    req: &LayerTranscodeRequest<'_>,
    scratch: &mut LayerTranscodeScratch,
    out: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    scratch.pixel_buf.clear();
    let decode_ctx = LayerDecodeContext::new(req.layer_index, req.width, req.height)
        .with_display_size_mm(req.display_width_mm, req.display_height_mm)
        .with_options(req.decode_options.clone());
    src_fmt.decode_layer_into(src, &decode_ctx, &mut scratch.pixel_buf)?;
    tgt_fmt.encode_layer_into(&scratch.pixel_buf, &req.encode_ctx, out)
}
