use crate::{
    common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage},
    formats::{
        common::{ExportTargetOptions, FooterPolicy, FormatSpec, HeaderLayout, LayerCodec},
        goo, LayerDecodeContext, LayerEncodeContext,
    },
};

/// Header layout adapter shared by GOO and PRZ profiles.
///
/// It delegates to the GOO-family parser/encoder and leaves header finalization
/// unchanged because GOO-family layer offsets are built directly into the header
/// from metadata.
///
/// GOO 与 PRZ profile 共享的 header layout adapter。它委托给 GOO-family parser/encoder；
/// header finalization 保持原样，因为 GOO-family layer offsets 已由 metadata 构建进 header。
pub struct GooFamilyHeaderLayout;

/// Layer codec adapter shared by GOO-family profiles.
///
/// `use_color_difference_compression` distinguishes GOO from PRZ while keeping
/// the rest of the RLE payload contract shared.
///
/// GOO-family profiles 共享的 layer codec adapter。`use_color_difference_compression`
/// 用于区分 GOO 与 PRZ，其余 RLE payload 契约保持共享。
pub struct GooFamilyLayerCodec {
    /// Whether encoder should use GOO color-difference RLE chunks.
    ///
    /// encoder 是否使用 GOO color-difference RLE chunks。
    pub use_color_difference_compression: bool,
}

/// Footer policy for GOO-family binary containers.
///
/// GOO-family binary container 的 footer policy。
pub struct GooFamilyFooterPolicy;

impl HeaderLayout for GooFamilyHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        goo::looks_like_goo_family(data)
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        goo::parse_header_data(data)
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        goo::build_header_bytes(metadata)
    }

    fn finalize_header(
        &self,
        header: &[u8],
        _layer_sizes: &[u32],
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(header.to_vec())
    }
}

impl LayerCodec for GooFamilyLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        goo::rle::decode_goo_rle_into(data, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        goo::encode_goo_layer_into(pixels, ctx, self.use_color_difference_compression, output)
    }

    fn decode_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        goo::decode_layer_preview_rgba(data, ctx.width, ctx.height, max_preview_size).map(Some)
    }
}

impl FooterPolicy for GooFamilyFooterPolicy {
    fn build_footer(
        &self,
        _metadata: &FileMetadata,
        _spec: &FormatSpec,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(goo::build_footer_bytes())
    }
}
