use crate::{
    common::ConvertError,
    formats::goo,
    transcode::{FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder},
};

/// Fast-path strategy for CTB gray7 RLE layers targeting GOO.
///
/// The strategy only applies to unencrypted, non-anti-aliased CTB layers because
/// those payloads can be rewritten into GOO runs without materializing a full
/// grayscale image.
///
/// CTB gray7 RLE layer 转 GOO 的快路径策略。该策略仅适用于未加密、无抗锯齿的 CTB 层，
/// 因为这些 payload 可以不展开完整灰度图而直接改写为 GOO runs。
pub(crate) struct CtbToGooTranscoder;

/// Process-wide CTB to GOO strategy instance used by direct tests and registry entries.
///
/// 供直接测试与 registry entry 使用的进程级 CTB to GOO 策略实例。
pub(crate) static CTB_TO_GOO_TRANSCODER: CtbToGooTranscoder = CtbToGooTranscoder;

/// Exported strategy slice for tests that inspect this route module directly.
///
/// 供直接检查此 route module 的测试使用的策略列表。
pub static TRANSCODERS: [&'static dyn LayerTranscoder; 1] = [&CTB_TO_GOO_TRANSCODER];

impl LayerTranscoder for CtbToGooTranscoder {
    fn try_transcode_layer_into(
        &self,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        _scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<FastPathOutcome, ConvertError> {
        if req.decode_options.anti_alias_level != 1 || req.decode_options.encryption_key != 0 {
            return Ok(FastPathOutcome::Fallback);
        }

        if let Some(metadata) = req.encode_ctx.layer_metadata {
            let rle_start =
                goo::encoder::begin_goo_layer_with_metadata_into(metadata, req.layer_index, out)?;
            if !transcode_ctb_rle_to_goo_rle(src, req, out, rle_start) {
                out.clear();
                return Ok(FastPathOutcome::Fallback);
            }
            goo::encoder::finish_goo_layer_with_metadata_in_place(out, rle_start)?;
        } else {
            out.clear();
            if !transcode_ctb_rle_to_goo_rle(src, req, out, 0) {
                out.clear();
                return Ok(FastPathOutcome::Fallback);
            }
        }

        Ok(FastPathOutcome::Applied)
    }
}

fn transcode_ctb_rle_to_goo_rle(
    src: &[u8],
    req: &LayerTranscodeRequest<'_>,
    out: &mut Vec<u8>,
    rle_start: usize,
) -> bool {
    let pixel_count = (req.width as usize).saturating_mul(req.height as usize);
    out.push(goo::LAYER_MAGIC);

    let mut i = 0usize;
    let mut src_pos = 0usize;
    while i < src.len() && src_pos < pixel_count {
        let byte0 = src[i];
        i += 1;

        let is_run = (byte0 & 0x80) != 0;
        let gray7 = byte0 & 0x7F;
        let gray8 = (gray7 << 1) | (gray7 >> 6);
        let Some(stride) = read_ctb_stride(src, &mut i, is_run) else {
            return false;
        };

        let count = stride.min(pixel_count - src_pos);
        push_goo_run(out, gray8, count);
        src_pos += count;
    }

    if src_pos < pixel_count {
        push_goo_run(out, 0, pixel_count - src_pos);
    }

    append_goo_checksum_from(out, rle_start);
    true
}

fn read_ctb_stride(data: &[u8], index: &mut usize, is_run: bool) -> Option<usize> {
    if !is_run {
        return Some(1);
    }

    if *index >= data.len() {
        return None;
    }
    let b1 = data[*index];
    *index += 1;

    if b1 < 0x80 {
        Some(b1 as usize)
    } else if b1 < 0xC0 {
        if *index >= data.len() {
            return None;
        }
        let b2 = data[*index];
        *index += 1;
        Some((((b1 & 0x3F) as usize) << 8) | (b2 as usize))
    } else if b1 < 0xE0 {
        if *index + 1 >= data.len() {
            return None;
        }
        let b2 = data[*index];
        let b3 = data[*index + 1];
        *index += 2;
        Some((((b1 & 0x1F) as usize) << 16) | ((b2 as usize) << 8) | (b3 as usize))
    } else {
        if *index + 2 >= data.len() {
            return None;
        }
        let b2 = data[*index];
        let b3 = data[*index + 1];
        let b4 = data[*index + 2];
        *index += 3;
        Some(
            (((b1 & 0x0F) as usize) << 24)
                | ((b2 as usize) << 16)
                | ((b3 as usize) << 8)
                | (b4 as usize),
        )
    }
}

fn push_goo_run(out: &mut Vec<u8>, value: u8, mut stride: usize) {
    while stride > 0 {
        let chunk = stride.min(0x0FFF_FFFF);
        if value == 0 {
            let byte0 = encode_goo_solid_header(0b00, chunk);
            out.push(byte0);
            encode_goo_stride_extra(out, chunk);
        } else if value == 0xFF {
            let byte0 = encode_goo_solid_header(0b11, chunk);
            out.push(byte0);
            encode_goo_stride_extra(out, chunk);
        } else {
            let byte0 = encode_goo_solid_header(0b01, chunk);
            out.push(byte0);
            out.push(value);
            encode_goo_stride_extra(out, chunk);
        }
        stride -= chunk;
    }
}

fn encode_goo_solid_header(chunk_type: u8, stride: usize) -> u8 {
    (chunk_type << 6) | encode_goo_stride_header(stride)
}

fn encode_goo_stride_header(stride: usize) -> u8 {
    let low4 = (stride & 0x0F) as u8;
    if stride <= 0x0F {
        low4
    } else if stride <= 0x0FFF {
        (0b01 << 4) | low4
    } else if stride <= 0x0F_FFFF {
        (0b10 << 4) | low4
    } else {
        (0b11 << 4) | low4
    }
}

fn encode_goo_stride_extra(output: &mut Vec<u8>, stride: usize) {
    if stride <= 0x0F {
    } else if stride <= 0x0FFF {
        output.push((stride >> 4) as u8);
    } else if stride <= 0x0F_FFFF {
        output.push((stride >> 12) as u8);
        output.push((stride >> 4) as u8);
    } else {
        output.push((stride >> 20) as u8);
        output.push((stride >> 12) as u8);
        output.push((stride >> 4) as u8);
    }
}

fn append_goo_checksum_from(output: &mut Vec<u8>, rle_start: usize) {
    let sum: u8 = output[rle_start + 1..]
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b));
    output.push(!sum);
}

