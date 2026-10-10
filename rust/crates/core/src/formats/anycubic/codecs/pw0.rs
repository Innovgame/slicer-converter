use crate::common::{ConvertError, PreviewLayerImage};
use crate::formats::anycubic::codec::AnycubicLayerCodec;

use super::pw0_rle;

/// Anycubic PW0 layer codec adapter.
///
/// PW0 keeps 4-bit grayscale information and is used by `.pw0` binary profiles.
/// The adapter wires the raw RLE functions into the shared
/// [`AnycubicLayerCodec`] trait.
///
/// Anycubic PW0 layer codec adapter。PW0 保留 4-bit grayscale 信息，用于 `.pw0`
/// binary profiles；该 adapter 将底层 RLE 函数接入共享 [`AnycubicLayerCodec`] trait。
pub struct Pw0Codec;

impl AnycubicLayerCodec for Pw0Codec {
    fn codec_name(&self) -> &'static str {
        "pw0"
    }

    fn decode_layer(&self, data: &[u8]) -> Result<Vec<u8>, ConvertError> {
        pw0_rle::decode_pw0_rle4(data)
    }

    fn decode_layer_into(&self, data: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
        pw0_rle::decode_pw0_rle4_into(data, output)
    }

    fn decode_layer_preview_rgba(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        max_preview_size: u32,
    ) -> Result<PreviewLayerImage, ConvertError> {
        if width <= max_preview_size && height <= max_preview_size {
            let rgba = pw0_rle::decode_pw0_rle4_rgba(data)?;
            return Ok(PreviewLayerImage {
                pixels: rgba,
                width,
                height,
            });
        }

        let (pixels, dst_width, dst_height) =
            pw0_rle::decode_pw0_rle4_preview_rgba(data, width, height, max_preview_size)?;
        Ok(PreviewLayerImage {
            pixels,
            width: dst_width,
            height: dst_height,
        })
    }

    fn encode_layer(&self, pixels: &[u8]) -> Result<Vec<u8>, ConvertError> {
        Ok(pw0_rle::encode_pw0_rle4(pixels))
    }

    fn encode_layer_into(&self, pixels: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
        pw0_rle::encode_pw0_rle4_into(pixels, output);
        Ok(())
    }

    fn layer_image_format(&self) -> &'static str {
        "pw0img"
    }
}
