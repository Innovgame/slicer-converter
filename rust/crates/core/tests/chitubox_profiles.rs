use aes::Aes256;
use cbc::cipher::{block_padding::NoPadding, BlockEncryptMut, KeyIvInit};
use sha2::{Digest, Sha256};
use slicer_converter_core::common::{ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset};
use slicer_converter_core::formats::{
    self,
    chitubox::specifics::ChituboxProfileKind,
    common::{
        keys, ArchiveEncodeContext, ContainerKind, ExportTargetOptions, FormatFamily, FormatSpec,
        LayerSource, VecOutputSink,
    },
    ctb, LayerDecodeContext, LayerEncodeContext, LayerOptions,
};

fn chitubox_header_bytes(magic: u32, version: u32) -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[0..4].copy_from_slice(&magic.to_le_bytes());
    header[4..8].copy_from_slice(&version.to_le_bytes());
    header
}

fn le_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .expect("u32 slice should be in bounds"),
    )
}

fn le_f32(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .expect("f32 slice should be in bounds"),
    )
}

fn be_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        data[offset..offset + 4]
            .try_into()
            .expect("u32 slice should be in bounds"),
    )
}

fn be_f64(data: &[u8], offset: usize) -> f64 {
    f64::from_be_bytes(
        data[offset..offset + 8]
            .try_into()
            .expect("f64 slice should be in bounds"),
    )
}

type Aes256CbcEnc = cbc::Encryptor<Aes256>;

fn chitubox_specifics(
    spec: &'static FormatSpec,
) -> &'static slicer_converter_core::formats::chitubox::specifics::ChituboxSpec {
    formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("chitubox profile")
        .as_chitubox()
        .expect("chitubox specifics")
}

const ENCRYPTED_CTB_MAGIC: u32 = 0x12FD_0107;
const ENCRYPTED_CTB_AES_KEY: [u8; 32] = [
    0xD0, 0x5B, 0x8E, 0x33, 0x71, 0xDE, 0x3D, 0x1A, 0xE5, 0x4F, 0x22, 0xDD, 0xDF, 0x5B, 0xFD, 0x94,
    0xAB, 0x5D, 0x64, 0x3A, 0x9D, 0x7E, 0xBF, 0xAF, 0x42, 0x03, 0xF3, 0x10, 0xD8, 0x52, 0x2A, 0xEA,
];
const ENCRYPTED_CTB_AES_IV: [u8; 16] = [
    0x0F, 0x01, 0x0A, 0x05, 0x05, 0x0B, 0x06, 0x07, 0x08, 0x06, 0x0A, 0x0C, 0x0C, 0x0D, 0x09, 0x0F,
];

fn put_u8(data: &mut [u8], offset: usize, value: u8) {
    data[offset] = value;
}

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(data: &mut [u8], offset: usize, value: f32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_be_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_be_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn put_be_f64(data: &mut [u8], offset: usize, value: f64) {
    data[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
}

fn write_ctb_layer_def(
    data: &mut [u8],
    offset: usize,
    position_z: f32,
    exposure: f32,
    data_offset: usize,
    data_len: usize,
    table_size: u32,
) {
    put_f32(data, offset, position_z);
    put_f32(data, offset + 4, exposure);
    put_f32(data, offset + 8, 0.0);
    put_u32(data, offset + 12, data_offset as u32);
    put_u32(data, offset + 16, data_len as u32);
    put_u32(data, offset + 20, 0);
    put_u32(data, offset + 24, table_size);
    put_u32(data, offset + 28, 0);
    put_u32(data, offset + 32, 0);
}

fn encrypt_ctb_block(block: &[u8]) -> Vec<u8> {
    Aes256CbcEnc::new(&ENCRYPTED_CTB_AES_KEY.into(), &ENCRYPTED_CTB_AES_IV.into())
        .encrypt_padded_vec_mut::<NoPadding>(block)
}

fn minimal_chitubox_export_metadata() -> FileMetadata {
    FileMetadataBuilder::new()
        .resolution(2, 2)
        .display_size_mm(68.04, 120.96)
        .machine_z_mm(165.0)
        .machine_name("Mock Chitubox Export")
        .mirror(true, false)
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .anti_alias_level(1)
        .exposure(2.4, 30.0)
        .light_off_delay(0.8, 0.4)
        .light_pwm(200, 210)
        .lift(7.0, 70.0)
        .bottom_lift(6.0, 60.0)
        .retract(150.0)
        .statistics(42, 1.25, 1.4, 0.33)
        .build()
        .expect("valid metadata")
}

fn encode_binary_document(
    spec: &'static FormatSpec,
    metadata: &FileMetadata,
    encoded_layers: &[Vec<u8>],
) -> Vec<u8> {
    let mut sink = VecOutputSink::new();
    let ctx = ArchiveEncodeContext::new("ignored");
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

fn encode_layers(
    spec: &'static FormatSpec,
    metadata: &FileMetadata,
    source_layers: &[Vec<u8>],
) -> Vec<Vec<u8>> {
    source_layers
        .iter()
        .enumerate()
        .map(|(index, pixels)| {
            spec.encode_layer(
                pixels,
                &LayerEncodeContext {
                    layer_index: index as u32,
                    width: metadata.printer.resolution_x,
                    height: metadata.printer.resolution_y,
                    layer_metadata: Some(metadata.as_layer_encode_metadata()),
                },
            )
            .expect("layer encode")
        })
        .collect()
}

fn parse_document(
    spec: &'static FormatSpec,
    data: &[u8],
    context: &str,
) -> (FileMetadata, Vec<LayerOffset>) {
    let parsed = spec.pipeline.parse_document(spec, data).expect(context);
    (parsed.metadata, parsed.layers)
}

#[test]
fn ctb_export_preserves_layer_xor_key_and_roundtrips_pixels() {
    let spec = formats::get_format_spec_by_name("ctb").expect("ctb spec");
    let mut metadata = minimal_chitubox_export_metadata();
    metadata.slicing.encryption_key = 42;
    let source_layers = [vec![255, 255, 0, 0], vec![0, 255, 255, 0]];
    let encoded_layers = encode_layers(spec, &metadata, &source_layers);
    let document = encode_binary_document(spec, &metadata, &encoded_layers);
    let (parsed, layers) = parse_document(spec, &document, "parse encrypted CTB export");

    assert_eq!(parsed.slicing.encryption_key, 42);
    let decoded = layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            let start = layer.offset as usize;
            let end = start + layer.size as usize;
            spec.decode_layer(
                &document[start..end],
                &LayerDecodeContext::new(
                    index as u32,
                    parsed.printer.resolution_x,
                    parsed.printer.resolution_y,
                )
                .with_options(LayerOptions::default().with_encryption_key(42)),
            )
            .expect("decode encrypted CTB layer")
        })
        .collect::<Vec<_>>();
    assert_eq!(decoded, source_layers);
}

#[test]
fn ctb_v4_export_splits_uvtools_lift_total() {
    let spec = formats::global_registry()
        .spec_by_name("ctb-v4")
        .expect("ctb v4 spec");
    let mut metadata = minimal_chitubox_export_metadata();
    metadata.motion.lift_height = 6.0;
    metadata.motion.lift_height2 = 1.5;
    metadata.motion.bottom_lift_height = 8.0;
    let source_layers = [vec![255, 255, 0, 0], vec![0, 255, 255, 0]];
    let encoded_layers = encode_layers(spec, &metadata, &source_layers);
    let document = encode_binary_document(spec, &metadata, &encoded_layers);

    let print_params_offset = le_u32(&document, 0x54) as usize;
    let stored_lift = f32::from_le_bytes(
        document[print_params_offset + 8..print_params_offset + 12]
            .try_into()
            .expect("stored lift height"),
    );
    assert_eq!(stored_lift, 7.5, "v4 PrintParameters stores the lift total");

    let (parsed, _) = parse_document(spec, &document, "parse v4 lift split");
    assert_eq!(parsed.motion.lift_height, 6.0);
    assert_eq!(parsed.motion.lift_height2, 1.5);
    assert_eq!(parsed.motion.bottom_lift_height, 8.0);
}

#[test]
fn ctb_export_roundtrips_projector_mirror_flag() {
    let spec = formats::get_format_spec_by_name("ctb").expect("ctb spec");
    let mut metadata = minimal_chitubox_export_metadata();
    metadata.printer.mirror_x = false;
    let source_layers = [vec![255, 255, 0, 0], vec![0, 255, 255, 0]];
    let encoded_layers = encode_layers(spec, &metadata, &source_layers);
    let document = encode_binary_document(spec, &metadata, &encoded_layers);

    assert_eq!(
        le_u32(&document, 0x50),
        0,
        "unmirrored CTB keeps projector type 0"
    );
    let (parsed, _) = parse_document(spec, &document, "parse unmirrored CTB");
    assert!(!parsed.printer.mirror_x);

    metadata.printer.mirror_x = true;
    let encoded_layers = encode_layers(spec, &metadata, &source_layers);
    let document = encode_binary_document(spec, &metadata, &encoded_layers);
    assert_eq!(
        le_u32(&document, 0x50),
        1,
        "mirrored CTB writes projector type 1"
    );
    let (parsed, _) = parse_document(spec, &document, "parse mirrored CTB");
    assert!(parsed.printer.mirror_x);
}

#[test]
fn ctb_export_roundtrips_slicer_info_anti_alias_level() {
    let spec = formats::global_registry()
        .spec_by_name("ctb-v2")
        .expect("ctb v2 spec");
    let mut metadata = minimal_chitubox_export_metadata();
    metadata.slicing.anti_alias_level = 4;
    let source_layers = [vec![255, 255, 0, 0], vec![0, 255, 255, 0]];
    let encoded_layers = encode_layers(spec, &metadata, &source_layers);
    let document = encode_binary_document(spec, &metadata, &encoded_layers);

    assert_eq!(le_u32(&document, 0x5C), 1, "CTB header AA stays 1");
    let (parsed, _) = parse_document(spec, &document, "parse CTB AA export");
    assert_eq!(parsed.slicing.anti_alias_level, 4);
}

#[test]
fn cbddlp_export_roundtrips_header_anti_alias_level() {
    let spec = formats::global_registry()
        .spec_by_name("cbddlp")
        .expect("cbddlp spec");
    let mut metadata = minimal_chitubox_export_metadata();
    metadata.slicing.anti_alias_level = 2;
    let source_layers = [vec![255, 255, 0, 0], vec![0, 255, 255, 0]];
    let encoded_layers = encode_layers(spec, &metadata, &source_layers);
    let document = encode_binary_document(spec, &metadata, &encoded_layers);

    assert_eq!(le_u32(&document, 0x5C), 2, "CBDDLP header keeps AA level");
    let (parsed, _) = parse_document(spec, &document, "parse CBDDLP AA export");
    assert_eq!(parsed.slicing.anti_alias_level, 2);
}

fn assert_binary_roundtrip(
    format_name: &str,
    source_layers: &[Vec<u8>],
) -> (FileMetadata, Vec<Vec<u8>>) {
    let spec = formats::get_format_spec_by_name(format_name).expect("format spec");
    let metadata = minimal_chitubox_export_metadata();
    let encoded_layers = encode_layers(spec, &metadata, source_layers);

    let output = encode_binary_document(spec, &metadata, &encoded_layers);
    let (parsed_metadata, layers) = parse_document(spec, &output, "parse encoded document");
    assert_eq!(layers.len(), source_layers.len());

    let decoded = layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            let start = layer.offset as usize;
            let end = start + layer.size as usize;
            spec.decode_layer(
                &output[start..end],
                &LayerDecodeContext::new(
                    index as u32,
                    parsed_metadata.printer.resolution_x,
                    parsed_metadata.printer.resolution_y,
                ),
            )
            .expect("decode encoded document layer")
        })
        .collect::<Vec<_>>();

    (parsed_metadata, decoded)
}

