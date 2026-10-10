// V515Layout — Anycubic protocol version 515.
//
// Why is there no `anycubic_format!(V515_FORMAT, ...)` entry in `mod.rs`?
//
// V515 is *not* tied to a unique file extension. It is an older write-side
// protocol version of the same extension family that V516+ uses (pwmo, pwmx,
// pmsq, pwma, pm3, etc.). The decode side does not need a separate format
// entry either: V515 files share FileMark + HeaderTable + LayerDef layout
// with V1, so `parse_anycubic_v1_header_data` reads V515 byte streams
// correctly (LayerImageColorTable is the only structural addition and is
// reached via explicit address fields).
//
// V515 is therefore a "supported but never default" version. It is reachable:
//   * On the encode side, only when a layout test or future typed spec selects
//     `V515Layout` directly. No concrete exported spec declares V515 as its
//     `default_version`; it appears only in `supported_versions` lists for
//     older firmware.
//   * On the decode side, transparently via the V1 parsing path.
//
// As a consequence the format-registry tests (`sample.pwma` / `sample.pm5`
// / `sample.pm5s`) cover V516/V517/V518 round-trips but not V515 directly;
// V515 is exercised by the in-file `v515_header_build_parse_roundtrip`
// unit test below and by the V1 sample tests on the decode side.

use crate::common::{ConvertError, FileMetadata, LayerOffset};
use crate::formats::anycubic::codec::AnycubicLayerCodec;
use crate::formats::anycubic::decoder::parse_anycubic_v1_header_data;
use crate::formats::anycubic::encoder::{build_v515_header_bytes, finalize_anycubic_header_bytes};
use crate::formats::anycubic::layout::AnycubicLayoutStrategy;
use crate::formats::anycubic::{AnycubicSpec, AnycubicVersion};

/// Anycubic v5.15 binary layout strategy.
///
/// V5.15 adds a layer image color table but otherwise parses through the v1
/// header path because the file mark and layer definition shapes remain
/// compatible.
///
/// Anycubic v5.15 binary layout strategy。V5.15 增加 layer image color table，但
/// file mark 与 layer definition shape 仍兼容 v1，因此 parse 复用 v1 header path。
pub struct V515Layout;

impl AnycubicLayoutStrategy for V515Layout {
    fn version(&self) -> AnycubicVersion {
        AnycubicVersion::V515
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _specifics: &AnycubicSpec,
        _codec: &dyn AnycubicLayerCodec,
    ) -> Result<Vec<u8>, ConvertError> {
        build_v515_header_bytes(metadata)
    }

    fn finalize_header(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError> {
        finalize_anycubic_header_bytes(header, layer_sizes)
    }

    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        // V515 shares FileMark + HeaderTable + LayerDef layout with V1; the only
        // structural addition is the LayerImageColorTable (pointed to by
        // layer_image_color_table_address in FileMark). Since parsing follows the
        // address fields explicitly, the V1 little-endian path reads V515 files
        // correctly without needing a separate implementation.
        parse_anycubic_v1_header_data(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::FileMetadataBuilder;

    fn pwmx_specifics() -> &'static AnycubicSpec {
        crate::formats::global_registry()
            .catalog_profile_by_name("pwmx")
            .expect("pwmx profile")
            .as_anycubic()
            .expect("pwmx specifics")
    }

    fn sample_metadata(layer_count: u32) -> FileMetadata {
        FileMetadataBuilder::new()
            .resolution(64, 48)
            .display_size_mm(3.2, 2.4)
            .layer_height_mm(0.05)
            .layer_count(layer_count)
            .bottom_layer_count(1)
            .transition_layer_count(0)
            .anti_alias_level(1)
            .exposure(2.5, 25.0)
            .light_off_delay(0.5, 0.5)
            .lift(8.0, 60.0)
            .bottom_lift(8.0, 60.0)
            .retract(150.0)
            .wait_times(0.5, 0.0, 0.0)
            .statistics(600, 5.5, 5.0, 0.25)
            .build()
            .expect("valid fixture")
    }

    #[test]
    fn v515_header_build_parse_roundtrip() {
        let metadata = sample_metadata(3);
        let layer_payloads: Vec<Vec<u8>> = (0..3)
            .map(|i| vec![0xA0u8 + i as u8; 32 + (i as usize) * 8])
            .collect();
        let layer_sizes: Vec<u32> = layer_payloads.iter().map(|p| p.len() as u32).collect();

        let header_skeleton = V515Layout
            .build_header(
                &metadata,
                pwmx_specifics(),
                &crate::formats::anycubic::codecs::Pw0Codec,
            )
            .expect("build_header");
        let finalized = V515Layout
            .finalize_header(header_skeleton, &layer_sizes)
            .expect("finalize_header");

        let mut full_file = finalized.clone();
        for payload in &layer_payloads {
            full_file.extend_from_slice(payload);
        }

        let (parsed_meta, offsets) = V515Layout.parse_header(&full_file).expect("parse_header");

        assert_eq!(
            parsed_meta.printer.resolution_x,
            metadata.printer.resolution_x
        );
        assert_eq!(
            parsed_meta.printer.resolution_y,
            metadata.printer.resolution_y
        );
        assert_eq!(
            parsed_meta.slicing.layer_count,
            metadata.slicing.layer_count
        );
        assert_eq!(
            parsed_meta.slicing.bottom_layer_count,
            metadata.slicing.bottom_layer_count
        );
        assert_eq!(
            parsed_meta.slicing.layer_height_mm,
            metadata.slicing.layer_height_mm
        );
        assert!(
            (parsed_meta.printer.display_width_mm - metadata.printer.display_width_mm).abs() < 1e-3
        );
        assert!(
            (parsed_meta.printer.display_height_mm - metadata.printer.display_height_mm).abs()
                < 1e-3
        );
        assert_eq!(offsets.len(), layer_payloads.len());

        for (offset, payload) in offsets.iter().zip(layer_payloads.iter()) {
            let start = offset.offset as usize;
            let end = start + offset.size as usize;
            assert_eq!(
                &full_file[start..end],
                payload.as_slice(),
                "layer payload mismatch at offset {}",
                offset.offset
            );
        }
    }
}
