use crate::common::{ConvertError, FileMetadata, LayerOffset};
use crate::formats::anycubic::codec::AnycubicLayerCodec;
use crate::formats::anycubic::decoder::parse_anycubic_v517_header_data;
use crate::formats::anycubic::encoder::{build_v517_header_bytes, finalize_anycubic_header_bytes};
use crate::formats::anycubic::layout::AnycubicLayoutStrategy;
use crate::formats::anycubic::{AnycubicSpec, AnycubicVersion};

/// Anycubic v5.17 binary layout strategy.
///
/// V5.17 extends v5.16 with software/model tables and a larger file mark, but
/// still uses the shared 32-byte primary layer definition table.
///
/// Anycubic v5.17 binary layout strategy。V5.17 在 v5.16 基础上增加 software/model tables
/// 与更大的 file mark，但仍使用共享的 32-byte primary layer definition table。
pub struct V517Layout;

impl AnycubicLayoutStrategy for V517Layout {
    fn version(&self) -> AnycubicVersion {
        AnycubicVersion::V517
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        specifics: &AnycubicSpec,
        _codec: &dyn AnycubicLayerCodec,
    ) -> Result<Vec<u8>, ConvertError> {
        build_v517_header_bytes(metadata, specifics)
    }

    fn finalize_header(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError> {
        finalize_anycubic_header_bytes(header, layer_sizes)
    }

    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_anycubic_v517_header_data(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::FileMetadataBuilder;
    use crate::formats::anycubic::codecs::Pw0Codec;

    #[test]
    fn v517_header_build_parse_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(64, 48)
            .display_size_mm(3.2, 2.4)
            .layer_height_mm(0.05)
            .layer_count(2)
            .bottom_layer_count(1)
            .anti_alias_level(1)
            .exposure(3.0, 30.0)
            .lift(7.0, 50.0)
            .bottom_lift(7.0, 50.0)
            .retract(140.0)
            .statistics(500, 4.5, 4.0, 0.20)
            .build()
            .unwrap();

        let payloads: Vec<Vec<u8>> = vec![vec![0xC1; 40], vec![0xC2; 48]];
        let sizes: Vec<u32> = payloads.iter().map(|p| p.len() as u32).collect();

        let specifics = crate::formats::anycubic::specs::ANYCUBIC_PROFILES
            .iter()
            .copied()
            .map(|profile| profile.facts)
            .find(|specifics| specifics.supports_version(AnycubicVersion::V517))
            .unwrap();
        let hdr = V517Layout
            .build_header(&metadata, specifics, &Pw0Codec)
            .unwrap();
        let finalized = V517Layout.finalize_header(hdr, &sizes).unwrap();
        let mut full = finalized.clone();
        for p in &payloads {
            full.extend_from_slice(p);
        }

        let (parsed, offsets) = V517Layout.parse_header(&full).expect("parse_header");
        assert_eq!(parsed.printer.resolution_x, metadata.printer.resolution_x);
        assert_eq!(parsed.slicing.layer_count, metadata.slicing.layer_count);
        assert_eq!(offsets.len(), payloads.len());
        for (off, p) in offsets.iter().zip(payloads.iter()) {
            let start = off.offset as usize;
            let end = start + off.size as usize;
            assert_eq!(&full[start..end], p.as_slice());
        }
    }
}
