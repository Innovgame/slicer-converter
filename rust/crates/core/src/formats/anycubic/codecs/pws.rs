use crate::common::{ConvertError, PreviewLayerImage};
use crate::formats::anycubic::codec::AnycubicLayerCodec;

use super::pws_rle;

/// Anycubic PWS layer codec adapter.
///
/// PWS stores binary black/white RLE and is used by `.pws` profiles. The adapter
/// exposes full-layer and preview decode paths through [`AnycubicLayerCodec`].
///
/// Anycubic PWS layer codec adapter。PWS 存储 binary black/white RLE，用于 `.pws`
/// profiles；该 adapter 通过 [`AnycubicLayerCodec`] 暴露整层与 preview decode paths。
pub struct PwsCodec;

impl AnycubicLayerCodec for PwsCodec {
    fn codec_name(&self) -> &'static str {
        "pws"
    }

    fn decode_layer(&self, data: &[u8]) -> Result<Vec<u8>, ConvertError> {
        pws_rle::decode_pws_rle1(data)
    }

    fn decode_layer_into(&self, data: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
        pws_rle::decode_pws_rle1_into(data, output)
    }

    fn decode_layer_preview_rgba(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        max_preview_size: u32,
    ) -> Result<PreviewLayerImage, ConvertError> {
        if width <= max_preview_size && height <= max_preview_size {
            let rgba = pws_rle::decode_pws_rle1_rgba(data)?;
            return Ok(PreviewLayerImage {
                pixels: rgba,
                width,
                height,
            });
        }

        let (pixels, dst_width, dst_height) =
            pws_rle::decode_pws_rle1_preview_rgba(data, width, height, max_preview_size)?;
        Ok(PreviewLayerImage {
            pixels,
            width: dst_width,
            height: dst_height,
        })
    }

    fn encode_layer(&self, pixels: &[u8]) -> Result<Vec<u8>, ConvertError> {
        Ok(pws_rle::encode_pws_rle1(pixels))
    }

    fn encode_layer_into(&self, pixels: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
        pws_rle::encode_pws_rle1_into(pixels, output);
        Ok(())
    }

    fn layer_image_format(&self) -> &'static str {
        "pwsimg"
    }
}