fn assert_ctb_rle_fills_exact_pixels(data: &[u8], width: u32, height: u32, label: &str) {
    let expected_pixels = (width as usize) * (height as usize);
    let mut offset = 0usize;
    let mut pixels = 0usize;

    while offset < data.len() {
        let byte0 = data[offset];
        offset += 1;
        let mut stride = 1usize;

        if byte0 & 0x80 != 0 {
            let slen = *data.get(offset).unwrap_or_else(|| {
                panic!("{label}: truncated CTB run stride at stream offset {offset}")
            });
            offset += 1;

            stride = if slen & 0x80 == 0 {
                slen as usize
            } else if slen & 0xC0 == 0x80 {
                let b1 = *data.get(offset).unwrap_or_else(|| {
                    panic!("{label}: truncated CTB 2-byte stride at stream offset {offset}")
                });
                offset += 1;
                (((slen & 0x3F) as usize) << 8) | b1 as usize
            } else if slen & 0xE0 == 0xC0 {
                let b1 = *data.get(offset).unwrap_or_else(|| {
                    panic!("{label}: truncated CTB 3-byte stride at stream offset {offset}")
                });
                let b2 = *data.get(offset + 1).unwrap_or_else(|| {
                    panic!("{label}: truncated CTB 3-byte stride at stream offset {offset}")
                });
                offset += 2;
                (((slen & 0x1F) as usize) << 16) | ((b1 as usize) << 8) | b2 as usize
            } else if slen & 0xF0 == 0xE0 {
                let b1 = *data.get(offset).unwrap_or_else(|| {
                    panic!("{label}: truncated CTB 4-byte stride at stream offset {offset}")
                });
                let b2 = *data.get(offset + 1).unwrap_or_else(|| {
                    panic!("{label}: truncated CTB 4-byte stride at stream offset {offset}")
                });
                let b3 = *data.get(offset + 2).unwrap_or_else(|| {
                    panic!("{label}: truncated CTB 4-byte stride at stream offset {offset}")
                });
                offset += 3;
                (((slen & 0x0F) as usize) << 24)
                    | ((b1 as usize) << 16)
                    | ((b2 as usize) << 8)
                    | b3 as usize
            } else {
                panic!("{label}: corrupted CTB stride marker {slen:#04x}");
            };
        }

        assert!(
            pixels + stride <= expected_pixels,
            "{label}: CTB RLE run exceeds image: pos={pixels} stride={stride} expected={expected_pixels}"
        );
        pixels += stride;
    }

    assert_eq!(
        pixels, expected_pixels,
        "{label}: CTB RLE stream should fill exactly one layer"
    );
}

fn generated_phz_fixture() -> Vec<u8> {
    let header_size = 216usize;
    let machine = b"Mock PHZ";
    let machine_name_offset = header_size;
    let layer_table_offset = machine_name_offset + machine.len();
    let layer_def_size = 36usize;
    let layer0_data_offset = layer_table_offset + layer_def_size * 2;
    let layer0_rle = [0x00, 0xFF, 0x03];
    let layer1_data_offset = layer0_data_offset + layer0_rle.len();
    let layer1_rle = [0x80, 0x00, 0x03];

    let mut data = vec![0u8; layer1_data_offset + layer1_rle.len()];
    put_u32(&mut data, 0, 0x9FDA_83AE);
    put_u32(&mut data, 4, 2);
    put_f32(&mut data, 8, 0.05);
    put_f32(&mut data, 12, 2.4);
    put_f32(&mut data, 16, 30.0);
    put_u32(&mut data, 20, 1);
    put_u32(&mut data, 24, 2);
    put_u32(&mut data, 28, 2);
    put_u32(&mut data, 36, layer_table_offset as u32);
    put_u32(&mut data, 40, 2);
    put_u32(&mut data, 48, 42);
    put_u32(&mut data, 52, 1);
    put_u32(&mut data, 56, 1);
    put_u16(&mut data, 60, 200);
    put_u16(&mut data, 62, 210);
    put_f32(&mut data, 72, 0.10);
    put_f32(&mut data, 76, 68.04);
    put_f32(&mut data, 80, 120.96);
    put_f32(&mut data, 84, 165.0);
    put_u32(&mut data, 88, 0);
    put_f32(&mut data, 92, 0.4);
    put_f32(&mut data, 96, 0.8);
    put_u32(&mut data, 100, 1);
    put_f32(&mut data, 108, 6.0);
    put_f32(&mut data, 112, 60.0);
    put_f32(&mut data, 116, 7.0);
    put_f32(&mut data, 120, 70.0);
    put_f32(&mut data, 124, 150.0);
    put_f32(&mut data, 128, 1.25);
    put_f32(&mut data, 132, 1.40);
    put_f32(&mut data, 136, 0.33);
    put_u32(&mut data, 144, machine_name_offset as u32);
    put_u32(&mut data, 148, machine.len() as u32);
    put_u32(&mut data, 176, 28);
    put_u32(&mut data, 184, 1);
    put_u32(&mut data, 188, 0x01060300);
    data[machine_name_offset..machine_name_offset + machine.len()].copy_from_slice(machine);

    write_phz_layer_def(
        &mut data,
        layer_table_offset,
        0.05,
        30.0,
        layer0_data_offset,
        layer0_rle.len(),
    );
    write_phz_layer_def(
        &mut data,
        layer_table_offset + layer_def_size,
        0.10,
        2.4,
        layer1_data_offset,
        layer1_rle.len(),
    );
    data[layer0_data_offset..layer0_data_offset + layer0_rle.len()].copy_from_slice(&layer0_rle);
    data[layer1_data_offset..layer1_data_offset + layer1_rle.len()].copy_from_slice(&layer1_rle);

    data
}

fn write_phz_layer_def(
    data: &mut [u8],
    offset: usize,
    position_z: f32,
    exposure: f32,
    data_offset: usize,
    data_len: usize,
) {
    put_f32(data, offset, position_z);
    put_f32(data, offset + 4, exposure);
    put_f32(data, offset + 8, 0.8);
    put_u32(data, offset + 12, data_offset as u32);
    put_u32(data, offset + 16, data_len as u32);
}

fn generated_fdg_fixture() -> Vec<u8> {
    let header_size = 216usize;
    let machine = b"Mock FDG";
    let machine_name_offset = header_size;
    let layer_table_offset = machine_name_offset + machine.len();
    let layer_def_size = 36usize;
    let layer0_data_offset = layer_table_offset + layer_def_size * 2;
    let layer0_rle = [0x00, 0xFF, 0x03];
    let layer1_data_offset = layer0_data_offset + layer0_rle.len();
    let layer1_rle = [0x80, 0x00, 0x03];

    let mut data = vec![0u8; layer1_data_offset + layer1_rle.len()];
    put_u32(&mut data, 0, 0xBD3C_7AC8);
    put_u32(&mut data, 4, 2);
    put_u32(&mut data, 8, 2);
    put_u32(&mut data, 12, 1);
    put_u32(&mut data, 16, 1);
    put_u32(&mut data, 20, 1);
    put_u32(&mut data, 24, 2);
    put_u32(&mut data, 28, 2);
    put_f32(&mut data, 32, 0.05);
    put_f32(&mut data, 36, 2.4);
    put_f32(&mut data, 40, 30.0);
    put_u32(&mut data, 52, layer_table_offset as u32);
    put_u32(&mut data, 56, 42);
    put_u32(&mut data, 60, 1);
    put_u16(&mut data, 64, 200);
    put_u16(&mut data, 66, 210);
    put_f32(&mut data, 76, 0.10);
    put_f32(&mut data, 80, 68.04);
    put_f32(&mut data, 84, 120.96);
    put_f32(&mut data, 88, 165.0);
    put_u32(&mut data, 92, 0);
    put_u32(&mut data, 96, 1);
    put_u32(&mut data, 100, 0x4c);
    put_f32(&mut data, 104, 1.25);
    put_f32(&mut data, 108, 1.40);
    put_f32(&mut data, 112, 0.33);
    put_u32(&mut data, 116, machine_name_offset as u32);
    put_u32(&mut data, 120, machine.len() as u32);
    put_f32(&mut data, 124, 0.4);
    put_f32(&mut data, 128, 0.8);
    put_f32(&mut data, 136, 6.0);
    put_f32(&mut data, 140, 60.0);
    put_f32(&mut data, 144, 7.0);
    put_f32(&mut data, 148, 70.0);
    put_f32(&mut data, 152, 150.0);
    put_u32(&mut data, 188, 0x01060300);
    data[machine_name_offset..machine_name_offset + machine.len()].copy_from_slice(machine);

    write_phz_layer_def(
        &mut data,
        layer_table_offset,
        0.05,
        30.0,
        layer0_data_offset,
        layer0_rle.len(),
    );
    write_phz_layer_def(
        &mut data,
        layer_table_offset + layer_def_size,
        0.10,
        2.4,
        layer1_data_offset,
        layer1_rle.len(),
    );
    data[layer0_data_offset..layer0_data_offset + layer0_rle.len()].copy_from_slice(&layer0_rle);
    data[layer1_data_offset..layer1_data_offset + layer1_rle.len()].copy_from_slice(&layer1_rle);

    data
}

fn generated_photons_fixture() -> Vec<u8> {
    let header_size = 98usize;
    let preview_offset = header_size;
    let preview_bytes = [0xFF, 0xFF];
    let layer_count_offset = preview_offset + preview_bytes.len();
    let layer0_def_offset = layer_count_offset + 4;
    let layer_def_size = 28usize;
    let layer0_rle = [0xC1];
    let layer0_data_offset = layer0_def_offset + layer_def_size;
    let layer1_def_offset = layer0_data_offset + layer0_rle.len();
    let layer1_rle = [0xC0];
    let layer1_data_offset = layer1_def_offset + layer_def_size;

    let mut data = vec![0u8; layer1_data_offset + layer1_rle.len()];
    put_be_u32(&mut data, 0, 2);
    put_be_u16(&mut data, 4, 49);
    put_be_f64(&mut data, 6, 0.04725);
    put_be_f64(&mut data, 14, 0.05);
    put_be_f64(&mut data, 22, 2.4);
    put_be_f64(&mut data, 30, 0.8);
    put_be_f64(&mut data, 38, 30.0);
    put_be_u32(&mut data, 46, 1);
    put_be_f64(&mut data, 50, 5.0);
    put_be_f64(&mut data, 58, 1.2);
    put_be_f64(&mut data, 66, 2.3);
    put_be_f64(&mut data, 74, 1.25);
    put_be_u32(&mut data, 82, 1);
    put_be_u32(&mut data, 86, 42);
    put_be_u32(&mut data, 90, 1);
    put_be_u32(&mut data, 94, 10);
    data[preview_offset..preview_offset + preview_bytes.len()].copy_from_slice(&preview_bytes);

    put_be_u32(&mut data, layer_count_offset, 2);
    write_photons_layer_def(&mut data, layer0_def_offset, 2, 2, layer0_rle.len() as u32);
    data[layer0_data_offset..layer0_data_offset + layer0_rle.len()].copy_from_slice(&layer0_rle);
    write_photons_layer_def(&mut data, layer1_def_offset, 2, 2, layer1_rle.len() as u32);
    data[layer1_data_offset..layer1_data_offset + layer1_rle.len()].copy_from_slice(&layer1_rle);

    data
}

fn write_photons_layer_def(data: &mut [u8], offset: usize, width: u32, height: u32, rle_len: u32) {
    put_be_u32(data, offset, 44944);
    put_be_u32(data, offset + 12, width);
    put_be_u32(data, offset + 16, height);
    put_be_u32(data, offset + 20, (rle_len + 4) << 3);
    put_be_u32(data, offset + 24, 2_684_702_720);
}

