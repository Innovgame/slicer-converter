use crate::common::{ConvertError, FileMetadata, LayerOffset};
use crate::formats::anycubic::codec::AnycubicLayerCodec;
use crate::formats::anycubic::decoder::parse_anycubic_v516_header_data;
use crate::formats::anycubic::encoder::{build_v516_header_bytes, finalize_anycubic_header_bytes};
use crate::formats::anycubic::layout::AnycubicLayoutStrategy;
use crate::formats::anycubic::{AnycubicSpec, AnycubicVersion};

/// Anycubic v5.16 binary layout strategy.
///
/// V5.16 introduces version-specific file mark/header table structs, the
/// machine table, and the motion `EXTRA` table while keeping primary layer
/// definitions compatible with the shared finalizer.
///
/// Anycubic v5.16 binary layout strategy。V5.16 引入版本专属 file mark/header table、
/// machine table 与 motion `EXTRA` table，同时 primary layer definitions 仍可复用共享
/// finalizer。
pub struct V516Layout;

impl AnycubicLayoutStrategy for V516Layout {
    fn version(&self) -> AnycubicVersion {
        AnycubicVersion::V516
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        specifics: &AnycubicSpec,
        _codec: &dyn AnycubicLayerCodec,
    ) -> Result<Vec<u8>, ConvertError> {
        build_v516_header_bytes(metadata, specifics)
    }

    fn finalize_header(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError> {
        finalize_anycubic_header_bytes(header, layer_sizes)
    }

    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_anycubic_v516_header_data(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::FileMetadataBuilder;
    use crate::formats::anycubic::codecs::Pw0Codec;

    fn pick_specifics(version: AnycubicVersion) -> &'static AnycubicSpec {
        crate::formats::anycubic::specs::ANYCUBIC_PROFILES
            .iter()
            .copied()
            .map(|profile| profile.facts)
            .find(|specifics| specifics.supports_version(version))
            .expect("specifics for version")
    }

    #[test]
    fn v516_header_build_parse_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(64, 48)
            .display_size_mm(3.2, 2.4)
            .layer_height_mm(0.05)
            .layer_count(3)
            .bottom_layer_count(1)
            .anti_alias_level(1)
            .exposure(2.5, 25.0)
            .lift(8.0, 60.0)
            .bottom_lift(8.0, 60.0)
            .retract(150.0)
            .statistics(600, 5.5, 5.0, 0.25)
            .build()
            .unwrap();

        let payloads: Vec<Vec<u8>> = (0..3)
            .map(|i| vec![0xB0 + i as u8; 32 + i as usize * 8])
            .collect();
        let sizes: Vec<u32> = payloads.iter().map(|p| p.len() as u32).collect();

        let specifics = pick_specifics(AnycubicVersion::V516);
        let hdr = V516Layout
            .build_header(&metadata, specifics, &Pw0Codec)
            .unwrap();
        let finalized = V516Layout.finalize_header(hdr, &sizes).unwrap();

        let mut full = finalized.clone();
        for p in &payloads {
            full.extend_from_slice(p);
        }

        let (parsed, offsets) = V516Layout.parse_header(&full).expect("parse_header");
        assert_eq!(parsed.printer.resolution_x, metadata.printer.resolution_x);
        assert_eq!(parsed.printer.resolution_y, metadata.printer.resolution_y);
        assert_eq!(parsed.slicing.layer_count, metadata.slicing.layer_count);
        assert_eq!(
            parsed.slicing.bottom_layer_count,
            metadata.slicing.bottom_layer_count
        );
        assert_eq!(
            parsed.slicing.layer_height_mm,
            metadata.slicing.layer_height_mm
        );
        assert_eq!(offsets.len(), payloads.len());

        for (off, p) in offsets.iter().zip(payloads.iter()) {
            let start = off.offset as usize;
            let end = start + off.size as usize;
            assert_eq!(&full[start..end], p.as_slice());
        }
    }
}
