use std::io::Cursor;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use binrw::BinRead;

use super::header::{
    GooHeader, GooLayerDef, BIG_PREVIEW_H, BIG_PREVIEW_W, SMALL_PREVIEW_H, SMALL_PREVIEW_W,
};
use crate::common::{
    ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset, ParseResult, Thumbnail,
};

const LAYER_DEF_SIZE: usize = 70;
const GOO_HEADER_SIZE_FALLBACK: usize = 195_477;

type GooLayerDataOffsets = Vec<(u64, u64)>;
type ParsedGooLayers = (Vec<GooLayerDef>, GooLayerDataOffsets);

fn warn_nonstandard_layer_delimiter(layer_index: usize, delimiter: [u8; 2]) {
    emit_decoder_warning(format!(
        "warning: non-standard GOO layer delimiter at layer {layer_index}: [{:02X}, {:02X}]; continuing with data_length-based alignment",
        delimiter[0], delimiter[1]
    ));
}

#[cfg(not(test))]
fn emit_decoder_warning(message: String) {
    eprintln!("{message}");
}

#[cfg(test)]
fn emit_decoder_warning(message: String) {
    test_warnings()
        .lock()
        .expect("warning buffer lock poisoned")
        .push(message);
}

#[cfg(test)]
fn test_warnings() -> &'static Mutex<Vec<String>> {
    static WARNINGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    WARNINGS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
fn drain_test_warnings() -> Vec<String> {
    let mut warnings = test_warnings()
        .lock()
        .expect("warning buffer lock poisoned");
    std::mem::take(&mut *warnings)
}

/// GOO format decoder — reads header + layer offsets from a full GOO file.
pub struct GooDecoder {
    pub header: GooHeader,
    pub layer_defs: Vec<GooLayerDef>,
    /// Byte offsets where each layer's RLE data starts in the source file.
    pub layer_data_offsets: GooLayerDataOffsets,
}

