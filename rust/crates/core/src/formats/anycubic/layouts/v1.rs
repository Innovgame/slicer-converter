use crate::common::{ConvertError, FileMetadata, LayerOffset};
use crate::formats::anycubic::codec::AnycubicLayerCodec;
use crate::formats::anycubic::decoder::parse_anycubic_v1_header_data;
use crate::formats::anycubic::encoder::{build_v1_header_bytes, finalize_anycubic_header_bytes};
use crate::formats::anycubic::layout::AnycubicLayoutStrategy;
use crate::formats::anycubic::{AnycubicSpec, AnycubicVersion};

/// Anycubic legacy v1 binary layout strategy.
///
/// V1 uses the original file mark, header table, fixed preview, and 32-byte
/// layer definitions. It is still used by early PW0/PWS-family profiles.
///
/// Anycubic legacy v1 binary layout strategy。V1 使用原始 file mark、header table、
/// 固定 preview 与 32-byte layer definitions，仍用于早期 PW0/PWS-family profiles。
pub struct V1Layout;

impl AnycubicLayoutStrategy for V1Layout {
    fn version(&self) -> AnycubicVersion {
        AnycubicVersion::V1
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _specifics: &AnycubicSpec,
        _codec: &dyn AnycubicLayerCodec,
    ) -> Result<Vec<u8>, ConvertError> {
        build_v1_header_bytes(metadata)
    }

    fn finalize_header(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError> {
        finalize_anycubic_header_bytes(header, layer_sizes)
    }

    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_anycubic_v1_header_data(data)
    }
}