fn generated_encrypted_ctb_fixture() -> Vec<u8> {
    let checksum = 0x0123_4567_89AB_CDEFu64;
    let machine = b"Mock Encrypted";
    let settings_offset = 48usize;
    let settings_size = 288usize;
    let signature_offset = settings_offset + settings_size;
    let machine_name_offset = signature_offset + 32;
    let layer_pointers_offset = machine_name_offset + machine.len();
    let layer0_def_offset = layer_pointers_offset + 32;
    let layer1_def_offset = layer0_def_offset + 88;
    let layer0_data_offset = layer1_def_offset + 88;
    let layer0_rle = [0xFF, 0x04];
    let layer1_data_offset = layer0_data_offset + layer0_rle.len();
    let layer1_rle = [0x80, 0x04];

    let mut settings = vec![0u8; settings_size];
    put_u64(&mut settings, 0, checksum);
    put_u32(&mut settings, 8, layer_pointers_offset as u32);
    put_f32(&mut settings, 12, 68.04);
    put_f32(&mut settings, 16, 120.96);
    put_f32(&mut settings, 20, 165.0);
    put_f32(&mut settings, 32, 0.10);
    put_f32(&mut settings, 36, 0.05);
    put_f32(&mut settings, 40, 2.5);
    put_f32(&mut settings, 44, 35.0);
    put_f32(&mut settings, 48, 1.2);
    put_u32(&mut settings, 52, 1);
    put_u32(&mut settings, 56, 2);
    put_u32(&mut settings, 60, 2);
    put_u32(&mut settings, 64, 2);
    put_u32(&mut settings, 76, 42);
    put_f32(&mut settings, 84, 6.0);
    put_f32(&mut settings, 88, 60.0);
    put_f32(&mut settings, 92, 7.0);
    put_f32(&mut settings, 96, 70.0);
    put_f32(&mut settings, 100, 150.0);
    put_f32(&mut settings, 104, 1.25);
    put_f32(&mut settings, 108, 1.40);
    put_f32(&mut settings, 112, 0.33);
    put_f32(&mut settings, 116, 0.4);
    put_u32(&mut settings, 120, 1);
    put_u16(&mut settings, 124, 200);
    put_u16(&mut settings, 126, 210);
    put_u32(&mut settings, 128, 0);
    put_f32(&mut settings, 132, 1.0);
    put_f32(&mut settings, 136, 20.0);
    put_f32(&mut settings, 140, 2.0);
    put_f32(&mut settings, 144, 30.0);
    put_f32(&mut settings, 148, 3.0);
    put_f32(&mut settings, 152, 40.0);
    put_f32(&mut settings, 156, 0.6);
    put_u32(&mut settings, 160, machine_name_offset as u32);
    put_u32(&mut settings, 164, machine.len() as u32);
    put_u8(&mut settings, 168, 0x0F);
    put_u16(&mut settings, 169, 0);
    put_u8(&mut settings, 171, 0x40);
    put_u32(&mut settings, 176, 1);
    put_f32(&mut settings, 180, 0.7);
    put_f32(&mut settings, 184, 0.8);
    put_u32(&mut settings, 188, 1);
    put_f32(&mut settings, 192, 45.0);
    put_f32(&mut settings, 196, 46.0);
    put_f32(&mut settings, 204, 4.0);
    put_f32(&mut settings, 212, 4.0);
    put_f32(&mut settings, 216, 0.9);
    put_f32(&mut settings, 220, 1.0);
    put_f32(&mut settings, 224, 1.1);
    put_u32(&mut settings, 240, 4);
    put_u32(&mut settings, 244, 1);

    let mut data = vec![0u8; layer1_data_offset + layer1_rle.len()];
    put_u32(&mut data, 0, ENCRYPTED_CTB_MAGIC);
    put_u32(&mut data, 4, settings_size as u32);
    put_u32(&mut data, 8, settings_offset as u32);
    put_u32(&mut data, 16, 5);
    put_u32(&mut data, 20, 32);
    put_u32(&mut data, 24, signature_offset as u32);
    put_u16(&mut data, 32, 1);
    put_u16(&mut data, 34, 1);
    put_u32(&mut data, 40, 42);

    let encrypted_settings = encrypt_ctb_block(&settings);
    data[settings_offset..settings_offset + settings_size].copy_from_slice(&encrypted_settings);

    let checksum_hash = Sha256::digest(checksum.to_le_bytes());
    let encrypted_signature = encrypt_ctb_block(&checksum_hash);
    data[signature_offset..signature_offset + 32].copy_from_slice(&encrypted_signature);
    data[machine_name_offset..machine_name_offset + machine.len()].copy_from_slice(machine);

    for (i, layer_def_offset) in [layer0_def_offset, layer1_def_offset].iter().enumerate() {
        let pointer = layer_pointers_offset + i * 16;
        put_u32(&mut data, pointer, *layer_def_offset as u32);
        put_u32(&mut data, pointer + 8, 88);
    }

    put_u32(&mut data, layer0_def_offset, 88);
    put_f32(&mut data, layer0_def_offset + 4, 0.05);
    put_f32(&mut data, layer0_def_offset + 8, 35.0);
    put_f32(&mut data, layer0_def_offset + 12, 0.4);
    put_u32(&mut data, layer0_def_offset + 16, layer0_data_offset as u32);
    put_u32(&mut data, layer0_def_offset + 24, layer0_rle.len() as u32);
    put_f32(&mut data, layer0_def_offset + 40, 6.0);
    put_f32(&mut data, layer0_def_offset + 44, 60.0);
    put_f32(&mut data, layer0_def_offset + 72, 0.7);
    put_f32(&mut data, layer0_def_offset + 80, 0.8);
    put_f32(&mut data, layer0_def_offset + 84, 210.0);

    put_u32(&mut data, layer1_def_offset, 88);
    put_f32(&mut data, layer1_def_offset + 4, 0.10);
    put_f32(&mut data, layer1_def_offset + 8, 2.5);
    put_f32(&mut data, layer1_def_offset + 12, 1.2);
    put_u32(&mut data, layer1_def_offset + 16, layer1_data_offset as u32);
    put_u32(&mut data, layer1_def_offset + 24, layer1_rle.len() as u32);
    put_f32(&mut data, layer1_def_offset + 40, 7.0);
    put_f32(&mut data, layer1_def_offset + 44, 70.0);
    put_f32(&mut data, layer1_def_offset + 72, 0.7);
    put_f32(&mut data, layer1_def_offset + 80, 0.8);
    put_f32(&mut data, layer1_def_offset + 84, 200.0);

    data[layer0_data_offset..layer0_data_offset + layer0_rle.len()].copy_from_slice(&layer0_rle);
    data[layer1_data_offset..layer1_data_offset + layer1_rle.len()].copy_from_slice(&layer1_rle);
    data
}

#[test]
fn chitubox_family_exposes_distinct_spec_profiles_for_extension_targets() {
    let registry = formats::global_registry();
    let ctb = registry.spec_by_name("ctb").expect("ctb spec");
    let ctb_v2 = registry.spec_by_name("ctb-v2").expect("ctb v2 spec");
    let ctb_v4 = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let cbddlp = registry.spec_by_name("cbddlp").expect("cbddlp spec");
    let photon = registry.spec_by_name("photon").expect("photon spec");

    assert_eq!(ctb.identity.name, "ctb");
    assert_eq!(ctb.catalog.profile_id, "chitubox-ctb-v2");
    assert_eq!(chitubox_specifics(ctb).ctb_version, Some(2));
    assert_eq!(ctb.keys.header_key, keys::HEADER_CHITU_CTB_V2);
    assert_eq!(ctb.keys.layer_codec_key, keys::CTB_RLE);

    assert_eq!(ctb_v2.identity.name, "ctb");
    assert_eq!(ctb_v2.catalog.profile_id, "chitubox-ctb-v2");
    assert_eq!(chitubox_specifics(ctb_v2).ctb_version, Some(2));
    assert_eq!(ctb_v2.keys.header_key, keys::HEADER_CHITU_CTB_V2);

    assert_eq!(ctb_v4.identity.name, "ctb-v4");
    assert_eq!(ctb_v4.catalog.profile_id, "chitubox-ctb-v4");
    assert_eq!(chitubox_specifics(ctb_v4).ctb_version, Some(4));
    assert_eq!(ctb_v4.keys.header_key, keys::HEADER_CHITU_CTB_V4);

    assert_eq!(cbddlp.identity.name, "cbddlp");
    assert_eq!(cbddlp.catalog.profile_id, "chitubox-cbddlp");
    assert_eq!(cbddlp.keys.layer_codec_key, keys::CBDDLP_BITPLANE);
    assert_eq!(cbddlp.keys.pixel_model_key, keys::PIXEL_BITPLANE_AA);

    assert_eq!(photon.identity.name, "photon");
    assert_eq!(photon.catalog.profile_id, "chitubox-photon");
    assert_eq!(photon.keys.layer_codec_key, keys::CBDDLP_BITPLANE);
    assert_eq!(photon.keys.pixel_model_key, keys::PIXEL_BITPLANE_AA);
}