impl GooDecoder {
    /// Parse GOO header + layer definitions from the complete file data.
    pub fn parse_header(data: &[u8]) -> Result<Self, ConvertError> {
        let header = GooHeader::from_bytes(data)?;

        let layer_count = header.layer_count as usize;
        let header_start = header.layer_def_address as usize;

        let mut starts = vec![header_start];
        if header_start != GOO_HEADER_SIZE_FALLBACK {
            starts.push(GOO_HEADER_SIZE_FALLBACK);
        }

        let mut last_error = None;
        for start in starts {
            match Self::parse_layers_from(data, layer_count, start) {
                Ok((layer_defs, layer_data_offsets)) => {
                    return Ok(Self {
                        header,
                        layer_defs,
                        layer_data_offsets,
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(ConvertError::BufferTooSmall {
            need: header_start.saturating_add(LAYER_DEF_SIZE),
            got: data.len(),
        }))
    }

    fn parse_layers_from(
        data: &[u8],
        layer_count: usize,
        start: usize,
    ) -> Result<ParsedGooLayers, ConvertError> {
        let mut layer_defs = Vec::with_capacity(layer_count);
        let mut layer_data_offsets = Vec::with_capacity(layer_count);
        let mut warned_nonstandard_layer_delimiter = false;
        let mut pos = start;

        for layer_index in 0..layer_count {
            let layer_def_end =
                pos.checked_add(LAYER_DEF_SIZE)
                    .ok_or(ConvertError::BufferTooSmall {
                        need: usize::MAX,
                        got: data.len(),
                    })?;

            if layer_def_end > data.len() {
                return Err(ConvertError::BufferTooSmall {
                    need: layer_def_end,
                    got: data.len(),
                });
            }

            let layer_def_data = &data[pos..layer_def_end];
            let mut cursor = Cursor::new(layer_def_data);
            let layer_def = GooLayerDef::read(&mut cursor)
                .map_err(|e| ConvertError::BinrwError(e.to_string()))?;

            if !warned_nonstandard_layer_delimiter && layer_def.delimiter != super::DELIMITER {
                warn_nonstandard_layer_delimiter(layer_index, layer_def.delimiter);
                warned_nonstandard_layer_delimiter = true;
            }

            let data_start = layer_def_end;
            let data_len = layer_def.data_length as usize;
            let data_end =
                data_start
                    .checked_add(data_len)
                    .ok_or(ConvertError::BufferTooSmall {
                        need: usize::MAX,
                        got: data.len(),
                    })?;

            if data_end > data.len() {
                return Err(ConvertError::BufferTooSmall {
                    need: data_end,
                    got: data.len(),
                });
            }

            layer_data_offsets.push((data_start as u64, data_len as u64));
            layer_defs.push(layer_def);
            // UVTools reads the 2-byte delimiter field but does not hard-fail if
            // it is non-standard; keep stream alignment driven by data_length and
            // skip the trailing 2-byte delimiter after the RLE payload.
            pos = data_end + 2;
        }

        Ok((layer_defs, layer_data_offsets))
    }

    /// Convert parsed header into the common ParseResult.
    pub fn to_parse_result(&self) -> ParseResult {
        let metadata = self.to_metadata();
        let layers = self
            .layer_data_offsets
            .iter()
            .map(|&(offset, size)| LayerOffset { offset, size })
            .collect();

        ParseResult {
            format: "goo".into(),
            metadata,
            layers,
        }
    }

    /// Convert GOO header fields into common FileMetadata.
    fn to_metadata(&self) -> FileMetadata {
        let h = &self.header;
        let name = String::from_utf8_lossy(&h.machine_name)
            .trim_end_matches('\0')
            .to_string();

        // Extract thumbnails from raw RGB565 preview data
        let mut thumbnails = Vec::new();
        if let Ok(rgba) =
            super::preview::decode_rgb565_be(&h.small_preview_565, SMALL_PREVIEW_W, SMALL_PREVIEW_H)
        {
            thumbnails.push(Thumbnail {
                width: SMALL_PREVIEW_W,
                height: SMALL_PREVIEW_H,
                pixels_rgba: rgba,
            });
        }
        if let Ok(rgba) =
            super::preview::decode_rgb565_be(&h.big_preview_565, BIG_PREVIEW_W, BIG_PREVIEW_H)
        {
            thumbnails.push(Thumbnail {
                width: BIG_PREVIEW_W,
                height: BIG_PREVIEW_H,
                pixels_rgba: rgba,
            });
        }

        FileMetadataBuilder::new()
            .resolution(h.resolution_x as u32, h.resolution_y as u32)
            .display_size_mm(h.display_width_mm, h.display_height_mm)
            .machine_z_mm(h.machine_z_mm)
            .machine_name(name)
            .mirror(h.mirror_x, h.mirror_y)
            .layer_height_mm(h.layer_height_mm)
            .layer_count(h.layer_count)
            .bottom_layer_count(h.bottom_layer_count)
            .transition_layer_count(h.transition_layer_count as u32)
            .anti_alias_level(u32::from(h.anti_aliasing_level.max(1)))
            .exposure(h.exposure_time, h.bottom_exposure_time)
            .light_off_delay(h.light_off_delay, h.light_off_delay)
            .light_pwm(h.light_pwm, h.bottom_light_pwm)
            .lift(h.lift_height, h.lift_speed)
            .lift2(h.lift_height2, h.lift_speed2)
            .bottom_lift(h.bottom_lift_height, h.bottom_lift_speed)
            .retract(h.retract_speed)
            .retract2(h.retract_height2, h.retract_speed2)
            .wait_times(
                h.wait_time_before_cure,
                h.wait_time_after_cure,
                h.wait_time_after_lift,
            )
            .statistics(
                h.print_time_sec,
                h.volume_mm3 / 1000.0,
                h.material_grams,
                h.material_cost,
            )
            .thumbnails(thumbnails)
            .build_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::{drain_test_warnings, GooDecoder, GOO_HEADER_SIZE_FALLBACK, LAYER_DEF_SIZE};
    use crate::common::FileMetadataBuilder;
    use crate::formats::common::binary::put_be_u32;
    use crate::formats::goo::encoder::{
        build_goo_header, build_goo_layer, build_goo_layer_with_metadata,
    };
    use crate::formats::goo::header::{GooHeader, GooLayerDef};
    use binrw::BinRead;
    use std::io::Cursor;

    #[test]
    fn export_sets_first_stage_retract_from_lift_height() {
        let metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .layer_height_mm(0.05)
            .layer_count(2)
            .bottom_layer_count(1)
            .lift(6.0, 65.0)
            .lift2(1.5, 40.0)
            .bottom_lift(8.0, 45.0)
            .retract(150.0)
            .retract2(2.0, 80.0)
            .build()
            .expect("motion metadata");
        let header = GooHeader::from_bytes(&build_goo_header(&metadata).expect("header"))
            .expect("parse header");
        assert_eq!(header.retract_height, 5.5);
        assert_eq!(header.bottom_lift_height2, 1.5);
        assert_eq!(header.bottom_lift_speed2, 40.0);
        assert_eq!(header.bottom_retract_height, 7.5);

        let layer =
            build_goo_layer_with_metadata(metadata.as_layer_encode_metadata(), 1, &[0x55, 0x00])
                .expect("normal layer");
        let layer_def = GooLayerDef::read(&mut Cursor::new(&layer)).expect("parse layer def");
        assert_eq!(layer_def.retract_height, 5.5);
        assert_eq!(layer_def.lift_height, 6.0);

        let bottom_layer =
            build_goo_layer_with_metadata(metadata.as_layer_encode_metadata(), 0, &[0x55, 0x00])
                .expect("bottom layer");
        let bottom_def =
            GooLayerDef::read(&mut Cursor::new(&bottom_layer)).expect("parse bottom layer def");
        assert_eq!(bottom_def.lift_height, 8.0);
        assert_eq!(bottom_def.lift_height2, 1.5);
        assert_eq!(bottom_def.retract_height, header.bottom_retract_height);
    }

    #[test]
    fn export_selects_wait_time_mode_when_light_off_delay_is_zero() {
        let wait_metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .layer_height_mm(0.05)
            .layer_count(1)
            .light_off_delay(0.0, 0.0)
            .wait_times(0.4, 0.2, 0.1)
            .build()
            .expect("wait metadata");
        let wait_header =
            GooHeader::from_bytes(&build_goo_header(&wait_metadata).expect("wait header"))
                .expect("parse wait header");
        assert_eq!(wait_header.delay_mode, 1);
        assert_eq!(wait_header.bottom_wait_time_after_cure, 0.2);

        let light_off_metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .layer_height_mm(0.05)
            .layer_count(1)
            .light_off_delay(0.3, 0.8)
            .wait_times(0.4, 0.0, 0.0)
            .build()
            .expect("light-off metadata");
        let light_off_header = GooHeader::from_bytes(
            &build_goo_header(&light_off_metadata).expect("light-off header"),
        )
        .expect("parse light-off header");
        assert_eq!(light_off_header.delay_mode, 0);
    }

    #[test]
    fn parse_header_keeps_anti_alias_level() {
        let metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .machine_name("Elegoo Mars")
            .layer_height_mm(0.05)
            .layer_count(1)
            .anti_alias_level(4)
            .build()
            .expect("valid fixture");
        let header = build_goo_header(&metadata).expect("build header");
        let layer = build_goo_layer(
            0,
            metadata.slicing.layer_height_mm,
            metadata.exposure.exposure_time,
            &[0x55, 0x00],
        )
        .expect("build layer");
        let mut file = header;
        file.extend_from_slice(&layer);

        let parsed = GooDecoder::parse_header(&file).expect("parse GOO");
        assert_eq!(parsed.header.anti_aliasing_level, 4);
        assert_eq!(
            parsed.to_parse_result().metadata.slicing.anti_alias_level,
            4
        );
    }

    #[test]
    fn parse_header_falls_back_when_layer_address_is_invalid() {
        let metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .machine_name("Elegoo Mars")
            .layer_height_mm(0.05)
            .layer_count(1)
            .build()
            .expect("valid fixture");

        let mut header = build_goo_header(&metadata).expect("build header");
        let layer = build_goo_layer(
            0,
            metadata.slicing.layer_height_mm,
            metadata.exposure.exposure_time,
            &[0x55, 0x00],
        )
        .expect("build layer");

        // Corrupt layer_def_address to emulate malformed third-party files.
        let lda_offset = header.len() - 7;
        put_be_u32(&mut header, lda_offset, 0x551F_5236).expect("patch layer address");

        let mut file = header;
        file.extend_from_slice(&layer);

        let parsed = GooDecoder::parse_header(&file).expect("parse with fallback");
        assert_eq!(parsed.layer_defs.len(), 1);
        assert_eq!(
            parsed.layer_data_offsets[0].0 as usize,
            GOO_HEADER_SIZE_FALLBACK + LAYER_DEF_SIZE
        );
        assert_eq!(parsed.layer_data_offsets[0].1, 2);
    }

    #[test]
    fn parse_header_accepts_nonstandard_layer_def_delimiter() {
        let _ = drain_test_warnings();
        let metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .machine_name("Elegoo Mars")
            .layer_height_mm(0.05)
            .layer_count(1)
            .build()
            .expect("valid fixture");

        let header = build_goo_header(&metadata).expect("build header");
        let mut layer = build_goo_layer(
            0,
            metadata.slicing.layer_height_mm,
            metadata.exposure.exposure_time,
            &[0x55, 0x00],
        )
        .expect("build layer");

        // UVTools tolerates non-standard bytes in the layer-def delimiter field
        // as long as the layer size and trailing delimiter keep the stream aligned.
        layer[64..66].copy_from_slice(&[0x00, 0x00]);

        let mut file = header;
        file.extend_from_slice(&layer);

        let parsed = GooDecoder::parse_header(&file).expect("parse with relaxed delimiter");
        assert_eq!(parsed.layer_defs.len(), 1);
        assert_eq!(parsed.layer_defs[0].delimiter, [0x00, 0x00]);
        assert_eq!(parsed.layer_data_offsets[0].1, 2);

        let warnings = drain_test_warnings();
        assert_eq!(
            warnings.len(),
            1,
            "expected exactly one compatibility warning"
        );
        assert!(
            warnings[0].contains("non-standard GOO layer delimiter"),
            "unexpected warning payload: {}",
            warnings[0]
        );
    }
}