#[cfg(test)]
mod tests {
    use super::CtbToGooTranscoder;
    use crate::{
        common::FileMetadata,
        formats::{goo, LayerEncodeContext, LayerOptions},
        transcode::{
            FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder,
        },
    };

    const GOO_LAYER_DEF_SIZE: usize = 70;

    fn request<'a>(
        width: u32,
        height: u32,
        options: &'a LayerOptions,
    ) -> LayerTranscodeRequest<'a> {
        LayerTranscodeRequest {
            layer_index: 0,
            width,
            height,
            display_width_mm: 0.0,
            display_height_mm: 0.0,
            decode_options: options,
            encode_ctx: LayerEncodeContext {
                layer_index: 0,
                width,
                height,
                layer_metadata: None,
            },
        }
    }

    #[test]
    fn ctb_to_goo_fast_path_applies_and_matches_ctb_fallback_decode_for_raw_output() {
        let pixels = vec![0, 0, 0, 64, 64, 128, 255, 255];
        let ctb_layer =
            crate::formats::ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
        let baseline = crate::formats::ctb::decode_layer_data(&ctb_layer, 0, 8, 1, 1, 0)
            .expect("baseline CTB decode");
        let options = LayerOptions::default();
        let req = request(8, 1, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let outcome = CtbToGooTranscoder
            .try_transcode_layer_into(&ctb_layer, &req, &mut scratch, &mut out)
            .expect("fast path should not error");

        let decoded = goo::decode_layer_data(&out, 8, 1).expect("decode GOO");
        assert_eq!(outcome, FastPathOutcome::Applied);
        assert_eq!(decoded, baseline);
    }

    #[test]
    fn ctb_to_goo_fast_path_wraps_layer_when_metadata_is_present() {
        let pixels = vec![0, 32, 64, 96];
        let ctb_layer =
            crate::formats::ctb::encode_layer_data(&pixels, 0, 4, 1).expect("encode CTB");
        let baseline = crate::formats::ctb::decode_layer_data(&ctb_layer, 0, 4, 1, 1, 0)
            .expect("baseline CTB decode");
        let metadata = FileMetadata::builder()
            .resolution(4, 1)
            .display_size_mm(1.0, 1.0)
            .layer_height_mm(0.05)
            .layer_count(1)
            .build()
            .expect("metadata");
        let layer_metadata = metadata.as_layer_encode_metadata();
        let options = LayerOptions::default();
        let req = LayerTranscodeRequest {
            layer_index: 0,
            width: 4,
            height: 1,
            display_width_mm: 1.0,
            display_height_mm: 1.0,
            decode_options: &options,
            encode_ctx: LayerEncodeContext {
                layer_index: 0,
                width: 4,
                height: 1,
                layer_metadata: Some(layer_metadata),
            },
        };
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let outcome = CtbToGooTranscoder
            .try_transcode_layer_into(&ctb_layer, &req, &mut scratch, &mut out)
            .expect("fast path should not error");

        let wrapped_rle = &out[GOO_LAYER_DEF_SIZE..out.len() - 2];
        let decoded = goo::decode_layer_data(wrapped_rle, 4, 1).expect("decode wrapped RLE");
        let checksum_sum = wrapped_rle[1..]
            .iter()
            .fold(0u8, |acc, &b| acc.wrapping_add(b));
        assert_eq!(outcome, FastPathOutcome::Applied);
        assert_eq!(decoded, baseline);
        assert_eq!(checksum_sum, 0xFF);
    }

    #[test]
    fn ctb_to_goo_fast_path_falls_back_on_truncated_stride() {
        let options = LayerOptions::default();
        let req = request(2, 1, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let outcome = CtbToGooTranscoder
            .try_transcode_layer_into(&[0x80], &req, &mut scratch, &mut out)
            .expect("malformed input should request fallback");

        assert_eq!(outcome, FastPathOutcome::Fallback);
    }
}