#[test]
fn chitubox_photon_profile_builds_cbddlp_magic_header() {
    let registry = formats::global_registry();
    let photon = registry.spec_by_name("photon").expect("photon spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .anti_alias_level(2)
        .build()
        .expect("metadata");

    let header = photon
        .header
        .build_header(&metadata, photon, ExportTargetOptions::default())
        .expect("header");

    assert_eq!(le_u32(&header, 0), ctb::MAGIC_CBDDLP);
    assert_eq!(le_u32(&header, 4), 2);
}

#[test]
fn chitubox_export_profiles_follow_uvtools_suffix_magic_version_matrix() {
    let registry = formats::global_registry();
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");

    for (name, preferred_target, expected_magic, expected_version, expected_stride) in [
        ("ctb-v1", "v1.ctb", ctb::MAGIC_CTB, 1, 36),
        ("ctb", "v2.ctb", ctb::MAGIC_CTB, 2, 36),
        ("ctb-v3", "v3.ctb", ctb::MAGIC_CTB, 3, 84),
        ("ctb-v4", "v4.ctb", ctb::MAGIC_CTB_V4, 4, 84),
        ("ctb-v5", "v5.ctb", ctb::MAGIC_CTB_V4, 5, 84),
        (
            "gktwo-ctb-v1",
            "gktwo-v1.ctb",
            ctb::MAGIC_CTB_V4_GKTWO,
            1,
            36,
        ),
        (
            "gktwo-ctb-v2",
            "gktwo-v2.ctb",
            ctb::MAGIC_CTB_V4_GKTWO,
            2,
            36,
        ),
        (
            "gktwo-ctb-v3",
            "gktwo-v3.ctb",
            ctb::MAGIC_CTB_V4_GKTWO,
            3,
            84,
        ),
        ("gktwo-ctb", "gktwo.ctb", ctb::MAGIC_CTB_V4_GKTWO, 4, 84),
        (
            "gktwo-ctb-v5",
            "gktwo-v5.ctb",
            ctb::MAGIC_CTB_V4_GKTWO,
            5,
            84,
        ),
        ("cbddlp", "cbddlp", ctb::MAGIC_CBDDLP, 2, 36),
        ("photon", "photon", ctb::MAGIC_CBDDLP, 2, 36),
    ] {
        let spec = registry.spec_by_name(name).expect("chitubox export spec");
        let specifics = chitubox_specifics(spec);
        let header = spec
            .header
            .build_header(&metadata, spec, ExportTargetOptions::default())
            .expect("build header");

        assert!(
            spec.capabilities.can_export,
            "{name} should be export-capable"
        );
        assert!(
            spec.catalog.target_names.contains(&preferred_target),
            "{name} should expose UVTools-compatible target suffix {preferred_target}"
        );
        assert!(
            spec.matches_name(preferred_target),
            "{name} should resolve the target suffix {preferred_target}"
        );
        assert_eq!(
            specifics.ctb_version,
            Some(expected_version),
            "{name} version"
        );
        assert_eq!(
            specifics.layer_table_entry_size,
            Some(expected_stride),
            "{name} layer table stride"
        );
        assert_eq!(le_u32(&header, 0), expected_magic, "{name} magic");
        assert_eq!(le_u32(&header, 4), expected_version, "{name} version");
    }
}

#[test]
fn chitubox_family_exposes_special_profiles_with_current_import_export_capabilities() {
    let registry = formats::global_registry();
    let cases = [
        ("phz", keys::HEADER_CHITU_PHZ, keys::PHZ_XOR, "phz", true),
        ("fdg", keys::HEADER_CHITU_FDG, keys::FDG_XOR, "fdg", true),
        (
            "photons",
            keys::HEADER_CHITU_PHOTONS,
            keys::NO_CRYPTO,
            "photons",
            true,
        ),
        (
            "encrypted-ctb-v4",
            keys::HEADER_CHITU_CTB_ENCRYPTED,
            keys::CHITU_AES_BLOCK,
            "encrypted-v4.ctb",
            true,
        ),
        (
            "encrypted-ctb",
            keys::HEADER_CHITU_CTB_ENCRYPTED,
            keys::CHITU_AES_BLOCK,
            "encrypted.ctb",
            true,
        ),
    ];

    for (name, header_key, crypto_key, extension, can_import) in cases {
        let spec = registry.spec_by_name(name).expect("special Chitubox spec");
        assert_eq!(spec.family, FormatFamily::Chitubox);
        assert_eq!(spec.keys.header_key, header_key);
        assert_eq!(spec.keys.crypto_key, crypto_key);
        assert!(
            spec.identity.extensions.contains(&extension),
            "{name} should expose extension {extension}"
        );
        assert_eq!(
            spec.capabilities.can_import, can_import,
            "{name} import capability should match current implementation state"
        );
        assert!(spec.capabilities.can_export, "{name} should export");
        assert!(
            spec.capabilities.supports_encryption || crypto_key == keys::NO_CRYPTO,
            "{name} should advertise encryption when it has a crypto policy"
        );
    }
}

#[test]
fn all_uvtools_chitubox_binary_profiles_are_export_capable() {
    let registry = formats::global_registry();

    for name in ["phz", "fdg", "photons", "encrypted-ctb-v4", "encrypted-ctb"] {
        let spec = registry.spec_by_name(name).expect("Chitubox spec");
        assert!(spec.capabilities.can_import, "{name} should import");
        assert!(spec.capabilities.can_export, "{name} should export");
    }
}

#[test]
fn chitubox_profiles_publish_catalog_default_export_extensions() {
    let registry = formats::global_registry();

    for (name, extension) in [
        ("ctb-v1", "ctb"),
        ("ctb", "ctb"),
        ("ctb-v3", "ctb"),
        ("ctb-v4", "ctb"),
        ("ctb-v5", "ctb"),
        ("cbddlp-v1", "cbddlp"),
        ("cbddlp", "cbddlp"),
        ("gktwo-ctb-v1", "gktwo.ctb"),
        ("gktwo-ctb-v2", "gktwo.ctb"),
        ("gktwo-ctb-v3", "gktwo.ctb"),
        ("gktwo-ctb", "gktwo.ctb"),
        ("gktwo-ctb-v5", "gktwo.ctb"),
        ("encrypted-ctb-v4", "encrypted.ctb"),
        ("encrypted-ctb", "encrypted.ctb"),
    ] {
        let spec = registry.spec_by_name(name).expect("Chitubox spec");
        assert_eq!(
            spec.catalog.default_extension, extension,
            "{name} should publish its catalog default extension"
        );
    }
}

#[test]
fn chitubox_special_profiles_use_active_profile_keys() {
    let registry = formats::global_registry();

    for name in ["phz", "fdg", "photons", "encrypted-ctb-v4", "encrypted-ctb"] {
        let spec = registry.spec_by_name(name).expect("special Chitubox spec");

        assert!(
            !spec.catalog.profile_id.ends_with("read-only"),
            "{name} should advertise an active profile id"
        );
        assert!(
            !spec.keys.profile_key.0.ends_with("read-only"),
            "{name} should use an active profile key, got {}",
            spec.keys.profile_key.0
        );
        assert!(
            !spec.keys.profile_key.0.contains("registry-only"),
            "{name} profile key should not keep stale registry-only naming"
        );
    }
}

#[test]
fn fdg_pipeline_parses_generated_fixture_and_decodes_gray_rle() {
    let fixture = generated_fdg_fixture();
    let fmt = formats::get_format_spec_by_name("fdg").expect("FDG format");

    let (metadata, layers) = parse_document(
        fmt,
        &fixture,
        "FDG should parse generated read-only fixture",
    );

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.printer.display_width_mm, 68.04);
    assert_eq!(metadata.printer.display_height_mm, 120.96);
    assert_eq!(metadata.printer.machine_name, "Mock FDG");
    assert!(metadata.printer.mirror_x);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(metadata.slicing.bottom_layer_count, 1);
    assert_eq!(metadata.slicing.anti_alias_level, 1);
    assert_eq!(metadata.exposure.exposure_time, 2.4);
    assert_eq!(metadata.exposure.bottom_exposure_time, 30.0);
    assert_eq!(metadata.exposure.light_off_delay, 0.8);
    assert_eq!(metadata.exposure.bottom_light_off_delay, 0.4);
    assert_eq!(metadata.exposure.light_pwm, 200);
    assert_eq!(metadata.exposure.bottom_light_pwm, 210);
    assert_eq!(metadata.motion.lift_height, 7.0);
    assert_eq!(metadata.motion.lift_speed, 70.0);
    assert_eq!(metadata.motion.bottom_lift_height, 6.0);
    assert_eq!(metadata.motion.bottom_lift_speed, 60.0);
    assert_eq!(metadata.motion.retract_speed, 150.0);
    assert_eq!(metadata.stats.volume_ml, 1.25);
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, 3);
    assert_eq!(layers[1].size, 3);

    let first_slice = &fixture[layers[0].offset as usize..][..layers[0].size as usize];
    let first_pixels = fmt
        .decode_layer(first_slice, &LayerDecodeContext::new(0, 2, 2))
        .expect("FDG white layer should decode");
    assert_eq!(first_pixels, vec![255, 255, 255, 255]);

    let second_slice = &fixture[layers[1].offset as usize..][..layers[1].size as usize];
    let second_pixels = fmt
        .decode_layer(second_slice, &LayerDecodeContext::new(1, 2, 2))
        .expect("FDG black layer should decode");
    assert_eq!(second_pixels, vec![0, 0, 0, 0]);
}

#[test]
fn fdg_pipeline_encodes_parseable_document() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];

    let (metadata, decoded) = assert_binary_roundtrip("fdg", &source_layers);

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(decoded, source_layers);
}

#[test]
fn fdg_header_parse_does_not_require_layer_payload_bytes() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];
    let fmt = formats::get_format_spec_by_name("fdg").expect("FDG format");
    let metadata = minimal_chitubox_export_metadata();
    let encoded_layers = encode_layers(fmt, &metadata, &source_layers);
    let document = encode_binary_document(fmt, &metadata, &encoded_layers);
    let layer_table_end = le_u32(&document, 52) as usize + 36 * source_layers.len();

    let (_, layers) = fmt
        .header
        .parse_header(&document[..layer_table_end], fmt)
        .expect("FDG header prefix");

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, encoded_layers[0].len() as u64);
    assert_eq!(layers[1].size, encoded_layers[1].len() as u64);
}

#[test]
fn fdg_export_places_machine_name_before_layer_table_for_uvtools_decode() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];
    let fmt = formats::get_format_spec_by_name("fdg").expect("FDG format");
    let metadata = minimal_chitubox_export_metadata();
    let encoded_layers = encode_layers(fmt, &metadata, &source_layers);
    let document = encode_binary_document(fmt, &metadata, &encoded_layers);

    let machine_name_offset = le_u32(&document, 116) as usize;
    let machine_name_size = le_u32(&document, 120) as usize;
    let layers_offset = le_u32(&document, 52) as usize;
    let layer_count = le_u32(&document, 8) as usize;
    let first_payload_offset = le_u32(&document, layers_offset + 12) as usize;

    assert_eq!(machine_name_offset, 216);
    assert_eq!(le_f32(&document, 80), 68.04);
    assert_eq!(machine_name_offset + machine_name_size, layers_offset);
    assert_eq!(first_payload_offset, layers_offset + 36 * layer_count);
}

#[test]
fn phz_pipeline_parses_generated_fixture_and_decodes_gray_rle() {
    let fixture = generated_phz_fixture();
    let fmt = formats::get_format_spec_by_name("phz").expect("PHZ format");

    let (metadata, layers) = parse_document(
        fmt,
        &fixture,
        "PHZ should parse generated read-only fixture",
    );

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.printer.display_width_mm, 68.04);
    assert_eq!(metadata.printer.display_height_mm, 120.96);
    assert_eq!(metadata.printer.machine_name, "Mock PHZ");
    assert!(metadata.printer.mirror_x);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(metadata.slicing.bottom_layer_count, 1);
    assert_eq!(metadata.slicing.anti_alias_level, 1);
    assert_eq!(metadata.exposure.exposure_time, 2.4);
    assert_eq!(metadata.exposure.bottom_exposure_time, 30.0);
    assert_eq!(metadata.exposure.light_pwm, 200);
    assert_eq!(metadata.exposure.bottom_light_pwm, 210);
    assert_eq!(metadata.motion.lift_height, 7.0);
    assert_eq!(metadata.motion.lift_speed, 70.0);
    assert_eq!(metadata.motion.bottom_lift_height, 6.0);
    assert_eq!(metadata.motion.bottom_lift_speed, 60.0);
    assert_eq!(metadata.motion.retract_speed, 150.0);
    assert_eq!(metadata.stats.volume_ml, 1.25);
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, 3);
    assert_eq!(layers[1].size, 3);

    let first_slice = &fixture[layers[0].offset as usize..][..layers[0].size as usize];
    let first_pixels = fmt
        .decode_layer(first_slice, &LayerDecodeContext::new(0, 2, 2))
        .expect("PHZ white layer should decode");
    assert_eq!(first_pixels, vec![255, 255, 255, 255]);

    let second_slice = &fixture[layers[1].offset as usize..][..layers[1].size as usize];
    let second_pixels = fmt
        .decode_layer(second_slice, &LayerDecodeContext::new(1, 2, 2))
        .expect("PHZ black layer should decode");
    assert_eq!(second_pixels, vec![0, 0, 0, 0]);
}

#[test]
fn phz_pipeline_encodes_parseable_document() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];

    let (metadata, decoded) = assert_binary_roundtrip("phz", &source_layers);

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(decoded, source_layers);
}

#[test]
fn phz_header_parse_does_not_require_layer_payload_bytes() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];
    let fmt = formats::get_format_spec_by_name("phz").expect("PHZ format");
    let metadata = minimal_chitubox_export_metadata();
    let encoded_layers = encode_layers(fmt, &metadata, &source_layers);
    let document = encode_binary_document(fmt, &metadata, &encoded_layers);
    let layer_table_end = le_u32(&document, 36) as usize + 36 * source_layers.len();

    let (_, layers) = fmt
        .header
        .parse_header(&document[..layer_table_end], fmt)
        .expect("PHZ header prefix");

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, encoded_layers[0].len() as u64);
    assert_eq!(layers[1].size, encoded_layers[1].len() as u64);
}

#[test]
fn phz_export_places_machine_name_before_layer_table_for_uvtools_decode() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];
    let fmt = formats::get_format_spec_by_name("phz").expect("PHZ format");
    let metadata = minimal_chitubox_export_metadata();
    let encoded_layers = encode_layers(fmt, &metadata, &source_layers);
    let document = encode_binary_document(fmt, &metadata, &encoded_layers);

    let machine_name_offset = le_u32(&document, 144) as usize;
    let machine_name_size = le_u32(&document, 148) as usize;
    let layers_offset = le_u32(&document, 36) as usize;
    let layer_count = le_u32(&document, 40) as usize;
    let first_payload_offset = le_u32(&document, layers_offset + 12) as usize;

    assert_eq!(machine_name_offset, 216);
    assert_eq!(machine_name_offset + machine_name_size, layers_offset);
    assert_eq!(first_payload_offset, layers_offset + 36 * layer_count);
}

#[test]
fn photons_pipeline_parses_generated_fixture_and_decodes_binary_rle() {
    let fixture = generated_photons_fixture();
    let fmt = formats::get_format_spec_by_name("photons").expect("PhotonS format");

    let (metadata, layers) = parse_document(
        fmt,
        &fixture,
        "PhotonS should parse generated read-only fixture",
    );

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.printer.display_width_mm, 68.04);
    assert_eq!(metadata.printer.display_height_mm, 120.96);
    assert_eq!(metadata.printer.machine_name, "Anycubic Photon S");
    assert!(metadata.printer.mirror_x);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(metadata.slicing.bottom_layer_count, 1);
    assert_eq!(metadata.exposure.exposure_time, 2.4);
    assert_eq!(metadata.exposure.bottom_exposure_time, 30.0);
    assert_eq!(metadata.exposure.light_off_delay, 0.8);
    assert_eq!(metadata.motion.lift_height, 5.0);
    assert_eq!(metadata.motion.lift_speed, 72.0);
    assert_eq!(metadata.motion.retract_speed, 138.0);
    assert_eq!(metadata.stats.volume_ml, 1.25);
    assert_eq!(metadata.thumbnails.len(), 1);
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, 29);
    assert_eq!(layers[1].size, 29);

    let first_slice = &fixture[layers[0].offset as usize..][..layers[0].size as usize];
    let first_pixels = fmt
        .decode_layer(first_slice, &LayerDecodeContext::new(0, 2, 2))
        .expect("PhotonS white layer should decode");
    assert_eq!(first_pixels, vec![255, 255, 255, 255]);

    let second_slice = &fixture[layers[1].offset as usize..][..layers[1].size as usize];
    let second_pixels = fmt
        .decode_layer(second_slice, &LayerDecodeContext::new(1, 2, 2))
        .expect("PhotonS black layer should decode");
    assert_eq!(second_pixels, vec![0, 0, 0, 0]);
}

