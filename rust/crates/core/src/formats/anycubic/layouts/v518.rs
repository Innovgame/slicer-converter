use crate::common::{ConvertError, FileMetadata, LayerOffset};
use crate::formats::anycubic::codec::AnycubicLayerCodec;
use crate::formats::anycubic::decoder::parse_anycubic_v518_header_data;
use crate::formats::anycubic::encoder::{build_v518_header_bytes, finalize_anycubic_header_bytes};
use crate::formats::anycubic::layout::AnycubicLayoutStrategy;
use crate::formats::anycubic::{AnycubicSpec, AnycubicVersion};

/// Anycubic v5.18 binary layout strategy.
///
/// V5.18 adds sub-layer definitions and a second preview block. The current
/// layer payload finalizer still patches the primary `LAYERDEF` table.
///
/// Anycubic v5.18 binary layout strategy。V5.18 增加 sub-layer definitions 与 second
/// preview block；当前 layer payload finalizer 仍 patch primary `LAYERDEF` table。
pub struct V518Layout;

impl AnycubicLayoutStrategy for V518Layout {
    fn version(&self) -> AnycubicVersion {
        AnycubicVersion::V518
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        specifics: &AnycubicSpec,
        _codec: &dyn AnycubicLayerCodec,
    ) -> Result<Vec<u8>, ConvertError> {
        build_v518_header_bytes(metadata, specifics)
    }

    fn finalize_header(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError> {
        finalize_anycubic_header_bytes(header, layer_sizes)
    }

    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_anycubic_v518_header_data(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::FileMetadataBuilder;
    use crate::formats::anycubic::codecs::Pw0Codec;

    #[test]
    fn v518_header_build_parse_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(64, 48)
            .display_size_mm(3.2, 2.4)
            .layer_height_mm(0.04)
            .layer_count(2)
            .bottom_layer_count(1)
            .anti_alias_level(1)
            .exposure(3.5, 35.0)
            .lift(6.0, 55.0)
            .bottom_lift(6.0, 55.0)
            .retract(130.0)
            .statistics(450, 3.8, 3.5, 0.18)
            .build()
            .unwrap();

        let payloads: Vec<Vec<u8>> = vec![vec![0xD1; 48], vec![0xD2; 56]];
        let sizes: Vec<u32> = payloads.iter().map(|p| p.len() as u32).collect();

        let specifics = crate::formats::anycubic::specs::ANYCUBIC_PROFILES
            .iter()
            .copied()
            .map(|profile| profile.facts)
            .find(|specifics| specifics.supports_version(AnycubicVersion::V518))
            .unwrap();
        let hdr = V518Layout
            .build_header(&metadata, specifics, &Pw0Codec)
            .unwrap();
        let finalized = V518Layout.finalize_header(hdr, &sizes).unwrap();
        let mut full = finalized.clone();
        for p in &payloads {
            full.extend_from_slice(p);
        }

        let (parsed, offsets) = V518Layout.parse_header(&full).expect("parse_header");
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
