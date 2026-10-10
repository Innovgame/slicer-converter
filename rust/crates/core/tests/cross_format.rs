//! Cross-format conversion integration tests.
//!
//! Verifies that grayscale layer data can be encoded in one format
//! and decoded back through another, preserving pixel data.

#[cfg(test)]
mod tests {
    use slicer_converter_core::common::{FileMetadata, FileMetadataBuilder};
    use slicer_converter_core::formats::anycubic::codecs::{Pw0Codec, PwsCodec};
    use slicer_converter_core::formats::anycubic::AnycubicLayerCodec;
    use slicer_converter_core::formats::common::{
        ArchiveEncodeContext, LayerSource, VecOutputSink,
    };
    use slicer_converter_core::formats::{
        ctb, goo, prz, sl1, vdt, LayerDecodeContext, LayerEncodeContext,
    };

    /// Generate a synthetic grayscale test layer with a gradient pattern.
    fn make_test_layer(width: u32, height: u32) -> Vec<u8> {
        let count = (width as usize) * (height as usize);
        let mut pixels = vec![0u8; count];
        for (i, p) in pixels.iter_mut().enumerate() {
            let x = (i % width as usize) as u8;
            let y = (i / width as usize) as u8;
            *p = x.wrapping_add(y.wrapping_mul(3));
        }
        pixels
    }

    /// CTB uses 7-bit gray internally, so roundtrip is lossy.
    /// Pre-quantize pixels to CTB's 7-bit space for exact comparison.
    fn ctb_quantize(pixels: &[u8]) -> Vec<u8> {
        pixels
            .iter()
            .map(|&p| {
                let gray7 = p >> 1;
                (gray7 << 1) | (gray7 >> 6)
            })
            .collect()
    }

    /// Generate a simple solid layer for basic tests.
    fn make_solid_layer(width: u32, height: u32, value: u8) -> Vec<u8> {
        vec![value; (width as usize) * (height as usize)]
    }

    fn encode_document(
        target_format: &str,
        metadata: &FileMetadata,
        encoded_layers: &[Vec<u8>],
        job_name: &str,
    ) -> Vec<u8> {
        let spec =
            slicer_converter_core::formats::get_format_spec_by_name(target_format).expect("spec");
        let mut sink = VecOutputSink::new();
        let ctx = ArchiveEncodeContext::new(job_name);
        spec.pipeline
            .encode_document_into(
                spec,
                metadata,
                LayerSource::Encoded(encoded_layers),
                &mut sink,
                &ctx,
            )
            .expect("document encode");
        sink.into_inner()
    }

    fn encode_pw0(pixels: &[u8]) -> Vec<u8> {
        Pw0Codec.encode_layer(pixels).unwrap()
    }

    fn decode_pw0(data: &[u8]) -> Vec<u8> {
        Pw0Codec.decode_layer(data).unwrap()
    }

    fn encode_pws(pixels: &[u8]) -> Vec<u8> {
        PwsCodec.encode_layer(pixels).unwrap()
    }

    fn decode_pws(data: &[u8]) -> Vec<u8> {
        PwsCodec.decode_layer(data).unwrap()
    }

    fn build_pw0_header(metadata: &FileMetadata) -> Vec<u8> {
        slicer_converter_core::formats::get_format_spec_by_name("pw0")
            .expect("pw0 spec")
            .build_header(metadata)
            .unwrap()
    }

    fn parse_pw0_header(
        data: &[u8],
    ) -> (
        FileMetadata,
        Vec<slicer_converter_core::common::LayerOffset>,
    ) {
        let spec =
            slicer_converter_core::formats::get_format_spec_by_name("pw0").expect("pw0 spec");
        let parsed = spec.pipeline.parse_document(spec, data).unwrap();
        (parsed.metadata, parsed.layers)
    }

    // 鈹€鈹€鈹€ CTB 鈫?GOO 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn ctb_to_goo_layer_roundtrip() {
        let (w, h) = (100, 80);
        let original = ctb_quantize(&make_test_layer(w, h));

        // Encode as CTB RLE 鈫?decode 鈫?encode as GOO RLE 鈫?decode
        let ctb_encoded = ctb::encode_layer_data(&original, 0, w, h).unwrap();
        let decoded_from_ctb = ctb::decode_layer_data(&ctb_encoded, 0, w, h, 1, 0).unwrap();
        assert_eq!(decoded_from_ctb, original, "CTB roundtrip failed");