#[test]
fn photons_pipeline_encodes_parseable_document() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];

    let (metadata, decoded) = assert_binary_roundtrip("photons", &source_layers);

    assert_eq!(metadata.printer.resolution_x, 1440);
    assert_eq!(metadata.printer.resolution_y, 2560);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(decoded[0].len(), 1440 * 2560);
    assert_eq!(decoded[1].len(), 1440 * 2560);
    assert!(decoded[0].iter().all(|&pixel| pixel == 255));
    assert!(decoded[1].iter().all(|&pixel| pixel == 0));
}

#[test]
fn photons_export_writes_uvtools_compatible_preview_envelope() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];
    let fmt = formats::get_format_spec_by_name("photons").expect("PhotonS format");
    let metadata = minimal_chitubox_export_metadata();
    let encoded_layers = encode_layers(fmt, &metadata, &source_layers);
    let document = encode_binary_document(fmt, &metadata, &encoded_layers);

    let layer_count_offset = 98 + 224 * 168 * 2;
    let first_layer_offset = layer_count_offset + 4;

    assert_eq!(be_f64(&document, 6), 0.04725);
    assert_eq!(be_u32(&document, 82), 224);
    assert_eq!(be_u32(&document, 86), 42);
    assert_eq!(be_u32(&document, 90), 168);
    assert_eq!(be_u32(&document, 94), 10);
    assert_eq!(be_u32(&document, layer_count_offset), 2);
    assert_eq!(be_u32(&document, first_layer_offset), 44944);
    assert_eq!(be_u32(&document, first_layer_offset + 24), 2_684_702_720);
}

#[test]
fn photons_export_normalizes_layers_to_photon_s_resolution_for_uvtools_ratio_check() {
    let fmt = formats::get_format_spec_by_name("photons").expect("PhotonS format");
    let metadata = FileMetadataBuilder::new()
        .resolution(8520, 4320)
        .display_size_mm(218.112, 122.803)
        .machine_z_mm(165.0)
        .machine_name("Real wide source")
        .mirror(true, false)
        .layer_height_mm(0.05)
        .layer_count(1)
        .bottom_layer_count(1)
        .anti_alias_level(1)
        .exposure(2.4, 30.0)
        .light_off_delay(0.8, 0.4)
        .lift(7.0, 70.0)
        .retract(150.0)
        .statistics(42, 1.25, 1.4, 0.33)
        .build()
        .expect("valid metadata");
    let pixels =
        vec![255u8; (metadata.printer.resolution_x * metadata.printer.resolution_y) as usize];
    let encoded_layers = encode_layers(fmt, &metadata, &[pixels]);
    let document = encode_binary_document(fmt, &metadata, &encoded_layers);

    let layer_count_offset = 98 + 224 * 168 * 2;
    let first_layer_offset = layer_count_offset + 4;
    let (parsed_metadata, _) = parse_document(fmt, &document, "parse normalized photons export");

    assert_eq!(be_u32(&document, first_layer_offset + 12), 1440);
    assert_eq!(be_u32(&document, first_layer_offset + 16), 2560);
    assert_eq!(parsed_metadata.printer.resolution_x, 1440);
    assert_eq!(parsed_metadata.printer.resolution_y, 2560);
    assert_eq!(parsed_metadata.printer.display_width_mm, 68.04);
    assert_eq!(parsed_metadata.printer.display_height_mm, 120.96);
}

#[test]
fn encrypted_ctb_pipeline_parses_aes_settings_and_layer_pointers() {
    let fixture = generated_encrypted_ctb_fixture();
    let fmt = formats::get_format_spec_by_name("encrypted-ctb").expect("encrypted ctb format");

    let (metadata, layers) =
        parse_document(fmt, &fixture, "encrypted CTB should parse AES settings");

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.printer.machine_name, "Mock Encrypted");
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(metadata.slicing.transition_layer_count, 1);
    assert_eq!(metadata.exposure.light_pwm, 200);
    assert_eq!(metadata.exposure.bottom_light_pwm, 210);
    assert_eq!(metadata.stats.print_time_sec, 42);
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, 178);
    assert_eq!(layers[1].size, 92);

    let first_slice = &fixture[layers[0].offset as usize..][..layers[0].size as usize];
    let pixels = fmt
        .decode_layer(first_slice, &LayerDecodeContext::new(0, 2, 2))
        .expect("plain encrypted-ctb layer payload should reuse CTB RLE decode");
    assert_eq!(pixels, vec![255; 4]);
}

#[test]
fn encrypted_ctb_pipeline_encodes_parseable_v5_document() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];

    let (metadata, decoded) = assert_binary_roundtrip("encrypted-ctb", &source_layers);

    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(metadata.printer.resolution_y, 2);
    assert_eq!(metadata.slicing.layer_count, 2);
    assert_eq!(decoded, source_layers);
}

#[test]
fn encrypted_ctb_profiles_export_v4_and_v5_documents() {
    let source_layers = [vec![255, 255, 255, 255], vec![0, 0, 0, 0]];
    let cases = [
        ("encrypted-ctb-v4", 4, "encrypted-v4.ctb"),
        ("encrypted-ctb", 5, "encrypted.ctb"),
        ("encrypted-ctb-v5", 5, "encrypted-v5.ctb"),
    ];

    for (name, expected_version, preferred_target) in cases {
        let spec = formats::get_format_spec_by_name(name).expect("encrypted CTB version spec");
        assert!(spec.capabilities.can_export, "{name} should export");
        assert_eq!(
            chitubox_specifics(spec).ctb_version,
            Some(expected_version),
            "{name} ctb version"
        );
        assert!(
            spec.catalog.target_names.contains(&preferred_target),
            "{name} should expose UVTools-compatible target {preferred_target}"
        );

        let metadata = minimal_chitubox_export_metadata();
        let encoded_layers = source_layers
            .iter()
            .enumerate()
            .map(|(index, pixels)| {
                spec.encode_layer(
                    pixels,
                    &LayerEncodeContext {
                        layer_index: index as u32,
                        width: metadata.printer.resolution_x,
                        height: metadata.printer.resolution_y,
                        layer_metadata: Some(metadata.as_layer_encode_metadata()),
                    },
                )
                .expect("layer encode")
            })
            .collect::<Vec<_>>();
        let output = encode_binary_document(spec, &metadata, &encoded_layers);

        assert_eq!(le_u32(&output, 0), ENCRYPTED_CTB_MAGIC);
        assert_eq!(le_u32(&output, 16), expected_version);

        let (parsed_metadata, layers) = parse_document(spec, &output, "parse encoded document");
        assert_eq!(layers.len(), source_layers.len());
        assert_eq!(
            parsed_metadata.slicing.layer_count,
            source_layers.len() as u32
        );
    }
}

#[test]
fn chitubox_special_profile_magic_detection_does_not_reuse_plain_ctb_specs() {
    let registry = formats::global_registry();
    let cases = [
        (generated_phz_fixture(), "phz"),
        (generated_fdg_fixture(), "fdg"),
        (generated_photons_fixture(), "photons"),
        (generated_encrypted_ctb_fixture(), "encrypted-ctb"),
    ];

    for (fixture, name) in cases {
        let spec = registry
            .detect_spec(&fixture, None)
            .expect("special Chitubox profile");
        assert_eq!(spec.identity.name, name);
        assert_ne!(spec.keys.header_key, keys::HEADER_CHITU_CTB_V2);
        assert_ne!(spec.keys.crypto_key, keys::CHITU_XOR);
    }
}

#[test]
fn chitubox_family_exposes_gktwo_as_ctb_v4_profile() {
    let registry = formats::global_registry();
    let gktwo = registry.spec_by_name("gktwo-ctb").expect("gktwo spec");

    assert_eq!(gktwo.identity.name, "gktwo-ctb");
    assert_eq!(gktwo.catalog.profile_id, "chitubox-gktwo-ctb-v4");
    assert_eq!(chitubox_specifics(gktwo).ctb_version, Some(4));
    assert_eq!(chitubox_specifics(gktwo).machine_profile, Some("gktwo"));
    assert_eq!(gktwo.keys.header_key, keys::HEADER_CHITU_CTB_V4);
    assert_eq!(gktwo.keys.layer_codec_key, keys::CTB_RLE);
}

#[test]
fn chitubox_family_exposes_gktwo_version_profiles() {
    let registry = formats::global_registry();

    for (name, version, header_key, stride) in [
        ("gktwo-ctb-v1", 1, keys::HEADER_CHITU_CTB_V1, 36),
        ("gktwo-ctb-v2", 2, keys::HEADER_CHITU_CTB_V2, 36),
        ("gktwo-ctb-v3", 3, keys::HEADER_CHITU_CTB_V3, 84),
        ("gktwo-ctb", 4, keys::HEADER_CHITU_CTB_V4, 84),
        ("gktwo-ctb-v5", 5, keys::HEADER_CHITU_CTB_V5, 84),
    ] {
        let spec = registry.spec_by_name(name).expect("gktwo version spec");
        let specifics = chitubox_specifics(spec);
        assert_eq!(specifics.kind, ChituboxProfileKind::GktwoCtb);
        assert_eq!(specifics.machine_profile, Some("gktwo"));
        assert_eq!(specifics.ctb_version, Some(version));
        assert_eq!(specifics.build_version, Some(version));
        assert_eq!(specifics.allowed_versions, &[version]);
        assert_eq!(specifics.layer_table_entry_size, Some(stride));
        assert_eq!(spec.keys.header_key, header_key);
        assert_eq!(spec.keys.layer_codec_key, keys::CTB_RLE);
    }
}

#[test]
fn chitubox_profiles_publish_concrete_strategy_keys() {
    let family = formats::global_registry()
        .families()
        .iter()
        .copied()
        .find(|family| family.identity.name == "chitubox")
        .expect("chitubox family");

    for spec in family.profiles {
        assert_eq!(spec.keys.family_key, keys::FAMILY_CHITUBOX);
        assert_eq!(spec.keys.container_key, spec.container.as_key());
        match spec.container {
            ContainerKind::Binary => {
                assert!(
                    spec.keys.header_key.0.starts_with("header/chitu-"),
                    "{} should use a Chitubox binary header strategy key",
                    spec.identity.name
                );
                assert!(
                    spec.keys.layer_codec_key.0.starts_with("layer/")
                        && spec.keys.layer_codec_key != keys::GENERIC_LAYER_FALLBACK,
                    "{} should use a concrete binary layer codec key",
                    spec.identity.name
                );
                assert_eq!(
                    spec.keys.footer_key,
                    keys::NO_FOOTER,
                    "{} should use the explicit no-footer policy",
                    spec.identity.name
                );
            }
            ContainerKind::Archive => {
                assert_eq!(spec.keys.header_key, keys::HEADER_CHITUBOX_ZIP);
                assert_eq!(spec.keys.layer_codec_key, keys::PNG_GRAYSCALE);
                assert_eq!(spec.keys.archive_key, keys::ZIP_ARCHIVE);
            }
        }
    }
}

