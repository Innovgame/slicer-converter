use crate::{
    common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage},
    formats::{
        common::{
            binary::{put_le_u32, read_le_u32},
            CryptoPolicy, ExportTargetOptions, FormatSpec, HeaderLayout, LayerCodec,
        },
        ctb, LayerDecodeContext, LayerEncodeContext,
    },
};

pub struct ChituboxHeaderLayout {
    pub allowed_magic: &'static [u32],
    pub allowed_versions: &'static [u32],
    pub build_magic: Option<u32>,
    pub build_version: Option<u32>,
    pub layer_table_entry_size: u32,
}

pub struct CtbRleLayerCodec;

pub struct CbddlpBitplaneLayerCodec;

pub struct ChituboxXorCryptoPolicy;

impl ChituboxHeaderLayout {
    fn read_magic(data: &[u8]) -> Option<u32> {
        read_le_u32(data, 0).ok()
    }

    fn read_version(data: &[u8]) -> Option<u32> {
        read_le_u32(data, 4).ok()
    }

    fn expected_magic(&self) -> u32 {
        self.allowed_magic.first().copied().unwrap_or(0)
    }

    fn build_magic(&self) -> Option<u32> {
        self.build_magic
            .or_else(|| self.allowed_magic.first().copied())
    }

    fn has_allowed_magic(&self, data: &[u8]) -> bool {
        Self::read_magic(data).is_some_and(|magic| self.allowed_magic.contains(&magic))
    }

    fn has_allowed_version(&self, data: &[u8]) -> bool {
        self.allowed_versions.is_empty()
            || Self::read_version(data)
                .is_some_and(|version| self.allowed_versions.contains(&version))
    }

    fn try_detect_profile_header(&self, data: &[u8]) -> bool {
        self.has_allowed_magic(data) && self.has_allowed_version(data)
    }

    fn apply_profile_header_identity(
        &self,
        header: &mut [u8],
        _spec: &FormatSpec,
    ) -> Result<(), ConvertError> {
        if self.build_magic().is_none() && self.build_version.is_none() {
            return Ok(());
        }
        if header.len() < 8 {
            return Err(ConvertError::BufferTooSmall {
                need: 8,
                got: header.len(),
            });
        }

        if let Some(magic) = self.build_magic() {
            put_le_u32(header, 0, magic)?;
        }
        if let Some(version) = self.build_version {
            put_le_u32(header, 4, version)?;
        }
        Ok(())
    }
}

impl HeaderLayout for ChituboxHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        self.try_detect_profile_header(data)
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        if !self.allowed_magic.is_empty() && !self.has_allowed_magic(data) {
            if let Some(actual) = Self::read_magic(data) {
                return Err(ConvertError::InvalidMagic {
                    expected: self.expected_magic(),
                    actual,
                });
            }
        }
        if !self.allowed_versions.is_empty() && !self.has_allowed_version(data) {
            if let Some(version) = Self::read_version(data) {
                return Err(ConvertError::UnsupportedVersion(version));
            }
        }
        let mut decoder = ctb::CtbDecoder::new();
        decoder.parse_header(data)
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        let mut header = ctb::encoder::build_ctb_header_with_profile(
            metadata,
            self.layer_table_entry_size,
            self.build_version.unwrap_or(2),
            spec.identity.name.starts_with("gktwo-ctb"),
        )?;
        self.apply_profile_header_identity(&mut header, spec)?;
        if self.build_magic() == Some(ctb::MAGIC_CBDDLP) {
            put_le_u32(&mut header, 0x5C, metadata.slicing.anti_alias_level.max(1))?;
        }
        Ok(header)
    }

    fn finalize_header(
        &self,
        header: &[u8],
        layer_sizes: &[u32],
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        ctb::finalize_header_bytes(header, layer_sizes)
    }
}

impl LayerCodec for CtbRleLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        ctb::rle::decode_ctb_rle_into(data, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        _ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        ctb::rle::encode_ctb_rle_into(pixels, output)
    }

    fn decode_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        if ctx.width <= max_preview_size && ctx.height <= max_preview_size {
            return Ok(None);
        }
        ctb::rle::decode_ctb_rle_preview_rgba(data, ctx.width, ctx.height, max_preview_size).map(
            |(pixels, width, height)| {
                Some(PreviewLayerImage {
                    pixels,
                    width,
                    height,
                })
            },
        )
    }
}

impl LayerCodec for CbddlpBitplaneLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        ctb::rle::decode_cbddlp_rle_into(
            data,
            ctx.width,
            ctx.height,
            ctx.options.anti_alias_level,
            output,
        )
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        let _ = ctx.layer_index;
        let _ = (ctx.width, ctx.height);
        ctb::rle::encode_cbddlp_rle_into(pixels, output);
        Ok(())
    }

    fn decode_preview_rgba(
        &self,
        _data: &[u8],
        _ctx: &LayerDecodeContext,
        _max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        // CBDDLP bitplane RLE differs from CTB grayscale RLE; use the generic
        // decode-layer fallback so preview pixels follow the same codec path.
        Ok(None)
    }
}

impl CryptoPolicy for ChituboxXorCryptoPolicy {
    fn decrypt_layer_in_place(
        &self,
        data: &mut [u8],
        layer_index: u32,
        ctx: &LayerDecodeContext,
    ) -> Result<(), ConvertError> {
        ctb::crypto::crypt_layer_data(data, ctx.options.encryption_key, layer_index);
        Ok(())
    }

    fn encrypt_layer_in_place(
        &self,
        data: &mut [u8],
        layer_index: u32,
        ctx: &LayerEncodeContext<'_>,
    ) -> Result<(), ConvertError> {
        let seed = ctx
            .layer_metadata
            .map(|metadata| metadata.slicing.encryption_key)
            .unwrap_or(0);
        ctb::crypto::crypt_layer_data(data, seed, layer_index);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::downsample_to_rgba_nearest,
        formats::{
            chitubox::adapters::CbddlpBitplaneLayerCodec,
            common::{LayerCodec, LayerDecodeContext},
            ctb,
        },
    };

    #[test]
    fn cbddlp_preview_falls_back_to_cbddlp_rle_decode_for_single_plane_layers() {
        let width = 128;
        let height = 96;
        let max_preview_size = 48;
        let mut pixels = vec![0u8; (width * height) as usize];
        for y in 0..height as usize {
            for x in 0..width as usize {
                if (x / 8 + y / 8) % 2 == 0 {
                    pixels[y * width as usize + x] = 255;
                }
            }
        }

        let encoded = ctb::rle::encode_cbddlp_rle(&pixels);
        let ctx = LayerDecodeContext::new(0, width, height);
        let direct_preview = CbddlpBitplaneLayerCodec
            .decode_preview_rgba(&encoded, &ctx, max_preview_size)
            .expect("CBDDLP preview");
        assert!(direct_preview.is_none());

        let mut fallback_pixels = Vec::new();
        CbddlpBitplaneLayerCodec
            .decode_layer_into(&encoded, &ctx, &mut fallback_pixels)
            .expect("CBDDLP fallback decode");
        let decoded =
            ctb::rle::decode_cbddlp_rle(&encoded, width, height, 1).expect("CBDDLP decode");
        let expected = downsample_to_rgba_nearest(&decoded, width, height, max_preview_size);
        let fallback_preview =
            downsample_to_rgba_nearest(&fallback_pixels, width, height, max_preview_size);

        assert_eq!(fallback_preview.width, expected.width);
        assert_eq!(fallback_preview.height, expected.height);
        assert_eq!(fallback_preview.pixels, expected.pixels);
    }
}
