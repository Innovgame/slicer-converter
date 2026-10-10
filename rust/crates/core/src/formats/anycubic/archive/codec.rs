use crate::{
    common::{ConvertError, PreviewLayerImage},
    formats::{LayerDecodeContext, LayerEncodeContext},
};

use super::pwsz::decode_pwsz_contours_into;
use crate::formats::anycubic::codecs::pw0_rle;
use crate::formats::common::LayerCodec;

/// Layer codec used by Anycubic ZIP/PWSZ archive profiles.
///
/// ZIP archives normally store `pw0Img` RLE payloads. Some entries use the PWSZ
/// contour payload marker (`{==\0`); those are decoded through the contour path
/// before falling back to PW0 RLE4.
///
/// Anycubic ZIP/PWSZ archive profiles 使用的 layer codec。ZIP archive 通常存储
/// `pw0Img` RLE payload；部分 entry 使用 PWSZ contour payload marker (`{==\0`)，会先走
/// contour decode path，否则回退到 PW0 RLE4。
pub struct AnycubicZipLayerCodec;

impl LayerCodec for AnycubicZipLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        if data.starts_with(b"{==\0") {
            return decode_pwsz_contours_into(data, ctx, output);
        }

        pw0_rle::decode_pw0_rle4_into(data, output)?;
        let expected = (ctx.width as usize).saturating_mul(ctx.height as usize);
        if expected > 0 && output.len() != expected {
            return Err(ConvertError::RleDecodeError {
                offset: output.len(),
                detail: format!(
                    "decoded pw0Img length {} does not match expected {expected}",
                    output.len()
                ),
            });
        }
        Ok(())
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        _ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        pw0_rle::encode_pw0_rle4_into(pixels, output);
        Ok(())
    }

    fn decode_preview_rgba(
        &self,
        _data: &[u8],
        _ctx: &LayerDecodeContext,
        _max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        Ok(None)
    }
}