#[test]
fn chitubox_ctb_profiles_use_explicit_xor_crypto_policy() {
    let registry = formats::global_registry();
    let seed = 0x1234_5678;
    let layer_index = 7;
    let plain = vec![0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];

    for name in ["ctb", "gktwo-ctb"] {
        let spec = registry.spec_by_name(name).expect("chitubox ctb spec");
        assert_eq!(
            spec.keys.crypto_key,
            keys::CHITU_XOR,
            "{name} should advertise the Chitubox XOR crypto policy"
        );

        let decode_ctx = LayerDecodeContext::new(layer_index, 2, 4)
            .with_options(LayerOptions::new().with_encryption_key(seed));
        let mut encrypted = plain.clone();
        ctb::crypto::crypt_layer_data(&mut encrypted, seed, layer_index);

        spec.crypto
            .decrypt_layer_in_place(&mut encrypted, layer_index, &decode_ctx)
            .expect("decrypt layer");
        assert_eq!(encrypted, plain, "{name} decrypt should restore plaintext");

        let metadata = FileMetadata::builder()
            .encryption_key(seed)
            .build()
            .expect("metadata");
        let layer_metadata = metadata.as_layer_encode_metadata();
        let encode_ctx = LayerEncodeContext {
            layer_index,
            width: 2,
            height: 4,
            layer_metadata: Some(layer_metadata),
        };
        let mut encoded = plain.clone();
        spec.crypto
            .encrypt_layer_in_place(&mut encoded, layer_index, &encode_ctx)
            .expect("encrypt layer");
        assert_ne!(encoded, plain, "{name} encrypt should mutate plaintext");
        ctb::crypto::crypt_layer_data(&mut encoded, seed, layer_index);
        assert_eq!(encoded, plain, "{name} encrypt should use CTB XOR");
    }
}

#[test]
fn chitubox_profile_detection_uses_magic_before_extension_aliases() {
    let cbddlp_header = chitubox_header_bytes(ctb::MAGIC_CBDDLP, 2);
    let spec = formats::global_registry()
        .detect_spec(&cbddlp_header, None)
        .expect("cbddlp spec");

    assert_eq!(spec.identity.name, "cbddlp");

    let ctb_header = chitubox_header_bytes(ctb::MAGIC_CTB, 2);
    let spec = formats::global_registry()
        .detect_spec(&ctb_header, Some("cbddlp"))
        .expect("ctb spec");

    assert_eq!(spec.identity.name, "ctb");
}

#[test]
fn chitubox_profile_detection_splits_gktwo_magic_from_generic_ctb_v4() {
    let ctb_v2_header = chitubox_header_bytes(ctb::MAGIC_CTB, 2);
    let spec = formats::global_registry()
        .detect_spec(&ctb_v2_header, Some("ctb"))
        .expect("ctb v2 spec");

    assert_eq!(spec.identity.name, "ctb");
    assert_eq!(spec.catalog.profile_id, "chitubox-ctb-v2");
    assert_eq!(chitubox_specifics(spec).ctb_version, Some(2));
    assert_eq!(spec.keys.header_key, keys::HEADER_CHITU_CTB_V2);

    let gktwo_header = chitubox_header_bytes(ctb::MAGIC_CTB_V4_GKTWO, 4);
    let spec = formats::global_registry()
        .detect_spec(&gktwo_header, Some("ctb"))
        .expect("gktwo spec");

    assert_eq!(spec.identity.name, "gktwo-ctb");

    let ctb_v4_header = chitubox_header_bytes(ctb::MAGIC_CTB_V4, 4);
    let spec = formats::global_registry()
        .detect_spec(&ctb_v4_header, Some("gktwo.ctb"))
        .expect("ctb v4 spec");

    assert_eq!(spec.identity.name, "ctb-v4");
    assert_eq!(spec.catalog.profile_id, "chitubox-ctb-v4");
    assert_eq!(chitubox_specifics(spec).ctb_version, Some(4));
    assert_eq!(spec.keys.header_key, keys::HEADER_CHITU_CTB_V4);

    for (version, expected_name) in [
        (1, "gktwo-ctb-v1"),
        (2, "gktwo-ctb-v2"),
        (3, "gktwo-ctb-v3"),
        (5, "gktwo-ctb-v5"),
    ] {
        let header = chitubox_header_bytes(ctb::MAGIC_CTB_V4_GKTWO, version);
        let spec = formats::global_registry()
            .detect_spec(&header, Some("gktwo.ctb"))
            .expect("gktwo versioned spec");
        assert_eq!(spec.identity.name, expected_name);
    }
}

#[test]
fn chitubox_profile_detection_splits_ctb_versions_with_shared_magic() {
    let registry = formats::global_registry();

    for (name, magic, version, header_key) in [
        ("ctb-v3", ctb::MAGIC_CTB, 3, keys::HEADER_CHITU_CTB_V3),
        ("ctb-v5", ctb::MAGIC_CTB_V4, 5, keys::HEADER_CHITU_CTB_V5),
    ] {
        let spec = registry.spec_by_name(name).expect("versioned ctb spec");
        assert_eq!(spec.identity.name, name);
        assert_eq!(chitubox_specifics(spec).ctb_version, Some(version));
        assert_eq!(spec.keys.header_key, header_key);

        let header = chitubox_header_bytes(magic, version);
        let detected = registry
            .detect_spec(&header, Some("ctb"))
            .expect("versioned ctb detected");
        assert_eq!(detected.identity.name, name);
    }
}

#[test]
fn chitubox_ctb_v4_detects_legacy_magic_version_4() {
    let mut header = [0u8; 8];
    header[0..4].copy_from_slice(&ctb::MAGIC_CTB.to_le_bytes());
    header[4..8].copy_from_slice(&4u32.to_le_bytes());

    let detected = formats::detect_format_spec(&header, Some("ctb"))
        .expect("legacy-magic CTB v4 should be detected");
    assert_eq!(detected.identity.name, "ctb-v4");
}

#[test]
fn chitubox_header_layout_rejects_headers_outside_profile() {
    let registry = formats::global_registry();
    let ctb_v2 = registry.spec_by_name("ctb-v2").expect("ctb v2 spec");
    let ctb_v4 = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");

    let mut v4_header = ctb::build_header_bytes(&metadata).expect("ctb header");
    v4_header[0..4].copy_from_slice(&ctb::MAGIC_CTB_V4.to_le_bytes());
    let err = ctb_v2
        .header
        .parse_header(&v4_header, ctb_v2)
        .expect_err("ctb v2 layout should reject ctb v4 magic");
    assert!(matches!(
        err,
        ConvertError::InvalidMagic {
            expected,
            actual,
        } if expected == ctb::MAGIC_CTB && actual == ctb::MAGIC_CTB_V4
    ));

    let v2_header = ctb::build_header_bytes(&metadata).expect("ctb header");
    let err = ctb_v4
        .header
        .parse_header(&v2_header, ctb_v4)
        .expect_err("ctb v4 layout should reject ctb v2 version");
    assert!(matches!(err, ConvertError::UnsupportedVersion(2)));
}

#[test]
fn chitubox_header_build_uses_profile_magic_and_version() {
    let registry = formats::global_registry();
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");

    for (name, expected_magic, expected_version) in [
        ("ctb-v2", ctb::MAGIC_CTB, 2),
        ("ctb-v3", ctb::MAGIC_CTB, 3),
        ("ctb-v4", ctb::MAGIC_CTB_V4, 4),
        ("ctb-v5", ctb::MAGIC_CTB_V4, 5),
        ("gktwo-ctb-v1", ctb::MAGIC_CTB_V4_GKTWO, 1),
        ("gktwo-ctb-v2", ctb::MAGIC_CTB_V4_GKTWO, 2),
        ("gktwo-ctb-v3", ctb::MAGIC_CTB_V4_GKTWO, 3),
        ("gktwo-ctb", ctb::MAGIC_CTB_V4_GKTWO, 4),
        ("gktwo-ctb-v5", ctb::MAGIC_CTB_V4_GKTWO, 5),
        ("cbddlp", ctb::MAGIC_CBDDLP, 2),
    ] {
        let spec = registry.spec_by_name(name).expect("chitubox spec");
        let header = spec
            .header
            .build_header(&metadata, spec, ExportTargetOptions::default())
            .expect("build header");
        let actual_magic = u32::from_le_bytes(header[0..4].try_into().expect("magic bytes"));
        let actual_version = u32::from_le_bytes(header[4..8].try_into().expect("version bytes"));

        assert_eq!(actual_magic, expected_magic, "{name} magic should match");
        assert_eq!(
            actual_version, expected_version,
            "{name} version should match"
        );
    }
}

#[test]
fn chitubox_ctb_v4_layout_preserves_profile_identity_on_finalize() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let finalized = spec
        .header
        .finalize_header(&header, &[3, 5], spec, ExportTargetOptions::default())
        .expect("finalize");

    assert_eq!(
        u32::from_le_bytes(finalized[0..4].try_into().expect("magic")),
        ctb::MAGIC_CTB_V4
    );
    assert_eq!(
        u32::from_le_bytes(finalized[4..8].try_into().expect("version")),
        4
    );
}

#[test]
fn chitubox_ctb_v4_layout_uses_extended_layer_table_stride() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let finalized = spec
        .header
        .finalize_header(&header, &[3, 5], spec, ExportTargetOptions::default())
        .expect("finalize");
    let layer_table_offset = u32::from_le_bytes(
        finalized[0x40..0x44]
            .try_into()
            .expect("layer table offset"),
    ) as usize;
    let first_table_size = u32::from_le_bytes(
        finalized[layer_table_offset + 24..layer_table_offset + 28]
            .try_into()
            .expect("first table size"),
    );
    let second_table_size = u32::from_le_bytes(
        finalized[layer_table_offset + 36 + 24..layer_table_offset + 36 + 28]
            .try_into()
            .expect("second table size"),
    );
    let first_data_address = u32::from_le_bytes(
        finalized[layer_table_offset + 12..layer_table_offset + 16]
            .try_into()
            .expect("first data address"),
    );
    let second_data_address = u32::from_le_bytes(
        finalized[layer_table_offset + 36 + 12..layer_table_offset + 36 + 16]
            .try_into()
            .expect("second data address"),
    );

    assert_eq!(first_table_size, 84);
    assert_eq!(second_table_size, 84);
    assert_eq!(first_data_address, layer_table_offset as u32 + 36 * 2 + 84);
    assert_eq!(second_data_address, first_data_address + 3 + 84);
}

#[test]
fn chitubox_ctb_v1_v2_v3_layout_blocks_follow_uvtools_version_rules() {
    let registry = formats::global_registry();

    for (
        name,
        version,
        expected_print_params,
        expected_slicer,
        expected_per_layer,
        expected_padding4,
    ) in [
        ("ctb-v1", 1, false, false, 0, 0),
        ("ctb-v2", 2, true, true, 0x20, 0x1234),
        ("ctb-v3", 3, true, true, 0x30, 0),
    ] {
        let spec = registry.spec_by_name(name).expect("ctb version spec");
        let metadata = FileMetadata::builder()
            .resolution(4, 4)
            .display_size_mm(1.0, 1.0)
            .machine_name(format!("Mock {name}"))
            .layer_height_mm(0.05)
            .layer_count(1)
            .anti_alias_level(4)
            .build()
            .expect("metadata");

        let header = spec
            .header
            .build_header(&metadata, spec, ExportTargetOptions::default())
            .expect("header");
        assert_eq!(le_u32(&header, 4), version, "{name} version");

        let print_params_offset = le_u32(&header, 0x54) as usize;
        let print_params_size = le_u32(&header, 0x58);
        let slicer_offset = le_u32(&header, 0x68) as usize;
        let slicer_size = le_u32(&header, 0x6C);
        let layer_table_offset = le_u32(&header, 0x40) as usize;

        if expected_print_params {
            assert!(print_params_offset >= 128, "{name} writes PrintParameters");
            assert_eq!(print_params_size, 60, "{name} PrintParameters size");
            assert_eq!(
                le_u32(&header, print_params_offset + 56),
                expected_padding4,
                "{name} PrintParameters Padding4 marker"
            );
        } else {
            assert_eq!(print_params_offset, 0, "{name} omits PrintParameters");
            assert_eq!(print_params_size, 0, "{name} PrintParameters size");
        }

        if expected_slicer {
            assert_eq!(
                slicer_offset,
                print_params_offset + print_params_size as usize,
                "{name} writes SlicerInfo after PrintParameters"
            );
            assert_eq!(slicer_size, 76, "{name} SlicerInfo fixed size");
            assert_eq!(
                header[slicer_offset + 39],
                expected_per_layer,
                "{name} SlicerInfo per-layer marker"
            );
            assert_eq!(
                le_u32(&header, slicer_offset + 64),
                0,
                "{name} does not reference PrintParametersV4"
            );
            assert_eq!(
                layer_table_offset,
                slicer_offset + slicer_size as usize + format!("Mock {name}").len(),
                "{name} layer table follows SlicerInfo and machine name"
            );
        } else {
            assert_eq!(slicer_offset, 0, "{name} omits SlicerInfo");
            assert_eq!(slicer_size, 0, "{name} SlicerInfo size");
            assert_eq!(layer_table_offset, 128, "{name} layer table follows header");
        }
    }
}