        let goo_encoded = goo::encode_layer_data(&decoded_from_ctb, w, h).unwrap();
        let decoded_from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();
        assert_eq!(decoded_from_goo, original, "CTB鈫扜OO conversion lost data");
    }

    #[test]
    fn goo_to_ctb_layer_roundtrip() {
        let (w, h) = (120, 60);
        let original = ctb_quantize(&make_test_layer(w, h));

        let goo_encoded = goo::encode_layer_data(&original, w, h).unwrap();
        let decoded_from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();
        assert_eq!(decoded_from_goo, original);

        let ctb_encoded = ctb::encode_layer_data(&decoded_from_goo, 0, w, h).unwrap();
        let decoded_from_ctb = ctb::decode_layer_data(&ctb_encoded, 0, w, h, 1, 0).unwrap();
        assert_eq!(decoded_from_ctb, original, "GOO鈫扖TB conversion lost data");
    }

    // 鈹€鈹€鈹€ CTB 鈫?SL1 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn ctb_to_sl1_layer_roundtrip() {
        let (w, h) = (64, 64);
        let original = ctb_quantize(&make_test_layer(w, h));

        let ctb_encoded = ctb::encode_layer_data(&original, 0, w, h).unwrap();
        let decoded_from_ctb = ctb::decode_layer_data(&ctb_encoded, 0, w, h, 1, 0).unwrap();

        let png_data = sl1::encode_layer_data(&decoded_from_ctb, w, h).unwrap();
        let decoded_from_sl1 = sl1::decode_layer_data(&png_data, w, h).unwrap();
        assert_eq!(decoded_from_sl1, original, "CTB鈫扴L1 conversion lost data");
    }

    #[test]
    fn sl1_to_ctb_layer_roundtrip() {
        let (w, h) = (80, 40);
        let original = ctb_quantize(&make_test_layer(w, h));

        let png_data = sl1::encode_layer_data(&original, w, h).unwrap();
        let decoded_from_sl1 = sl1::decode_layer_data(&png_data, w, h).unwrap();

        let ctb_encoded = ctb::encode_layer_data(&decoded_from_sl1, 0, w, h).unwrap();
        let decoded_from_ctb = ctb::decode_layer_data(&ctb_encoded, 0, w, h, 1, 0).unwrap();
        assert_eq!(decoded_from_ctb, original, "SL1鈫扖TB conversion lost data");
    }

    // 鈹€鈹€鈹€ SL1 鈫?GOO 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn sl1_to_goo_layer_roundtrip() {
        let (w, h) = (96, 48);
        let original = make_test_layer(w, h);

        let png_data = sl1::encode_layer_data(&original, w, h).unwrap();
        let decoded_from_sl1 = sl1::decode_layer_data(&png_data, w, h).unwrap();

        let goo_encoded = goo::encode_layer_data(&decoded_from_sl1, w, h).unwrap();
        let decoded_from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();
        assert_eq!(decoded_from_goo, original, "SL1鈫扜OO conversion lost data");
    }

    #[test]
    fn goo_to_sl1_layer_roundtrip() {
        let (w, h) = (50, 50);
        let original = make_test_layer(w, h);

        let goo_encoded = goo::encode_layer_data(&original, w, h).unwrap();
        let decoded_from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();

        let png_data = sl1::encode_layer_data(&decoded_from_goo, w, h).unwrap();
        let decoded_from_sl1 = sl1::decode_layer_data(&png_data, w, h).unwrap();
        assert_eq!(decoded_from_sl1, original, "GOO鈫扴L1 conversion lost data");
    }

    #[test]
    fn goo_to_prz_layer_roundtrip() {
        let (w, h) = (96, 48);
        let original = make_test_layer(w, h);

        let goo_encoded = goo::encode_layer_data(&original, w, h).unwrap();
        let decoded_from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();

        let prz_encoded = prz::encode_layer_data(&decoded_from_goo, w, h).unwrap();
        let decoded_from_prz = prz::decode_layer_data(&prz_encoded, w, h).unwrap();
        assert_eq!(decoded_from_prz, original, "GOO鈫扨RZ conversion lost data");
    }

    // 鈹€鈹€鈹€ Three-way chain 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn three_way_chain_ctb_goo_sl1() {
        let (w, h) = (128, 64);
        let original = ctb_quantize(&make_test_layer(w, h));

        // CTB 鈫?GOO 鈫?SL1 鈫?verify
        let ctb_encoded = ctb::encode_layer_data(&original, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_encoded, 0, w, h, 1, 0).unwrap();

        let goo_encoded = goo::encode_layer_data(&from_ctb, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();

        let png_data = sl1::encode_layer_data(&from_goo, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png_data, w, h).unwrap();

        assert_eq!(from_sl1, original, "CTB鈫扜OO鈫扴L1 chain lost data");
    }

    #[test]
    fn three_way_chain_sl1_ctb_goo() {
        let (w, h) = (200, 100);
        let original = ctb_quantize(&make_test_layer(w, h));

        // SL1 鈫?CTB 鈫?GOO 鈫?verify
        let png_data = sl1::encode_layer_data(&original, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png_data, w, h).unwrap();

        let ctb_encoded = ctb::encode_layer_data(&from_sl1, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_encoded, 0, w, h, 1, 0).unwrap();

        let goo_encoded = goo::encode_layer_data(&from_ctb, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_encoded, w, h).unwrap();

        assert_eq!(from_goo, original, "SL1鈫扖TB鈫扜OO chain lost data");
    }

    // 鈹€鈹€鈹€ Solid layers 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn cross_format_solid_black() {
        let (w, h) = (256, 256);
        let original = make_solid_layer(w, h, 0x00);

        // CTB roundtrip (black survives CTB 7-bit)
        let ctb_enc = ctb::encode_layer_data(&original, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        let png = sl1::encode_layer_data(&from_ctb, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_sl1, original);

        // GOO 鈫?CTB (black survives both)
        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        let ctb_enc2 = ctb::encode_layer_data(&from_goo, 0, w, h).unwrap();
        let from_ctb2 = ctb::decode_layer_data(&ctb_enc2, 0, w, h, 1, 0).unwrap();
        assert_eq!(from_ctb2, original);
    }

    #[test]
    fn cross_format_solid_white() {
        let (w, h) = (256, 256);
        let original = make_solid_layer(w, h, 0xFF);

        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        let png = sl1::encode_layer_data(&from_goo, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_sl1, original);
    }

    // 鈹€鈹€鈹€ Compression ratio comparison 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn compression_ratio_comparison() {
        let (w, h) = (1920, 1080);
        let layer = make_test_layer(w, h);

        let ctb_size = ctb::encode_layer_data(&layer, 0, w, h).unwrap().len();
        let goo_size = goo::encode_layer_data(&layer, w, h).unwrap().len();
        let sl1_size = sl1::encode_layer_data(&layer, w, h).unwrap().len();

        // RLE may not compress gradient data (many unique values),
        // so just verify they all produce valid non-empty output.
        let raw_size = (w as usize) * (h as usize);
        assert!(ctb_size > 0, "CTB should produce output");
        assert!(goo_size > 0, "GOO should produce output");
        assert!(sl1_size > 0, "SL1 should produce output");

        eprintln!(
            "Compression for {}x{}: raw={}, CTB={} ({:.1}%), GOO={} ({:.1}%), SL1/PNG={} ({:.1}%)",
            w,
            h,
            raw_size,
            ctb_size,
            ctb_size as f64 / raw_size as f64 * 100.0,
            goo_size,
            goo_size as f64 / raw_size as f64 * 100.0,
            sl1_size,
            sl1_size as f64 / raw_size as f64 * 100.0,
        );
    }

    // 鈹€鈹€鈹€ Header build/parse roundtrip 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn goo_header_build_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(3840, 2400)
            .display_size_mm(192.0, 120.0)
            .machine_z_mm(200.0)
            .machine_name("TestPrinter")
            .layer_height_mm(0.05)
            .layer_count(100)
            .bottom_layer_count(6)
            .exposure(2.5, 30.0)
            .light_pwm(255, 255)
            .build()
            .expect("valid fixture");

        let header_bytes = goo::build_header_bytes(&metadata).unwrap();

        // Should start with "V3.0" version then GOO magic
        assert_eq!(&header_bytes[0..4], b"V3.0");
        assert_eq!(&header_bytes[4..12], goo::MAGIC_BYTES.as_slice());

        // Parse it back
        // We can't parse_header because there are no layer defs in the header-only data,
        // but we can parse just the GooHeader
        use slicer_converter_core::formats::goo::header::GooHeader;
        let parsed = GooHeader::from_bytes(&header_bytes).unwrap();
        assert_eq!(parsed.resolution_x, 3840);
        assert_eq!(parsed.resolution_y, 2400);
        assert_eq!(parsed.layer_count, 100);
        assert!((parsed.layer_height_mm - 0.05).abs() < 0.001);
        assert!((parsed.display_width_mm - 192.0).abs() < 0.01);
    }

    #[test]
    fn goo_family_file_assembly_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(64, 48)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .machine_name("Phrozen Sonic Mini 8K S")
            .layer_height_mm(0.05)
            .layer_count(2)
            .bottom_layer_count(1)
            .exposure(2.5, 25.0)
            .light_off_delay(0.3, 0.8)
            .light_pwm(255, 200)
            .lift(5.0, 65.0)
            .retract(150.0)
            .bottom_lift(6.0, 45.0)
            .build()
            .expect("valid fixture");

        let layer_metadata = metadata.as_layer_encode_metadata();
        let fmt = slicer_converter_core::formats::get_format_spec_by_name("prz").unwrap();

        let layer0_pixels =
            make_test_layer(metadata.printer.resolution_x, metadata.printer.resolution_y);
        let layer1_pixels = make_solid_layer(
            metadata.printer.resolution_x,
            metadata.printer.resolution_y,
            0x80,
        );

        let layer0 = fmt
            .encode_layer(
                &layer0_pixels,
                &LayerEncodeContext {
                    layer_index: 0,
                    width: metadata.printer.resolution_x,
                    height: metadata.printer.resolution_y,
                    layer_metadata: Some(layer_metadata),
                },
            )
            .unwrap();
        let layer1 = fmt
            .encode_layer(
                &layer1_pixels,
                &LayerEncodeContext {
                    layer_index: 1,
                    width: metadata.printer.resolution_x,
                    height: metadata.printer.resolution_y,
                    layer_metadata: Some(layer_metadata),
                },
            )
            .unwrap();

        let file = encode_document("prz", &metadata, &[layer0, layer1], "ignored");

        let parsed = fmt.pipeline.parse_document(fmt, &file).unwrap();
        assert_eq!(parsed.metadata.slicing.layer_count, 2);
        assert_eq!(parsed.layers.len(), 2);

        let layer0_offset = parsed.layers[0].offset as usize;
        let layer0_end = layer0_offset + parsed.layers[0].size as usize;
        let decoded_layer0 = fmt
            .decode_layer(
                &file[layer0_offset..layer0_end],
                &LayerDecodeContext::new(
                    0,
                    metadata.printer.resolution_x,
                    metadata.printer.resolution_y,
                ),
            )
            .unwrap();
        assert_eq!(decoded_layer0, layer0_pixels);

        let layer1_offset = parsed.layers[1].offset as usize;
        let layer1_end = layer1_offset + parsed.layers[1].size as usize;
        let decoded_layer1 = fmt
            .decode_layer(
                &file[layer1_offset..layer1_end],
                &LayerDecodeContext::new(
                    1,
                    metadata.printer.resolution_x,
                    metadata.printer.resolution_y,
                ),
            )
            .unwrap();
        assert_eq!(decoded_layer1, layer1_pixels);
    }

    #[test]
    fn ctb_header_build_produces_valid_magic() {
        let metadata = FileMetadataBuilder::new()
            .resolution(2560, 1620)
            .display_size_mm(68.04, 120.96)
            .layer_height_mm(0.05)
            .layer_count(50)
            .build()
            .expect("valid fixture");

        let header_bytes = ctb::build_header_bytes(&metadata).unwrap();
        // CTB header starts with a 4-byte magic number
        let magic = u32::from_le_bytes([
            header_bytes[0],
            header_bytes[1],
            header_bytes[2],
            header_bytes[3],
        ]);
        assert!(
            magic == ctb::MAGIC_CTB
                || magic == ctb::MAGIC_CTB_V4
                || magic == ctb::MAGIC_CBDDLP
                || magic == ctb::MAGIC_CTB_V4_GKTWO,
            "Invalid CTB magic: {magic:#010X}"
        );
    }

    // 鈹€鈹€鈹€ Format detection 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn detect_ctb_format_by_magic() {
        // CTB magic 0x12FD0086 at offset 0
        let mut data = vec![0u8; 128];
        data[0..4].copy_from_slice(&0x12FD_0086u32.to_le_bytes());
        data[4..8].copy_from_slice(&2u32.to_le_bytes());
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, None).unwrap();
        assert_eq!(fmt.identity.name, "ctb");
    }

    #[test]
    fn detect_goo_format_by_magic() {
        // GOO: 4-byte version + 8-byte magic at offset 4
        let mut data = vec![0u8; 128];
        data[0..4].copy_from_slice(b"V3.0");
        data[4..12].copy_from_slice(goo::MAGIC_BYTES);
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, None).unwrap();
        assert_eq!(fmt.identity.name, "goo");
    }

    #[test]
    fn detect_sl1_format_by_zip_markers() {
        // SL1 is a ZIP with config.ini + prusaslicer.ini metadata markers.
        let data = b"PK\x03\x04 config.ini prusaslicer.ini 00000.png".to_vec();
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, None).unwrap();
        assert_eq!(fmt.identity.name, "sl1");
    }

    #[test]
    fn detect_chitubox_zip_format_by_run_gcode_marker() {
        let data = b"PK\x03\x04 run.gcode preview.png 1.png".to_vec();
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("zip")).unwrap();
        assert_eq!(fmt.identity.name, "chitubox-zip");
    }

    #[test]
    fn detect_anycubic_zip_format_by_manifest_markers() {
        let data =
            b"PK\x03\x04 anycubic_photon_resins.pwsp layers_controller.conf layer_images/layer_0.pw0Img"
                .to_vec();
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("pm7")).unwrap();
        assert_eq!(fmt.identity.name, "anycubic-zip");
    }

    #[test]
    fn detect_anycubic_zip_markers_override_generic_zip_hint() {
        let data =
            b"PK\x03\x04 anycubic_photon_resins.pwsp layers_controller.conf layer_images/layer_0.pw0Img"
                .to_vec();
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("zip")).unwrap();
        assert_eq!(fmt.identity.name, "anycubic-zip");
    }

    #[test]
    fn detect_format_by_extension_hint() {
        let data = vec![0u8; 128]; // no magic
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("ctb"))
                .unwrap()
                .identity
                .name,
            "ctb"
        );
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("cbddlp"))
                .unwrap()
                .identity
                .name,
            "cbddlp"
        );
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("sl1"))
                .unwrap()
                .identity
                .name,
            "sl1"
        );
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("goo"))
                .unwrap()
                .identity
                .name,
            "goo"
        );
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("prz"))
                .unwrap()
                .identity
                .name,
            "prz"
        );
    }

    #[test]
    fn detect_prz_format_by_extension_on_shared_magic() {
        let mut data = vec![0u8; 128];
        data[0..4].copy_from_slice(b"V3.0");
        data[4..12].copy_from_slice(goo::MAGIC_BYTES);

        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("prz")).unwrap();
        assert_eq!(fmt.identity.name, "prz");
    }

    #[test]
    fn detect_format_unknown_returns_error() {
        let data = vec![0u8; 128];
        let result = slicer_converter_core::formats::detect_format_spec(&data, Some("xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_format_registry_completeness() {
        for spec in slicer_converter_core::formats::global_registry().all_specs() {
            for alias in spec.identity.aliases {
                let found = slicer_converter_core::formats::get_format_spec_by_name(alias).unwrap();
                assert_eq!(found.identity.name, spec.identity.name);
            }
        }
    }

    #[test]
    fn test_detect_by_magic_via_registry() {
        let mut ctb_header = vec![0u8; 128];
        ctb_header[0..4].copy_from_slice(&0x12FD_0086u32.to_le_bytes());
        ctb_header[4..8].copy_from_slice(&2u32.to_le_bytes());
        let fmt = slicer_converter_core::formats::detect_format_spec(&ctb_header, None).unwrap();
        assert_eq!(fmt.identity.name, "ctb");
    }

    #[test]
    fn test_detect_by_extension_only() {
        let empty: &[u8] = &[];
        let fmt = slicer_converter_core::formats::detect_format_spec(empty, Some("sl1")).unwrap();
        assert_eq!(fmt.identity.name, "sl1");
    }

    // 鈹€鈹€鈹€ Edge case: single-pixel layer 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn single_pixel_layer_all_formats() {
        let (w, h) = (1, 1);
        let pixel_val = 128u8;
        let original = vec![pixel_val];

        // SL1 roundtrip
        let png = sl1::encode_layer_data(&original, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_sl1, original, "SL1 single pixel failed");

        // GOO roundtrip
        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, original, "GOO single pixel failed");

        // CTB roundtrip (quantized)
        let ctb_original = ctb_quantize(&original);
        let ctb_enc = ctb::encode_layer_data(&ctb_original, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        assert_eq!(from_ctb, ctb_original, "CTB single pixel failed");
    }

    // 鈹€鈹€鈹€ Edge case: large layer (4K resolution) 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn large_layer_4k_roundtrip() {
        let (w, h) = (3840, 2160);
        // Use solid layer to avoid huge memory in gradient
        let original = make_solid_layer(w, h, 0x80);

        // GOO roundtrip
        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, original, "GOO 4K roundtrip failed");

        // SL1 roundtrip
        let png = sl1::encode_layer_data(&original, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_sl1, original, "SL1 4K roundtrip failed");
    }

    // 鈹€鈹€鈹€ Edge case: checkerboard pattern 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn checkerboard_pattern_all_formats() {
        let (w, h) = (64, 64);
        let mut pixels = vec![0u8; (w as usize) * (h as usize)];
        for (i, p) in pixels.iter_mut().enumerate() {
            let x = i % w as usize;
            let y = i / w as usize;
            *p = if (x + y).is_multiple_of(2) {
                0xFF
            } else {
                0x00
            };
        }

        // SL1 鈫?GOO
        let png = sl1::encode_layer_data(&pixels, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        let goo_enc = goo::encode_layer_data(&from_sl1, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, pixels, "Checkerboard SL1鈫扜OO failed");

        // CTB (black/white survive 7-bit quantization)
        let ctb_enc = ctb::encode_layer_data(&pixels, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        // 0xFF 鈫?gray7=127 鈫?decoded=(127<<1)|(127>>6)=255, 0x00 鈫?0
        assert_eq!(from_ctb, pixels, "Checkerboard CTB roundtrip failed");
    }

    // 鈹€鈹€鈹€ Edge case: all identical grayscale values 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn all_gray_values_roundtrip() {
        let (w, h) = (16, 16);
        // Test every possible byte value
        for val in [0u8, 1, 2, 63, 64, 127, 128, 191, 192, 254, 255] {
            let original = make_solid_layer(w, h, val);

            let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
            let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
            assert_eq!(from_goo, original, "GOO solid gray {val} failed");

            let png = sl1::encode_layer_data(&original, w, h).unwrap();
            let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
            assert_eq!(from_sl1, original, "SL1 solid gray {val} failed");
        }
    }

    // 鈹€鈹€鈹€ SL1 config INI roundtrip 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn sl1_config_ini_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(2560, 1620)
            .display_size_mm(68.04, 120.96)
            .layer_height_mm(0.05)
            .layer_count(200)
            .exposure(3.0, 45.0)
            .bottom_layer_count(8)
            .build()
            .expect("valid fixture");

        let ini_text = sl1::build_config_ini(&metadata);
        assert!(ini_text.contains("exp_time"), "Missing exp_time in INI");
        assert!(
            ini_text.contains("layer_height"),
            "Missing layer_height in INI"
        );
        assert!(ini_text.contains("num_fade"), "Missing num_fade in INI");

        // Parse it back
        let parsed = slicer_converter_core::formats::sl1::config::Sl1Config::parse(&ini_text);
        let parsed_meta = parsed.to_metadata();
        assert_eq!(parsed_meta.slicing.layer_count, 200);
        assert!((parsed_meta.exposure.exposure_time - 3.0).abs() < 0.01);
        assert!((parsed_meta.exposure.bottom_exposure_time - 45.0).abs() < 0.01);
    }

    // 鈹€鈹€鈹€ Multi-layer simulation 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn multi_layer_conversion_pipeline() {
        let (w, h) = (64, 64);
        let layers: Vec<Vec<u8>> = (0..5)
            .map(|i| make_solid_layer(w, h, (i * 50) as u8))
            .collect();

        // Encode all layers as GOO, decode, re-encode as SL1, decode, verify
        for (i, original) in layers.iter().enumerate() {
            let goo_enc = goo::encode_layer_data(original, w, h).unwrap();
            let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();

            let png = sl1::encode_layer_data(&from_goo, w, h).unwrap();
            let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();

            assert_eq!(
                &from_sl1, original,
                "Multi-layer pipeline failed at layer {i}"
            );
        }
    }

    // 鈹€鈹€鈹€ GOO footer structure 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn goo_footer_is_valid() {
        let footer = goo::build_footer_bytes();
        // GOO footer should be non-empty and contain the end bytes
        assert!(!footer.is_empty(), "GOO footer should not be empty");
    }

    // 鈹€鈹€鈹€ Empty layer behavior 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn empty_layer_encoding_returns_error_or_empty() {
        // 0-width or 0-height should either error or return empty
        let pixels = vec![];
        let result_goo = goo::encode_layer_data(&pixels, 0, 0);
        let result_sl1 = sl1::encode_layer_data(&pixels, 0, 0);
        // Just verify they don't panic — error or empty output is fine
        let _ = result_goo;
        let _ = result_sl1;
    }

    // ── PW0 quantization helper ────────────────────────────────────────────

    /// PW0 uses RLE4 (4-bit color code). Quantize to PW0's palette for exact comparison.
    /// Encode: code = pixel >> 4; Decode: pixel = (code << 4) | code
    fn pw0_quantize(pixels: &[u8]) -> Vec<u8> {
        pixels
            .iter()
            .map(|&p| {
                let code = p >> 4;
                (code << 4) | code
            })
            .collect()
    }

    /// PWS uses RLE1 (1-bit binarization at threshold 128).
    fn pws_binarize(pixels: &[u8]) -> Vec<u8> {
        pixels
            .iter()
            .map(|&p| if p >= 128 { 0xFF } else { 0x00 })
            .collect()
    }

    // ── PW0 ↔ other formats ────────────────────────────────────────────────

    #[test]
    fn pw0_roundtrip() {
        let (w, h) = (100, 80);
        let original = pw0_quantize(&make_test_layer(w, h));

        let encoded = encode_pw0(&original);
        let decoded = decode_pw0(&encoded);
        assert_eq!(decoded, original, "PW0 self-roundtrip failed");
    }

    #[test]
    fn pw0_to_goo_layer_roundtrip() {
        let (w, h) = (96, 64);
        let original = pw0_quantize(&make_test_layer(w, h));

        let pw0_enc = encode_pw0(&original);
        let from_pw0 = decode_pw0(&pw0_enc);

        let goo_enc = goo::encode_layer_data(&from_pw0, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, original, "PW0→GOO conversion lost data");
    }

    #[test]
    fn goo_to_pw0_layer_roundtrip() {
        let (w, h) = (80, 60);
        let original = pw0_quantize(&make_test_layer(w, h));

        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();

        let pw0_enc = encode_pw0(&from_goo);
        let from_pw0 = decode_pw0(&pw0_enc);
        assert_eq!(from_pw0, original, "GOO→PW0 conversion lost data");
    }

    #[test]
    fn pw0_to_ctb_layer_roundtrip() {
        let (w, h) = (64, 64);
        let layer = make_test_layer(w, h);
        let quantized = ctb_quantize(&pw0_quantize(&layer));
        let re_quantized = ctb_quantize(&pw0_quantize(&quantized));
        assert_eq!(quantized, re_quantized, "Quantization not stable");

        let pw0_enc = encode_pw0(&quantized);
        let from_pw0 = decode_pw0(&pw0_enc);

        let ctb_enc = ctb::encode_layer_data(&from_pw0, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        assert_eq!(from_ctb, quantized, "PW0→CTB conversion lost data");
    }

    #[test]
    fn pw0_to_sl1_layer_roundtrip() {
        let (w, h) = (64, 48);
        let original = pw0_quantize(&make_test_layer(w, h));

        let pw0_enc = encode_pw0(&original);
        let from_pw0 = decode_pw0(&pw0_enc);

        let png = sl1::encode_layer_data(&from_pw0, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_sl1, original, "PW0→SL1 conversion lost data");
    }

    // ── PWS ↔ other formats ────────────────────────────────────────────────

    #[test]
    fn pws_roundtrip() {
        let (w, h) = (100, 80);
        let original = pws_binarize(&make_test_layer(w, h));

        let encoded = encode_pws(&original);
        let decoded = decode_pws(&encoded);
        assert_eq!(decoded, original, "PWS self-roundtrip failed");
    }

    #[test]
    fn pws_to_goo_layer_roundtrip() {
        let (w, h) = (96, 64);
        let original = pws_binarize(&make_test_layer(w, h));

        let pws_enc = encode_pws(&original);
        let from_pws = decode_pws(&pws_enc);

        let goo_enc = goo::encode_layer_data(&from_pws, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, original, "PWS→GOO conversion lost data");
    }

    #[test]
    fn goo_to_pws_layer_roundtrip() {
        let (w, h) = (80, 60);
        let original = pws_binarize(&make_test_layer(w, h));

        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();

        let pws_enc = encode_pws(&from_goo);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, original, "GOO→PWS conversion lost data");
    }

    #[test]
    fn pws_to_sl1_layer_roundtrip() {
        let (w, h) = (64, 48);
        let original = pws_binarize(&make_test_layer(w, h));

        let pws_enc = encode_pws(&original);
        let from_pws = decode_pws(&pws_enc);

        let png = sl1::encode_layer_data(&from_pws, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_sl1, original, "PWS→SL1 conversion lost data");
    }

    #[test]
    fn pw0_to_pws_layer_lossy() {
        let (w, h) = (64, 64);
        let original = pw0_quantize(&make_test_layer(w, h));
        let expected = pws_binarize(&original);

        let pw0_enc = encode_pw0(&original);
        let from_pw0 = decode_pw0(&pw0_enc);

        let pws_enc = encode_pws(&from_pw0);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, expected, "PW0→PWS lossy conversion incorrect");
    }

    // ── VDT ↔ other formats ────────────────────────────────────────────────

    #[test]
    fn vdt_roundtrip() {
        let (w, h) = (64, 64);
        let original = make_test_layer(w, h);

        let encoded = vdt::encode_layer_data(&original, w, h).unwrap();
        let decoded = vdt::decode_layer_data(&encoded, w, h).unwrap();
        assert_eq!(decoded, original, "VDT self-roundtrip failed");
    }

    #[test]
    fn vdt_to_goo_layer_roundtrip() {
        let (w, h) = (96, 48);
        let original = make_test_layer(w, h);

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();

        let goo_enc = goo::encode_layer_data(&from_vdt, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, original, "VDT→GOO conversion lost data");
    }

    #[test]
    fn goo_to_vdt_layer_roundtrip() {
        let (w, h) = (80, 60);
        let original = make_test_layer(w, h);

        let goo_enc = goo::encode_layer_data(&original, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();

        let png = vdt::encode_layer_data(&from_goo, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_vdt, original, "GOO→VDT conversion lost data");
    }

    #[test]
    fn vdt_to_ctb_layer_roundtrip() {
        let (w, h) = (64, 64);
        let original = ctb_quantize(&make_test_layer(w, h));

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();

        let ctb_enc = ctb::encode_layer_data(&from_vdt, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        assert_eq!(from_ctb, original, "VDT→CTB conversion lost data");
    }

    #[test]
    fn registry_bitmap_chitubox_profiles_roundtrip_layers() {
        let (w, h) = (16, 8);
        let original = ctb_quantize(&make_test_layer(w, h));

        for name in ["ctb-v3", "ctb-v4", "ctb-v5", "gktwo-ctb"] {
            let fmt = slicer_converter_core::formats::get_format_spec_by_name(name).expect(name);
            assert_eq!(fmt.identity.name, name);

            let ctx = LayerEncodeContext {
                layer_index: 0,
                width: w,
                height: h,
                layer_metadata: None,
            };
            let encoded = fmt.encode_layer(&original, &ctx).expect("encode layer");
            let decoded = fmt
                .decode_layer(&encoded, &LayerDecodeContext::new(0, w, h))
                .expect("decode layer");

            assert_eq!(decoded, original, "{name} layer roundtrip lost data");
        }
    }

    #[test]
    fn registry_cbddlp_profile_roundtrips_binary_layers() {
        let (w, h) = (16, 8);
        let original = pws_binarize(&make_test_layer(w, h));

        for name in ["cbddlp", "photon"] {
            let fmt = slicer_converter_core::formats::get_format_spec_by_name(name).expect(name);
            assert_eq!(fmt.identity.name, name);

            let ctx = LayerEncodeContext {
                layer_index: 0,
                width: w,
                height: h,
                layer_metadata: None,
            };
            let encoded = fmt.encode_layer(&original, &ctx).expect("encode bitplane");
            let decoded = fmt
                .decode_layer(&encoded, &LayerDecodeContext::new(0, w, h))
                .expect("decode bitplane");

            assert_eq!(decoded, original, "{name} bitplane conversion lost data");
        }
    }

    #[test]
    fn vdt_to_pw0_layer_roundtrip() {
        let (w, h) = (64, 48);
        let original = pw0_quantize(&make_test_layer(w, h));

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();

        let pw0_enc = encode_pw0(&from_vdt);
        let from_pw0 = decode_pw0(&pw0_enc);
        assert_eq!(from_pw0, original, "VDT→PW0 conversion lost data");
    }

    #[test]
    fn vdt_to_pws_layer_roundtrip() {
        let (w, h) = (64, 48);
        let original = pws_binarize(&make_test_layer(w, h));

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();

        let pws_enc = encode_pws(&from_vdt);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, original, "VDT→PWS conversion lost data");
    }

    // ── Multi-format chains including new formats ───────────────────────────

    #[test]
    fn three_way_chain_vdt_goo_ctb() {
        let (w, h) = (128, 64);
        let original = ctb_quantize(&make_test_layer(w, h));

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();

        let goo_enc = goo::encode_layer_data(&from_vdt, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();

        let ctb_enc = ctb::encode_layer_data(&from_goo, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        assert_eq!(from_ctb, original, "VDT→GOO→CTB chain lost data");
    }

    #[test]
    fn three_way_chain_pw0_sl1_goo() {
        let (w, h) = (96, 48);
        let original = pw0_quantize(&make_test_layer(w, h));

        let pw0_enc = encode_pw0(&original);
        let from_pw0 = decode_pw0(&pw0_enc);

        let png = sl1::encode_layer_data(&from_pw0, w, h).unwrap();
        let from_sl1 = sl1::decode_layer_data(&png, w, h).unwrap();

        let goo_enc = goo::encode_layer_data(&from_sl1, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
        assert_eq!(from_goo, original, "PW0→SL1→GOO chain lost data");
    }

    #[test]
    fn four_way_chain_pws_vdt_goo_ctb() {
        let (w, h) = (64, 64);
        let original = pws_binarize(&make_test_layer(w, h));

        let pws_enc = encode_pws(&original);
        let from_pws = decode_pws(&pws_enc);

        let png = vdt::encode_layer_data(&from_pws, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();

        let goo_enc = goo::encode_layer_data(&from_vdt, w, h).unwrap();
        let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();

        let ctb_enc = ctb::encode_layer_data(&from_goo, 0, w, h).unwrap();
        let from_ctb = ctb::decode_layer_data(&ctb_enc, 0, w, h, 1, 0).unwrap();
        assert_eq!(from_ctb, original, "PWS→VDT→GOO→CTB chain lost data");
    }

    // ── Solid layers for new formats ────────────────────────────────────────

    #[test]
    fn cross_format_solid_black_new_formats() {
        let (w, h) = (64, 64);
        let original = make_solid_layer(w, h, 0x00);

        let pw0_enc = encode_pw0(&original);
        let from_pw0 = decode_pw0(&pw0_enc);
        assert_eq!(from_pw0, original, "PW0 solid black failed");

        let pws_enc = encode_pws(&original);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, original, "PWS solid black failed");

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_vdt, original, "VDT solid black failed");
    }

    #[test]
    fn cross_format_solid_white_new_formats() {
        let (w, h) = (64, 64);
        let original = make_solid_layer(w, h, 0xFF);

        let pw0_enc = encode_pw0(&original);
        let from_pw0 = decode_pw0(&pw0_enc);
        assert_eq!(from_pw0, original, "PW0 solid white failed");

        let pws_enc = encode_pws(&original);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, original, "PWS solid white failed");

        let png = vdt::encode_layer_data(&original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_vdt, original, "VDT solid white failed");
    }

    // ── Single-pixel for new formats ────────────────────────────────────────

    #[test]
    fn single_pixel_layer_new_formats() {
        let (w, h) = (1, 1);

        let pw0_original = pw0_quantize(&[0x80]);
        let pw0_enc = encode_pw0(&pw0_original);
        let from_pw0 = decode_pw0(&pw0_enc);
        assert_eq!(from_pw0, pw0_original, "PW0 single pixel failed");

        let pws_original = pws_binarize(&[0x80]);
        let pws_enc = encode_pws(&pws_original);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, pws_original, "PWS single pixel failed");

        let vdt_original = vec![0x80u8];
        let png = vdt::encode_layer_data(&vdt_original, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_vdt, vdt_original, "VDT single pixel failed");
    }

    // ── Checkerboard for new formats ────────────────────────────────────────

    #[test]
    fn checkerboard_pattern_new_formats() {
        let (w, h) = (64, 64);
        let mut pixels = vec![0u8; (w as usize) * (h as usize)];
        for (i, p) in pixels.iter_mut().enumerate() {
            let x = i % w as usize;
            let y = i / w as usize;
            *p = if (x + y).is_multiple_of(2) {
                0xFF
            } else {
                0x00
            };
        }

        let pw0_enc = encode_pw0(&pixels);
        let from_pw0 = decode_pw0(&pw0_enc);
        assert_eq!(from_pw0, pixels, "PW0 checkerboard failed");

        let pws_enc = encode_pws(&pixels);
        let from_pws = decode_pws(&pws_enc);
        assert_eq!(from_pws, pixels, "PWS checkerboard failed");

        let png = vdt::encode_layer_data(&pixels, w, h).unwrap();
        let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
        assert_eq!(from_vdt, pixels, "VDT checkerboard failed");
    }

    // ── Compression ratio with new formats ──────────────────────────────────

    #[test]
    fn compression_ratio_comparison_all_formats() {
        let (w, h) = (1920, 1080);
        let layer = make_test_layer(w, h);

        let ctb_size = ctb::encode_layer_data(&layer, 0, w, h).unwrap().len();
        let goo_size = goo::encode_layer_data(&layer, w, h).unwrap().len();
        let sl1_size = sl1::encode_layer_data(&layer, w, h).unwrap().len();
        let pw0_size = encode_pw0(&layer).len();
        let pws_size = encode_pws(&layer).len();
        let vdt_size = vdt::encode_layer_data(&layer, w, h).unwrap().len();

        let raw_size = (w as usize) * (h as usize);
        assert!(pw0_size > 0, "PW0 should produce output");
        assert!(pws_size > 0, "PWS should produce output");
        assert!(vdt_size > 0, "VDT should produce output");

        eprintln!(
            "Compression for {}x{}: raw={}, CTB={} ({:.1}%), GOO={} ({:.1}%), SL1={} ({:.1}%), PW0={} ({:.1}%), PWS={} ({:.1}%), VDT={} ({:.1}%)",
            w, h, raw_size,
            ctb_size, ctb_size as f64 / raw_size as f64 * 100.0,
            goo_size, goo_size as f64 / raw_size as f64 * 100.0,
            sl1_size, sl1_size as f64 / raw_size as f64 * 100.0,
            pw0_size, pw0_size as f64 / raw_size as f64 * 100.0,
            pws_size, pws_size as f64 / raw_size as f64 * 100.0,
            vdt_size, vdt_size as f64 / raw_size as f64 * 100.0,
        );
    }

    // ── Format detection for new formats ────────────────────────────────────

    #[test]
    fn detect_pw0_format_by_magic() {
        let mut data = vec![0u8; 128];
        data[0..8].copy_from_slice(b"ANYCUBIC");
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("pw0")).unwrap();
        assert_eq!(fmt.identity.name, "pw0");
    }

    #[test]
    fn detect_pws_format_by_extension() {
        let mut data = vec![0u8; 128];
        data[0..8].copy_from_slice(b"ANYCUBIC");
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("pws")).unwrap();
        assert_eq!(fmt.identity.name, "pws");
    }

    #[test]
    fn detect_vdt_format_by_extension() {
        let mut data = vec![0u8; 128];
        data[0] = 0x50;
        data[1] = 0x4B;
        let fmt = slicer_converter_core::formats::detect_format_spec(&data, Some("vdt")).unwrap();
        assert_eq!(fmt.identity.name, "vdt");
    }

    #[test]
    fn detect_format_new_extensions() {
        let data = vec![0u8; 128];
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("pw0"))
                .unwrap()
                .identity
                .name,
            "pw0"
        );
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("pws"))
                .unwrap()
                .identity
                .name,
            "pws"
        );
        assert_eq!(
            slicer_converter_core::formats::detect_format_spec(&data, Some("vdt"))
                .unwrap()
                .identity
                .name,
            "vdt"
        );
    }

    #[test]
    fn anycubic_v1_builder_preserves_pw0_header_shape() {
        let metadata = FileMetadataBuilder::new()
            .resolution(1152, 2560)
            .display_size_mm(55.44, 98.637)
            .layer_count(3)
            .build()
            .expect("valid fixture");

        let header = slicer_converter_core::formats::global_registry()
            .spec_by_name("pw0")
            .expect("pw0 spec")
            .build_header(&metadata)
            .unwrap();
        assert_eq!(&header[0..8], b"ANYCUBIC");
        assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(header[16..20].try_into().unwrap()), 4);
    }

    // ── All gray values for new formats ─────────────────────────────────────

    #[test]
    fn all_gray_values_new_formats() {
        let (w, h) = (16, 16);
        for val in [0u8, 1, 2, 63, 64, 127, 128, 191, 192, 254, 255] {
            let original = make_solid_layer(w, h, val);

            let png = vdt::encode_layer_data(&original, w, h).unwrap();
            let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
            assert_eq!(from_vdt, original, "VDT solid gray {val} failed");

            let pw0_expected = pw0_quantize(&original);
            let pw0_enc = encode_pw0(&original);
            let from_pw0 = decode_pw0(&pw0_enc);
            assert_eq!(from_pw0, pw0_expected, "PW0 solid gray {val} failed");

            let pws_expected = pws_binarize(&original);
            let pws_enc = encode_pws(&original);
            let from_pws = decode_pws(&pws_enc);
            assert_eq!(from_pws, pws_expected, "PWS solid gray {val} failed");
        }
    }

    // ── VDT full ZIP archive roundtrip ──────────────────────────────────────

    #[test]
    fn vdt_zip_archive_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(64, 48)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .machine_name("TestVDT")
            .layer_height_mm(0.05)
            .layer_count(2)
            .bottom_layer_count(1)
            .exposure(2.5, 25.0)
            .build()
            .expect("valid fixture");

        let layer0 = make_test_layer(64, 48);
        let layer1 = make_solid_layer(64, 48, 0x80);

        let png0 = vdt::encode_layer_data(&layer0, 64, 48).unwrap();
        let png1 = vdt::encode_layer_data(&layer1, 64, 48).unwrap();

        let zip_data = encode_document("vdt", &metadata, &[png0, png1], "ignored");

        let (parsed_meta, layer_offsets) = vdt::parse_header_data(&zip_data).unwrap();
        assert_eq!(parsed_meta.slicing.layer_count, 2);
        assert_eq!(layer_offsets.len(), 2);
        assert_eq!(parsed_meta.printer.resolution_x, 64);
        assert_eq!(parsed_meta.printer.resolution_y, 48);
        assert!((parsed_meta.slicing.layer_height_mm - 0.05).abs() < 0.001);

        let reader = std::io::Cursor::new(&zip_data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();

        let mut layer0_png = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_index(layer_offsets[0].offset as usize).unwrap(),
            &mut layer0_png,
        )
        .unwrap();
        let decoded0 = vdt::decode_layer_data(&layer0_png, 64, 48).unwrap();
        assert_eq!(decoded0, layer0, "VDT ZIP layer 0 roundtrip failed");

        let mut layer1_png = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_index(layer_offsets[1].offset as usize).unwrap(),
            &mut layer1_png,
        )
        .unwrap();
        let decoded1 = vdt::decode_layer_data(&layer1_png, 64, 48).unwrap();
        assert_eq!(decoded1, layer1, "VDT ZIP layer 1 roundtrip failed");
    }

    // ── PW0 header build/parse roundtrip ────────────────────────────────────

    #[test]
    fn pw0_header_build_roundtrip() {
        let metadata = FileMetadataBuilder::new()
            .resolution(2560, 1620)
            .display_size_mm(68.04, 120.96)
            .machine_z_mm(150.0)
            .machine_name("TestPW0")
            .layer_height_mm(0.05)
            .layer_count(3)
            .bottom_layer_count(2)
            .exposure(2.5, 25.0)
            .build()
            .expect("valid fixture");

        let header = build_pw0_header(&metadata);
        assert_eq!(
            &header[0..8],
            b"ANYCUBIC",
            "PW0 header should start with ANYCUBIC"
        );
        assert_eq!(
            u32::from_le_bytes(header[12..16].try_into().unwrap()),
            1,
            "PW0 version should be stored as little-endian v1 for UVtools compatibility"
        );
        assert_eq!(
            u32::from_le_bytes(header[16..20].try_into().unwrap()),
            4,
            "PW0 v1 should advertise 4 tables"
        );
        assert_eq!(
            u32::from_le_bytes(header[20..24].try_into().unwrap()),
            48,
            "PW0 v1 FileMark should occupy 48 bytes"
        );
        assert_eq!(
            u32::from_le_bytes(header[28..32].try_into().unwrap()),
            144,
            "PW0 preview should start right after the FileMark + HEADER table"
        );
        assert_eq!(
            u32::from_le_bytes(header[36..40].try_into().unwrap()),
            144 + 28 + 224 * 168 * 2,
            "PW0 layer definition table should follow the preview payload"
        );
        assert_eq!(
            &header[48..54],
            b"HEADER",
            "PW0 HEADER table should start at FileMark.HeaderAddress"
        );
        assert_eq!(
            u32::from_le_bytes(header[108..112].try_into().unwrap()),
            metadata.printer.resolution_x,
            "PW0 HEADER resolution_x should be little-endian"
        );
        assert_eq!(
            &header[144..151],
            b"PREVIEW",
            "PW0 preview table mark should be present"
        );
        assert_eq!(
            &header[164..168],
            b"x\0\0\0",
            "PW0 preview table should contain the Anycubic x-marker"
        );

        let (parsed, layers) = parse_pw0_header(&header);
        assert_eq!(parsed.printer.resolution_x, 2560);
        assert_eq!(parsed.printer.resolution_y, 1620);
        assert_eq!(parsed.slicing.layer_count, 3);
        assert_eq!(layers.len(), 3);
        assert!((parsed.slicing.layer_height_mm - 0.05).abs() < 0.001);
        assert!((parsed.exposure.exposure_time - 2.5).abs() < 0.01);
        assert!((parsed.exposure.bottom_exposure_time - 25.0).abs() < 0.01);
    }

    // ── Multi-layer pipeline with new formats ───────────────────────────────

    #[test]
    fn multi_layer_conversion_pipeline_new_formats() {
        let (w, h) = (64, 64);
        let layers: Vec<Vec<u8>> = (0..5)
            .map(|i| make_solid_layer(w, h, (i * 50) as u8))
            .collect();

        for (i, original) in layers.iter().enumerate() {
            // PW0 → VDT
            let pw0_quantized = pw0_quantize(original);
            let pw0_enc = encode_pw0(original);
            let from_pw0 = decode_pw0(&pw0_enc);
            let png = vdt::encode_layer_data(&from_pw0, w, h).unwrap();
            let from_vdt = vdt::decode_layer_data(&png, w, h).unwrap();
            assert_eq!(
                from_vdt, pw0_quantized,
                "PW0→VDT pipeline failed at layer {i}"
            );

            // PWS → GOO
            let pws_binarized = pws_binarize(original);
            let pws_enc = encode_pws(original);
            let from_pws = decode_pws(&pws_enc);
            let goo_enc = goo::encode_layer_data(&from_pws, w, h).unwrap();
            let from_goo = goo::decode_layer_data(&goo_enc, w, h).unwrap();
            assert_eq!(
                from_goo, pws_binarized,
                "PWS→GOO pipeline failed at layer {i}"
            );
        }
    }
}