#[test]
fn chitubox_gktwo_v1_v2_v3_layout_blocks_follow_uvtools_version_rules() {
    let registry = formats::global_registry();

    for (
        name,
        version,
        expected_print_params,
        expected_slicer,
        expected_per_layer,
        expected_padding4,
    ) in [
        ("gktwo-ctb-v1", 1, false, false, 0, 0),
        ("gktwo-ctb-v2", 2, true, true, 0x20, 0x1234),
        ("gktwo-ctb-v3", 3, true, true, 0x30, 0),
    ] {
        let spec = registry.spec_by_name(name).expect("gktwo version spec");
        let metadata = FileMetadata::builder()
            .resolution(4, 4)
            .display_size_mm(1.0, 1.0)
            .machine_name(format!("Mock {name}"))
            .layer_height_mm(0.05)
            .layer_count(1)
            .anti_alias_level(4)
            .build()
            .expect("metadata");

        let header = spec
            .header
            .build_header(&metadata, spec, ExportTargetOptions::default())
            .expect("header");
        assert_eq!(le_u32(&header, 0), ctb::MAGIC_CTB_V4_GKTWO);
        assert_eq!(le_u32(&header, 4), version, "{name} version");

        let print_params_offset = le_u32(&header, 0x54) as usize;
        let print_params_size = le_u32(&header, 0x58);
        let slicer_offset = le_u32(&header, 0x68) as usize;
        let slicer_size = le_u32(&header, 0x6C);
        let layer_table_offset = le_u32(&header, 0x40) as usize;

        if expected_print_params {
            assert!(print_params_offset >= 128, "{name} writes PrintParameters");
            assert_eq!(print_params_size, 60, "{name} PrintParameters size");
            assert_eq!(
                le_u32(&header, print_params_offset + 56),
                expected_padding4,
                "{name} PrintParameters Padding4 marker"
            );
        } else {
            assert_eq!(print_params_offset, 0, "{name} omits PrintParameters");
            assert_eq!(print_params_size, 0, "{name} PrintParameters size");
        }

        if expected_slicer {
            assert_eq!(
                slicer_offset,
                print_params_offset + print_params_size as usize,
                "{name} writes SlicerInfo after PrintParameters"
            );
            assert_eq!(slicer_size, 76, "{name} SlicerInfo fixed size");
            assert_eq!(
                header[slicer_offset + 39],
                expected_per_layer,
                "{name} SlicerInfo per-layer marker"
            );
            assert_eq!(
                le_u32(&header, slicer_offset + 64),
                0,
                "{name} does not reference PrintParametersV4"
            );
            assert_eq!(
                layer_table_offset,
                slicer_offset + slicer_size as usize + format!("Mock {name}").len(),
                "{name} layer table follows SlicerInfo and machine name"
            );
        } else {
            assert_eq!(slicer_offset, 0, "{name} omits SlicerInfo");
            assert_eq!(slicer_size, 0, "{name} SlicerInfo size");
            assert_eq!(layer_table_offset, 128, "{name} layer table follows header");
        }
    }
}

#[test]
fn chitubox_ctb_v2_v3_parser_reads_generated_slicer_info() {
    let registry = formats::global_registry();

    for (name, transition_layers, machine_name) in [
        ("ctb-v2", 1, "Parsed CTB v2"),
        ("ctb-v3", 2, "Parsed CTB v3"),
    ] {
        let spec = registry.spec_by_name(name).expect("ctb version spec");
        let metadata = FileMetadata::builder()
            .resolution(4, 4)
            .display_size_mm(1.0, 1.0)
            .machine_name(machine_name)
            .layer_height_mm(0.05)
            .layer_count(4)
            .bottom_layer_count(1)
            .transition_layer_count(transition_layers)
            .lift2(1.1, 2.2)
            .retract2(3.3, 4.4)
            .build()
            .expect("metadata");

        let header = spec
            .header
            .build_header(&metadata, spec, ExportTargetOptions::default())
            .expect("header");
        let (parsed, _) = spec
            .header
            .parse_header(&header, spec)
            .expect("parse generated header");

        assert_eq!(parsed.printer.machine_name, machine_name);
        assert_eq!(parsed.slicing.transition_layer_count, transition_layers);
        assert_eq!(parsed.motion.lift_height2, 1.1);
        assert_eq!(parsed.motion.lift_speed2, 2.2);
        assert_eq!(parsed.motion.retract_height2, 3.3);
        assert_eq!(parsed.motion.retract_speed2, 4.4);
    }
}

#[test]
fn chitubox_ctb_v3_finalize_writes_layer_def_ex_before_layer_payloads() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v3").expect("ctb v3 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .lift(6.0, 60.0)
        .lift2(1.0, 10.0)
        .retract(150.0)
        .retract2(2.0, 20.0)
        .wait_times(1.25, 1.75, 2.5)
        .light_pwm(200, 210)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let finalized = spec
        .header
        .finalize_header(&header, &[3, 5], spec, ExportTargetOptions::default())
        .expect("finalize");

    let layer_table_offset = le_u32(&finalized, 0x40) as usize;
    let layer_table_size = 36 * 2;
    let first_ex_offset = layer_table_offset + layer_table_size;
    let first_data_address = le_u32(&finalized, layer_table_offset + 12) as usize;
    let second_ex_offset = first_data_address + 3;
    let second_data_address = le_u32(&finalized, layer_table_offset + 36 + 12) as usize;

    assert_eq!(le_u32(&finalized, layer_table_offset + 24), 84);
    assert_eq!(le_u32(&finalized, layer_table_offset + 36 + 24), 84);
    assert_eq!(
        first_data_address,
        first_ex_offset + 84,
        "first payload follows its LayerDefEx block"
    );
    assert_eq!(
        second_data_address,
        second_ex_offset + 84,
        "second payload follows its LayerDefEx block"
    );

    assert_eq!(le_u32(&finalized, first_ex_offset + 36), 87);
    assert_eq!(le_u32(&finalized, second_ex_offset + 36), 89);
    assert_eq!(
        f32::from_le_bytes(
            finalized[first_ex_offset + 40..first_ex_offset + 44]
                .try_into()
                .expect("lift height bytes"),
        ),
        6.0
    );
    assert_eq!(
        f32::from_le_bytes(
            finalized[first_ex_offset + 80..first_ex_offset + 84]
                .try_into()
                .expect("light pwm bytes"),
        ),
        210.0
    );
}

#[test]
fn chitubox_ctb_v4_finalize_preserves_moved_layer_def_ex_metadata() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .exposure(2.4, 30.0)
        .lift(6.0, 60.0)
        .lift2(1.0, 10.0)
        .retract(150.0)
        .retract2(2.0, 20.0)
        .wait_times(1.25, 1.75, 2.5)
        .light_pwm(200, 210)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let finalized = spec
        .header
        .finalize_header(&header, &[3, 5], spec, ExportTargetOptions::default())
        .expect("finalize");

    let layer_table_offset = le_u32(&finalized, 0x40) as usize;
    let first_data_address = le_u32(&finalized, layer_table_offset + 12) as usize;
    let second_data_address = le_u32(&finalized, layer_table_offset + 36 + 12) as usize;
    let first_ex_offset = first_data_address - 84;
    let second_ex_offset = second_data_address - 84;

    assert_ne!(
        second_ex_offset,
        layer_table_offset + 36 * 2 + 84,
        "test must move the second LayerDefEx away from its placeholder position"
    );
    assert_eq!(le_f32(&finalized, first_ex_offset), 0.05);
    assert_eq!(le_f32(&finalized, second_ex_offset), 0.10);
    assert_eq!(le_f32(&finalized, first_ex_offset + 4), 30.0);
    assert_eq!(le_f32(&finalized, second_ex_offset + 4), 2.4);
    assert_eq!(
        le_u32(&finalized, first_ex_offset + 12) as usize,
        first_data_address
    );
    assert_eq!(
        le_u32(&finalized, second_ex_offset + 12) as usize,
        second_data_address
    );
}

#[test]
fn chitubox_ctb_v4_document_pipeline_places_layer_payloads_at_finalized_offsets() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(8, 2)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .lift(6.0, 60.0)
        .lift2(1.0, 10.0)
        .retract(150.0)
        .retract2(2.0, 20.0)
        .wait_times(1.25, 1.75, 2.5)
        .light_pwm(200, 210)
        .build()
        .expect("metadata");
    let layer0 = ctb::encode_layer_data(&[0u8; 16], 0, 8, 2).expect("layer 0");
    let layer1 = ctb::encode_layer_data(&[255u8; 16], 1, 8, 2).expect("layer 1");

    let output = encode_binary_document(spec, &metadata, &[layer0.clone(), layer1.clone()]);
    let layer_table_offset = le_u32(&output, 0x40) as usize;
    let first_data_address = le_u32(&output, layer_table_offset + 12) as usize;
    let first_data_size = le_u32(&output, layer_table_offset + 16) as usize;
    let second_data_address = le_u32(&output, layer_table_offset + 36 + 12) as usize;
    let second_data_size = le_u32(&output, layer_table_offset + 36 + 16) as usize;

    assert_eq!(
        &output[first_data_address..first_data_address + first_data_size],
        layer0.as_slice()
    );
    assert_eq!(
        &output[second_data_address..second_data_address + second_data_size],
        layer1.as_slice()
    );
    assert_ctb_rle_fills_exact_pixels(
        &output[first_data_address..first_data_address + first_data_size],
        8,
        2,
        "ctb-v4 layer 0",
    );
    assert_ctb_rle_fills_exact_pixels(
        &output[second_data_address..second_data_address + second_data_size],
        8,
        2,
        "ctb-v4 layer 1",
    );
}

#[test]
fn chitubox_ctb_v4_preview_returns_fallback_when_layer_fits_preview_size() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let width = 4;
    let height = 2;
    let pixels = vec![0, 32, 64, 96, 128, 160, 192, 224];
    let encoded = ctb::encode_layer_data(&pixels, 0, width, height).expect("encoded layer");

    let preview = spec
        .decode_layer_preview_rgba(&encoded, &LayerDecodeContext::new(0, width, height), 16)
        .expect("preview fallback decision");

    assert!(
        preview.is_none(),
        "full-size CTB layer previews should fall back to generic decode"
    );
}

#[test]
fn chitubox_gktwo_v3_finalize_writes_layer_def_ex_before_layer_payloads() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("gktwo-ctb-v3")
        .expect("gktwo ctb v3 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .lift(6.0, 60.0)
        .lift2(1.0, 10.0)
        .retract(150.0)
        .retract2(2.0, 20.0)
        .wait_times(1.25, 1.75, 2.5)
        .light_pwm(200, 210)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let finalized = spec
        .header
        .finalize_header(&header, &[3, 5], spec, ExportTargetOptions::default())
        .expect("finalize");

    let layer_table_offset = le_u32(&finalized, 0x40) as usize;
    let first_ex_offset = layer_table_offset + 36 * 2;
    let first_data_address = le_u32(&finalized, layer_table_offset + 12) as usize;
    let second_ex_offset = first_data_address + 3;
    let second_data_address = le_u32(&finalized, layer_table_offset + 36 + 12) as usize;

    assert_eq!(le_u32(&finalized, 0), ctb::MAGIC_CTB_V4_GKTWO);
    assert_eq!(le_u32(&finalized, layer_table_offset + 24), 84);
    assert_eq!(le_u32(&finalized, layer_table_offset + 36 + 24), 84);
    assert_eq!(first_data_address, first_ex_offset + 84);
    assert_eq!(second_data_address, second_ex_offset + 84);
    assert_eq!(le_u32(&finalized, first_ex_offset + 36), 87);
    assert_eq!(le_u32(&finalized, second_ex_offset + 36), 89);
}

#[test]
fn chitubox_ctb_v4_parser_accepts_uvtools_layer_def_ex_blocks_between_layers() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let layer_table_offset = 128usize;
    let layer_count = 2usize;
    let layer_def_size = 36usize;
    let layer_def_ex_size = 84usize;
    let layer0_data_offset = layer_table_offset + layer_def_size * layer_count + layer_def_ex_size;
    let layer0_size = 3usize;
    let layer1_data_offset = layer0_data_offset + layer0_size + layer_def_ex_size;
    let layer1_size = 5usize;
    let mut data = vec![0u8; layer1_data_offset + layer1_size];

    put_u32(&mut data, 0, ctb::MAGIC_CTB_V4);
    put_u32(&mut data, 4, 4);
    put_f32(&mut data, 8, 1.0);
    put_f32(&mut data, 12, 1.0);
    put_f32(&mut data, 16, 1.0);
    put_f32(&mut data, 28, 0.10);
    put_f32(&mut data, 32, 0.05);
    put_f32(&mut data, 36, 2.4);
    put_f32(&mut data, 40, 30.0);
    put_u32(&mut data, 48, 1);
    put_u32(&mut data, 52, 2);
    put_u32(&mut data, 56, 2);
    put_u32(&mut data, 64, layer_table_offset as u32);
    put_u32(&mut data, 68, layer_count as u32);
    put_u32(&mut data, 88, 1);

    write_ctb_layer_def(
        &mut data,
        layer_table_offset,
        0.05,
        30.0,
        layer0_data_offset,
        layer0_size,
        84,
    );
    write_ctb_layer_def(
        &mut data,
        layer_table_offset + layer_def_size,
        0.10,
        2.4,
        layer1_data_offset,
        layer1_size,
        84,
    );

    let (_metadata, layers) = spec
        .header
        .parse_header(&data, spec)
        .expect("ctb v4 uvtools layer table should parse");

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].offset, layer0_data_offset as u64);
    assert_eq!(layers[0].size, layer0_size as u64);
    assert_eq!(layers[1].offset, layer1_data_offset as u64);
    assert_eq!(layers[1].size, layer1_size as u64);
}

#[test]
fn chitubox_ctb_v4_layout_writes_print_parameters_v4_table() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .machine_name("Mock CTB v4")
        .layer_height_mm(0.05)
        .layer_count(2)
        .transition_layer_count(1)
        .wait_times(1.25, 0.0, 2.5)
        .lift2(1.1, 2.2)
        .retract2(3.3, 4.4)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let slicer_offset = le_u32(&header, 0x68) as usize;
    let slicer_size = le_u32(&header, 0x6C) as usize;
    assert!(slicer_offset > 0, "ctb v4 should write SlicerInfo");
    assert_eq!(slicer_size, 76, "SlicerInfo fixed block size");

    let machine_name_address = le_u32(&header, slicer_offset + 28) as usize;
    let machine_name_size = le_u32(&header, slicer_offset + 32) as usize;
    let transition_layers = le_u32(&header, slicer_offset + 60);
    let print_params_v4_address = le_u32(&header, slicer_offset + 64) as usize;
    assert_eq!(machine_name_size, "Mock CTB v4".len());
    assert_eq!(
        &header[machine_name_address..machine_name_address + machine_name_size],
        b"Mock CTB v4"
    );
    assert_eq!(transition_layers, 1);

    let disclaimer_address = le_u32(&header, print_params_v4_address + 72) as usize;
    let disclaimer_length = le_u32(&header, print_params_v4_address + 76);
    let resin_address = le_u32(&header, print_params_v4_address + 80);
    assert_eq!(disclaimer_address, machine_name_address + machine_name_size);
    assert_eq!(disclaimer_length, 320);
    assert_eq!(resin_address, 0, "ctb v4 should not write resin parameters");
    assert_eq!(
        le_u32(&header, 0x40) as usize,
        print_params_v4_address + 464,
        "layer table should follow PrintParametersV4"
    );
}

#[test]
fn chitubox_gktwo_v4_layout_writes_gktwo_disclaimer_table() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("gktwo-ctb")
        .expect("gktwo ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .machine_name("Mock GKtwo v4")
        .layer_height_mm(0.05)
        .layer_count(2)
        .transition_layer_count(1)
        .wait_times(1.25, 0.0, 2.5)
        .lift2(1.1, 2.2)
        .retract2(3.3, 4.4)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let slicer_offset = le_u32(&header, 0x68) as usize;
    let slicer_size = le_u32(&header, 0x6C) as usize;
    assert_eq!(le_u32(&header, 0), ctb::MAGIC_CTB_V4_GKTWO);
    assert!(slicer_offset > 0, "gktwo v4 should write SlicerInfo");
    assert_eq!(slicer_size, 76, "SlicerInfo fixed block size");

    let machine_name_address = le_u32(&header, slicer_offset + 28) as usize;
    let machine_name_size = le_u32(&header, slicer_offset + 32) as usize;
    let print_params_v4_address = le_u32(&header, slicer_offset + 64) as usize;
    assert_eq!(machine_name_size, "Mock GKtwo v4".len());
    assert_eq!(
        &header[machine_name_address..machine_name_address + machine_name_size],
        b"Mock GKtwo v4"
    );

    let disclaimer_address = le_u32(&header, print_params_v4_address + 72) as usize;
    let disclaimer_length = le_u32(&header, print_params_v4_address + 76);
    let resin_address = le_u32(&header, print_params_v4_address + 80);
    assert_eq!(disclaimer_address, machine_name_address + machine_name_size);
    assert_eq!(disclaimer_length, 323);
    assert_eq!(
        &header[disclaimer_address..disclaimer_address + 32],
        b"Layout and record format for the"
    );
    assert_eq!(
        resin_address, 0,
        "gktwo v4 should not write resin parameters"
    );
    assert_eq!(
        le_u32(&header, 0x40) as usize,
        print_params_v4_address + 464,
        "layer table should follow PrintParametersV4"
    );
}

#[test]
fn chitubox_ctb_v5_layout_writes_resin_parameters_table() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v5").expect("ctb v5 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .machine_name("Mock CTB v5")
        .layer_height_mm(0.05)
        .layer_count(2)
        .statistics(0, 1.5, 1.65, 0.0)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let slicer_offset = le_u32(&header, 0x68) as usize;
    let print_params_v4_address = le_u32(&header, slicer_offset + 64) as usize;
    let resin_address = le_u32(&header, print_params_v4_address + 80) as usize;
    assert!(resin_address > print_params_v4_address);
    assert_eq!(resin_address, print_params_v4_address + 464);
    assert_eq!(
        le_u32(&header, resin_address + 8) as usize,
        resin_address + 40,
        "resin machine name should follow the fixed prefix"
    );
    assert_eq!(
        le_u32(&header, resin_address + 28) as usize,
        "Mock CTB v5".len()
    );
    assert_eq!(le_u32(&header, resin_address + 32), 0x3F8C_CCCD); // 1.1f default resin density
    assert!(
        le_u32(&header, 0x40) as usize > resin_address + 40,
        "layer table should follow resin parameters and strings"
    );

    let (parsed, _) = spec
        .header
        .parse_header(&header, spec)
        .expect("parse generated ctb v5 header");
    assert_eq!(parsed.printer.machine_name, "Mock CTB v5");
}

#[test]
fn chitubox_gktwo_v5_layout_writes_gktwo_disclaimer_and_resin_parameters_table() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("gktwo-ctb-v5")
        .expect("gktwo ctb v5 spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .machine_name("Mock GKtwo v5")
        .layer_height_mm(0.05)
        .layer_count(2)
        .statistics(0, 1.5, 1.65, 0.0)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let slicer_offset = le_u32(&header, 0x68) as usize;
    let print_params_v4_address = le_u32(&header, slicer_offset + 64) as usize;
    let disclaimer_length = le_u32(&header, print_params_v4_address + 76);
    let resin_address = le_u32(&header, print_params_v4_address + 80) as usize;

    assert_eq!(le_u32(&header, 0), ctb::MAGIC_CTB_V4_GKTWO);
    assert_eq!(disclaimer_length, 323);
    assert_eq!(resin_address, print_params_v4_address + 464);
    assert_eq!(
        le_u32(&header, resin_address + 8) as usize,
        resin_address + 40,
        "resin machine name should follow the fixed prefix"
    );
    assert_eq!(
        le_u32(&header, resin_address + 28) as usize,
        "Mock GKtwo v5".len()
    );
    assert_eq!(le_u32(&header, resin_address + 32), 0x3F8C_CCCD);
    assert!(
        le_u32(&header, 0x40) as usize > resin_address + 40,
        "layer table should follow resin parameters and strings"
    );

    let (parsed, _) = spec
        .header
        .parse_header(&header, spec)
        .expect("parse generated gktwo ctb v5 header");
    assert_eq!(parsed.printer.machine_name, "Mock GKtwo v5");
}

#[test]
fn chitubox_ctb_v4_layout_parses_slicer_and_print_parameters_v4() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let metadata = FileMetadata::builder()
        .resolution(8, 6)
        .display_size_mm(2.0, 1.5)
        .machine_name("Parsed CTB v4")
        .layer_height_mm(0.05)
        .layer_count(2)
        .transition_layer_count(1)
        .wait_times(1.25, 1.75, 2.5)
        .lift2(1.1, 2.2)
        .retract2(3.3, 4.4)
        .build()
        .expect("metadata");

    let header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("header");
    let (parsed, _) = spec
        .header
        .parse_header(&header, spec)
        .expect("parse generated ctb v4 header");

    assert_eq!(parsed.printer.machine_name, "Parsed CTB v4");
    assert_eq!(parsed.slicing.transition_layer_count, 1);
    assert_eq!(parsed.motion.lift_height2, 1.1);
    assert_eq!(parsed.motion.lift_speed2, 2.2);
    assert_eq!(parsed.motion.retract_height2, 3.3);
    assert_eq!(parsed.motion.retract_speed2, 4.4);
    assert_eq!(parsed.waits.before_cure, 1.25);
    assert_eq!(parsed.waits.after_cure, 1.75);
    assert_eq!(parsed.waits.after_lift, 2.5);
}

#[test]
fn chitubox_export_resolution_targets_concrete_binary_profile() {
    let family = formats::global_registry()
        .families()
        .iter()
        .copied()
        .find(|family| family.identity.name == "chitubox")
        .expect("chitubox family");

    let cbddlp = family
        .resolver
        .resolve_export_profile(
            "cbddlp",
            None,
            ExportTargetOptions::default(),
            family.profiles,
        )
        .expect("cbddlp export profile");
    let photon = family
        .resolver
        .resolve_export_profile(
            "photon",
            None,
            ExportTargetOptions::default(),
            family.profiles,
        )
        .expect("photon export profile");

    assert_eq!(cbddlp.identity.name, "cbddlp");
    assert_eq!(photon.identity.name, "photon");
}

#[test]
fn chitubox_export_resolution_targets_gktwo_profile() {
    let family = formats::global_registry()
        .families()
        .iter()
        .copied()
        .find(|family| family.identity.name == "chitubox")
        .expect("chitubox family");

    let gktwo = family
        .resolver
        .resolve_export_profile(
            "gktwo.ctb",
            None,
            ExportTargetOptions::default(),
            family.profiles,
        )
        .expect("gktwo export profile");

    assert_eq!(gktwo.identity.name, "gktwo-ctb");

    for (target, expected_name) in [
        ("gktwo-v1.ctb", "gktwo-ctb-v1"),
        ("gktwo-v2.ctb", "gktwo-ctb-v2"),
        ("gktwo-v3.ctb", "gktwo-ctb-v3"),
        ("gktwo.ctb", "gktwo-ctb"),
        ("gktwo-v5.ctb", "gktwo-ctb-v5"),
    ] {
        let spec = family
            .resolver
            .resolve_export_profile(
                target,
                None,
                ExportTargetOptions::default(),
                family.profiles,
            )
            .expect("gktwo versioned export profile");
        assert_eq!(spec.identity.name, expected_name);
    }
}
