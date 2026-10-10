use aes::Aes256;
use cbc::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha2::{Digest, Sha256};

use crate::{
    common::{ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset, PerLayerSettings},
    formats::{
        common::{
            binary::{
                put_le_f32, put_le_u16, put_le_u32, put_le_u64, read_le_f32, read_le_u16,
                read_le_u32, read_le_u64, read_range, BinaryWriter,
            },
            ArchiveEncodeContext, DocumentEncodeStats, ExportTargetOptions, FormatPipeline,
            FormatSpec, HeaderLayout, LayerCodec, LayerSource, OutputSink, ParsedDocument,
        },
        ctb, LayerDecodeContext, LayerEncodeContext,
    },
};

type Aes256CbcDec = cbc::Decryptor<Aes256>;
type Aes256CbcEnc = cbc::Encryptor<Aes256>;

const MAGIC: u32 = 0x12FD_0107;
const FILE_HEADER_SIZE: usize = 48;
const SETTINGS_SIZE: usize = 288;
const LAYER_POINTER_SIZE: usize = 16;
const LAYER_DEF_SIZE: usize = 88;
const PAGE_SIZE: u64 = 4_294_967_296;
const DEFAULT_VERSION: u32 = 5;
const DEFAULT_CHECKSUM: u64 = 0x0123_4567_89AB_CDEF;
const AES_KEY: [u8; 32] = [
    0xD0, 0x5B, 0x8E, 0x33, 0x71, 0xDE, 0x3D, 0x1A, 0xE5, 0x4F, 0x22, 0xDD, 0xDF, 0x5B, 0xFD, 0x94,
    0xAB, 0x5D, 0x64, 0x3A, 0x9D, 0x7E, 0xBF, 0xAF, 0x42, 0x03, 0xF3, 0x10, 0xD8, 0x52, 0x2A, 0xEA,
];
const AES_IV: [u8; 16] = [
    0x0F, 0x01, 0x0A, 0x05, 0x05, 0x0B, 0x06, 0x07, 0x08, 0x06, 0x0A, 0x0C, 0x0C, 0x0D, 0x09, 0x0F,
];

pub struct EncryptedCtbHeaderLayout {
    pub version: Option<u32>,
}
pub struct EncryptedCtbLayerCodec;
pub struct EncryptedCtbPipeline;

#[derive(Debug, Clone)]
struct FileHeader {
    magic: u32,
    settings_size: u32,
    settings_offset: u32,
    version: u32,
    signature_size: u32,
    signature_offset: u32,
}

#[derive(Debug, Clone)]
struct Settings {
    checksum: u64,
    layer_pointers_offset: u32,
    display_width: f32,
    display_height: f32,
    machine_z: f32,
    layer_height: f32,
    exposure_time: f32,
    bottom_exposure_time: f32,
    light_off_delay: f32,
    bottom_layer_count: u32,
    resolution_x: u32,
    resolution_y: u32,
    layer_count: u32,
    print_time: u32,
    projector_type: u32,
    bottom_lift_height: f32,
    bottom_lift_speed: f32,
    lift_height: f32,
    lift_speed: f32,
    retract_speed: f32,
    material_ml: f32,
    material_g: f32,
    material_cost: f32,
    bottom_light_off_delay: f32,
    light_pwm: u16,
    bottom_light_pwm: u16,
    layer_xor_key: u32,
    lift_height2: f32,
    lift_speed2: f32,
    retract_height2: f32,
    retract_speed2: f32,
    rest_time_after_lift: f32,
    machine_name_offset: u32,
    machine_name_size: u32,
    anti_alias_level: u32,
    rest_time_after_retract: f32,
    transition_layer_count: u32,
    rest_time_before_lift: f32,
}

#[derive(Debug, Clone)]
struct LayerPointer {
    offset: u32,
    page: u32,
}

#[derive(Debug, Clone)]
struct LayerDef {
    table_size: u32,
    position_z: f32,
    exposure_time: f32,
    data_offset: u32,
    data_page: u32,
    data_len: u32,
    encrypted_offset: u32,
    encrypted_len: u32,
    lift_height: f32,
    lift_speed: f32,
    retract_speed: f32,
    light_pwm: f32,
}

impl HeaderLayout for EncryptedCtbHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        if data.len() < FILE_HEADER_SIZE || !read_le_u32(data, 0).is_ok_and(|magic| magic == MAGIC)
        {
            return false;
        }
        self.version
            .is_none_or(|version| read_le_u32(data, 16).is_ok_and(|actual| actual == version))
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_document_header(data)
    }

    fn build_header(
        &self,
        _metadata: &FileMetadata,
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} header encoding is not available",
            spec.identity.name
        )))
    }

    fn finalize_header(
        &self,
        _header: &[u8],
        _layer_sizes: &[u32],
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} header finalization is not available",
            spec.identity.name
        )))
    }
}

impl LayerCodec for EncryptedCtbLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        let def = parse_layer_def(data)?;
        let payload_start =
            data.len()
                .checked_sub(def.data_len as usize)
                .ok_or(ConvertError::BufferTooSmall {
                    need: def.data_len as usize,
                    got: data.len(),
                })?;
        let payload_end = payload_start.checked_add(def.data_len as usize).ok_or(
            ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            },
        )?;
        if payload_end > data.len() {
            return Err(ConvertError::BufferTooSmall {
                need: payload_end,
                got: data.len(),
            });
        }

        let mut payload = data[payload_start..payload_end].to_vec();
        if def.encrypted_len > 0 {
            let encrypted_start = def.encrypted_offset as usize;
            let encrypted_end = encrypted_start
                .checked_add(def.encrypted_len as usize)
                .ok_or(ConvertError::BufferTooSmall {
                    need: usize::MAX,
                    got: payload.len(),
                })?;
            if encrypted_end > payload.len() {
                return Err(ConvertError::BufferTooSmall {
                    need: encrypted_end,
                    got: payload.len(),
                });
            }
            let decrypted = aes_decrypt_zero_padded(&payload[encrypted_start..encrypted_end])?;
            payload[encrypted_start..encrypted_end]
                .copy_from_slice(&decrypted[..def.encrypted_len as usize]);
        }
        if ctx.options.encryption_key != 0 {
            ctb::crypto::crypt_layer_data(
                &mut payload,
                ctx.options.encryption_key,
                ctx.layer_index,
            );
        }

        ctb::rle::decode_ctb_rle_into(&payload, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        ctb::rle::encode_ctb_rle_into(pixels, output)?;
        let encryption_key = ctx
            .layer_metadata
            .map(|metadata| metadata.slicing.encryption_key)
            .unwrap_or(0);
        if encryption_key != 0 {
            ctb::crypto::crypt_layer_data(output, encryption_key, ctx.layer_index);
        }
        Ok(())
    }
}

impl FormatPipeline for EncryptedCtbPipeline {
    fn parse_document(
        &self,
        spec: &FormatSpec,
        data: &[u8],
    ) -> Result<ParsedDocument, ConvertError> {
        let (metadata, layers) = spec.header.parse_header(data, spec)?;
        Ok(ParsedDocument { metadata, layers })
    }

    fn encode_document_into(
        &self,
        spec: &FormatSpec,
        metadata: &FileMetadata,
        layers: LayerSource<'_>,
        sink: &mut dyn OutputSink,
        _ctx: &ArchiveEncodeContext<'_>,
    ) -> Result<DocumentEncodeStats, ConvertError> {
        let layers = match layers {
            LayerSource::Encoded(layers) => layers,
            LayerSource::Streaming { .. } => {
                return Err(ConvertError::BinrwError(
                    "encrypted CTB export requires encoded layer buffers".to_string(),
                ));
            }
        };
        let version = encrypted_ctb_version(spec);
        let encoded = build_encrypted_ctb_document(metadata, layers, version, spec)?;
        let stats = DocumentEncodeStats {
            bytes_written: encoded.len(),
            layers_written: layers.len(),
        };
        sink.write_all(&encoded)?;
        Ok(stats)
    }
}

fn build_encrypted_ctb_document(
    metadata: &FileMetadata,
    layers: &[Vec<u8>],
    version: u32,
    _spec: &FormatSpec,
) -> Result<Vec<u8>, ConvertError> {
    let layer_count = layers.len();
    let machine_name = metadata.printer.machine_name.as_bytes();
    let settings_offset = FILE_HEADER_SIZE;
    let settings_size = SETTINGS_SIZE;
    let machine_name_offset = settings_offset + settings_size;
    let layer_pointers_offset = machine_name_offset + machine_name.len();
    let first_layer_offset = layer_pointers_offset + layer_count * LAYER_POINTER_SIZE;

    let mut output = vec![0u8; first_layer_offset];
    output[machine_name_offset..machine_name_offset + machine_name.len()]
        .copy_from_slice(machine_name);

    for (index, layer) in layers.iter().enumerate() {
        let layer_def_offset = output.len();
        let layer_data_offset = layer_def_offset + LAYER_DEF_SIZE;
        let position_z = metadata.slicing.layer_height_mm * (index as f32 + 1.0);
        let exposure_time = if index < metadata.slicing.bottom_layer_count as usize {
            metadata.exposure.bottom_exposure_time
        } else {
            metadata.exposure.exposure_time
        };
        let lift_height = if index < metadata.slicing.bottom_layer_count as usize {
            metadata.motion.bottom_lift_height
        } else {
            metadata.motion.lift_height
        };
        let lift_speed = if index < metadata.slicing.bottom_layer_count as usize {
            metadata.motion.bottom_lift_speed
        } else {
            metadata.motion.lift_speed
        };
        let light_pwm = if index < metadata.slicing.bottom_layer_count as usize {
            metadata.exposure.bottom_light_pwm
        } else {
            metadata.exposure.light_pwm
        };

        write_layer_pointer(
            &mut output,
            layer_pointers_offset + index * LAYER_POINTER_SIZE,
            layer_def_offset,
        )?;
        output.resize(layer_data_offset, 0);
        write_layer_def(
            &mut output[layer_def_offset..layer_data_offset],
            LayerDefWrite {
                position_z,
                exposure_time,
                light_off_delay: metadata.exposure.light_off_delay,
                data_offset: layer_data_offset,
                data_len: layer.len(),
                lift_height,
                lift_speed,
                lift_height2: metadata.motion.lift_height2,
                lift_speed2: metadata.motion.lift_speed2,
                retract_speed: metadata.motion.retract_speed,
                retract_height2: metadata.motion.retract_height2,
                retract_speed2: metadata.motion.retract_speed2,
                rest_time_before_lift: metadata.waits.before_cure,
                rest_time_after_lift: metadata.waits.after_lift,
                rest_time_after_retract: metadata.waits.after_cure,
                light_pwm,
            },
        )?;
        output.extend_from_slice(layer);
    }

    if version >= 5 {
        let mut writer = BinaryWriter::new(&mut output);
        writer.write_le_u32(1_109_414_650);
        writer.write_le_u32(0);
    }

    let signature_offset = output.len();
    let signature = encrypted_signature(DEFAULT_CHECKSUM)?;
    output.extend_from_slice(&signature);
    BinaryWriter::new(&mut output).write_le_u32(1_833_054_899);

    let mut settings = vec![0u8; settings_size];
    write_settings(
        &mut settings,
        metadata,
        DEFAULT_CHECKSUM,
        layer_pointers_offset,
        machine_name_offset,
        machine_name.len(),
        layer_count,
        version,
    )?;
    let encrypted_settings = aes_encrypt_zero_padded(&settings)?;
    output[settings_offset..settings_offset + settings_size].copy_from_slice(&encrypted_settings);

    write_file_header(
        &mut output[0..FILE_HEADER_SIZE],
        settings_size,
        settings_offset,
        signature.len(),
        signature_offset,
        version,
    )?;

    Ok(output)
}

fn encrypted_ctb_version(spec: &FormatSpec) -> u32 {
    if spec.catalog.profile_id.ends_with("-v4") || spec.identity.name.ends_with("-v4") {
        4
    } else {
        DEFAULT_VERSION
    }
}

struct LayerDefWrite {
    position_z: f32,
    exposure_time: f32,
    light_off_delay: f32,
    data_offset: usize,
    data_len: usize,
    lift_height: f32,
    lift_speed: f32,
    lift_height2: f32,
    lift_speed2: f32,
    retract_speed: f32,
    retract_height2: f32,
    retract_speed2: f32,
    rest_time_before_lift: f32,
    rest_time_after_lift: f32,
    rest_time_after_retract: f32,
    light_pwm: u16,
}

fn write_file_header(
    data: &mut [u8],
    settings_size: usize,
    settings_offset: usize,
    signature_size: usize,
    signature_offset: usize,
    version: u32,
) -> Result<(), ConvertError> {
    put_le_u32(data, 0, MAGIC)?;
    put_le_u32(data, 4, checked_u32(settings_size, "settings size")?)?;
    put_le_u32(data, 8, checked_u32(settings_offset, "settings offset")?)?;
    put_le_u32(data, 16, version)?;
    put_le_u32(data, 20, checked_u32(signature_size, "signature size")?)?;
    put_le_u32(data, 24, checked_u32(signature_offset, "signature offset")?)?;
    put_le_u16(data, 32, 1)?;
    put_le_u16(data, 34, 1)?;
    Ok(())
}

fn write_settings(
    settings: &mut [u8],
    metadata: &FileMetadata,
    checksum: u64,
    layer_pointers_offset: usize,
    machine_name_offset: usize,
    machine_name_size: usize,
    layer_count: usize,
    version: u32,
) -> Result<(), ConvertError> {
    put_le_u64(settings, 0, checksum)?;
    put_le_u32(
        settings,
        8,
        checked_u32(layer_pointers_offset, "layer pointers offset")?,
    )?;
    put_le_f32(settings, 12, metadata.printer.display_width_mm)?;
    put_le_f32(settings, 16, metadata.printer.display_height_mm)?;
    put_le_f32(settings, 20, metadata.printer.machine_z_mm)?;
    put_le_f32(
        settings,
        32,
        metadata.slicing.layer_height_mm * layer_count as f32,
    )?;
    put_le_f32(settings, 36, metadata.slicing.layer_height_mm)?;
    put_le_f32(settings, 40, metadata.exposure.exposure_time)?;
    put_le_f32(settings, 44, metadata.exposure.bottom_exposure_time)?;
    put_le_f32(settings, 48, metadata.exposure.light_off_delay)?;
    put_le_u32(settings, 52, metadata.slicing.bottom_layer_count)?;
    put_le_u32(settings, 56, metadata.printer.resolution_x)?;
    put_le_u32(settings, 60, metadata.printer.resolution_y)?;
    put_le_u32(settings, 64, checked_u32(layer_count, "layer count")?)?;
    put_le_u32(settings, 76, metadata.stats.print_time_sec)?;
    put_le_u32(settings, 80, u32::from(metadata.printer.mirror_x))?;
    put_le_f32(settings, 84, metadata.motion.bottom_lift_height)?;
    put_le_f32(settings, 88, metadata.motion.bottom_lift_speed)?;
    put_le_f32(settings, 92, metadata.motion.lift_height)?;
    put_le_f32(settings, 96, metadata.motion.lift_speed)?;
    put_le_f32(settings, 100, metadata.motion.retract_speed)?;
    put_le_f32(settings, 104, metadata.stats.volume_ml)?;
    put_le_f32(settings, 108, metadata.stats.weight_g)?;
    put_le_f32(settings, 112, metadata.stats.cost)?;
    put_le_f32(settings, 116, metadata.exposure.bottom_light_off_delay)?;
    put_le_u16(settings, 124, metadata.exposure.light_pwm)?;
    put_le_u16(settings, 126, metadata.exposure.bottom_light_pwm)?;
    put_le_u32(settings, 128, metadata.slicing.encryption_key)?;
    put_le_f32(settings, 132, metadata.motion.lift_height2)?;
    put_le_f32(settings, 136, metadata.motion.lift_speed2)?;
    put_le_f32(settings, 140, metadata.motion.lift_height2)?;
    put_le_f32(settings, 144, metadata.motion.lift_speed2)?;
    put_le_f32(settings, 148, metadata.motion.retract_height2)?;
    put_le_f32(settings, 152, metadata.motion.retract_speed2)?;
    put_le_f32(settings, 156, metadata.waits.after_lift)?;
    put_le_u32(
        settings,
        160,
        checked_u32(machine_name_offset, "machine name offset")?,
    )?;
    put_le_u32(
        settings,
        164,
        checked_u32(machine_name_size, "machine name size")?,
    )?;
    settings[168] = 0x0F;
    settings[171] = 0x40;
    put_le_u32(settings, 176, metadata.slicing.anti_alias_level.max(1))?;
    put_le_f32(settings, 180, metadata.waits.after_cure)?;
    put_le_f32(settings, 184, metadata.waits.before_cure)?;
    put_le_u32(settings, 188, metadata.slicing.transition_layer_count)?;
    put_le_f32(settings, 224, metadata.waits.before_cure)?;
    put_le_u32(settings, 240, version)?;
    put_le_u32(settings, 244, 1)?;
    Ok(())
}

fn write_layer_pointer(
    data: &mut [u8],
    offset: usize,
    layer_def_offset: usize,
) -> Result<(), ConvertError> {
    let page = layer_def_offset as u64 / PAGE_SIZE;
    let relative_offset = layer_def_offset as u64 - PAGE_SIZE * page;
    put_le_u32(
        data,
        offset,
        checked_u32(relative_offset as usize, "layer pointer offset")?,
    )?;
    put_le_u32(
        data,
        offset + 4,
        checked_u32(page as usize, "layer pointer page")?,
    )?;
    put_le_u32(data, offset + 8, LAYER_DEF_SIZE as u32)?;
    Ok(())
}

fn write_layer_def(data: &mut [u8], layer: LayerDefWrite) -> Result<(), ConvertError> {
    let page = layer.data_offset as u64 / PAGE_SIZE;
    let relative_offset = layer.data_offset as u64 - PAGE_SIZE * page;
    put_le_u32(data, 0, LAYER_DEF_SIZE as u32)?;
    put_le_f32(data, 4, layer.position_z)?;
    put_le_f32(data, 8, layer.exposure_time)?;
    put_le_f32(data, 12, layer.light_off_delay)?;
    put_le_u32(
        data,
        16,
        checked_u32(relative_offset as usize, "layer data offset")?,
    )?;
    put_le_u32(data, 20, checked_u32(page as usize, "layer data page")?)?;
    put_le_u32(data, 24, checked_u32(layer.data_len, "layer data length")?)?;
    put_le_f32(data, 40, layer.lift_height)?;
    put_le_f32(data, 44, layer.lift_speed)?;
    put_le_f32(data, 48, layer.lift_height2)?;
    put_le_f32(data, 52, layer.lift_speed2)?;
    put_le_f32(data, 56, layer.retract_speed)?;
    put_le_f32(data, 60, layer.retract_height2)?;
    put_le_f32(data, 64, layer.retract_speed2)?;
    put_le_f32(data, 72, layer.rest_time_before_lift)?;
    put_le_f32(data, 76, layer.rest_time_after_lift)?;
    put_le_f32(data, 80, layer.rest_time_after_retract)?;
    put_le_f32(data, 84, layer.light_pwm as f32)?;
    Ok(())
}

fn encrypted_signature(checksum: u64) -> Result<Vec<u8>, ConvertError> {
    let hash = Sha256::digest(checksum.to_le_bytes());
    aes_encrypt_zero_padded(&hash)
}

fn aes_encrypt_zero_padded(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut padded = data.to_vec();
    let remainder = padded.len() % 16;
    if remainder != 0 {
        padded.resize(padded.len() + (16 - remainder), 0);
    }
    Ok(Aes256CbcEnc::new(&AES_KEY.into(), &AES_IV.into())
        .encrypt_padded_vec_mut::<NoPadding>(&padded))
}

fn checked_u32(value: usize, label: &str) -> Result<u32, ConvertError> {
    u32::try_from(value).map_err(|_| {
        ConvertError::BinrwError(format!(
            "encrypted CTB {label} exceeds 32-bit address space"
        ))
    })
}

fn parse_document_header(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let header = parse_file_header(data)?;
    if header.magic != MAGIC {
        return Err(ConvertError::InvalidMagic {
            expected: MAGIC,
            actual: header.magic,
        });
    }
    if header.version < 4 {
        return Err(ConvertError::UnsupportedVersion(header.version));
    }
    let settings_offset = header.settings_offset as usize;
    let settings_size = header.settings_size as usize;
    if settings_size < SETTINGS_SIZE {
        return Err(ConvertError::BufferTooSmall {
            need: SETTINGS_SIZE,
            got: settings_size,
        });
    }
    let settings_end =
        settings_offset
            .checked_add(settings_size)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
    if settings_end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: settings_end,
            got: data.len(),
        });
    }
    let settings_block = aes_decrypt_zero_padded(&data[settings_offset..settings_end])?;
    let settings = parse_settings(&settings_block)?;
    verify_signature(data, &header, settings.checksum)?;

    let machine_name = read_string(
        data,
        settings.machine_name_offset as usize,
        settings.machine_name_size as usize,
    )
    .unwrap_or_default();

    let mut layer_offsets = Vec::with_capacity(settings.layer_count as usize);
    let mut per_layer_settings = Vec::with_capacity(settings.layer_count as usize);
    for index in 0..settings.layer_count as usize {
        let pointer_offset = settings.layer_pointers_offset as usize + index * LAYER_POINTER_SIZE;
        let pointer = parse_layer_pointer(read_range(data, pointer_offset, LAYER_POINTER_SIZE)?)?;
        let layer_def_abs = absolute_offset(pointer.page, pointer.offset);
        if layer_def_abs > usize::MAX as u64 {
            return Err(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            });
        }
        let layer_def_offset = layer_def_abs as usize;
        let layer_def = parse_layer_def(read_range(data, layer_def_offset, LAYER_DEF_SIZE)?)?;
        if layer_def.table_size < LAYER_DEF_SIZE as u32 {
            return Err(ConvertError::BufferTooSmall {
                need: LAYER_DEF_SIZE,
                got: layer_def.table_size as usize,
            });
        }
        let data_abs = absolute_offset(layer_def.data_page, layer_def.data_offset);
        let record_end = data_abs.checked_add(layer_def.data_len as u64).ok_or(
            ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            },
        )?;
        if record_end > usize::MAX as u64 {
            return Err(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            });
        }
        let record_size = record_end.checked_sub(layer_def_abs).ok_or_else(|| {
            ConvertError::BinrwError(
                "encrypted CTB layer data precedes layer definition".to_string(),
            )
        })?;
        if record_end as usize > data.len() {
            return Err(ConvertError::BufferTooSmall {
                need: record_end as usize,
                got: data.len(),
            });
        }
        layer_offsets.push(LayerOffset {
            offset: layer_def_abs,
            size: record_size,
        });
        per_layer_settings.push(layer_def.to_per_layer_settings());
    }

    let metadata = FileMetadataBuilder::new()
        .resolution(settings.resolution_x, settings.resolution_y)
        .display_size_mm(settings.display_width, settings.display_height)
        .machine_z_mm(settings.machine_z)
        .machine_name(machine_name)
        .mirror(settings.projector_type == 1, false)
        .layer_height_mm(settings.layer_height)
        .layer_count(settings.layer_count)
        .bottom_layer_count(settings.bottom_layer_count)
        .transition_layer_count(settings.transition_layer_count)
        .anti_alias_level(settings.anti_alias_level.max(1))
        .encryption_key(settings.layer_xor_key)
        .exposure(settings.exposure_time, settings.bottom_exposure_time)
        .light_off_delay(settings.light_off_delay, settings.bottom_light_off_delay)
        .light_pwm(settings.light_pwm, settings.bottom_light_pwm)
        .lift(settings.lift_height, settings.lift_speed)
        .lift2(settings.lift_height2, settings.lift_speed2)
        .bottom_lift(settings.bottom_lift_height, settings.bottom_lift_speed)
        .retract(settings.retract_speed)
        .retract2(settings.retract_height2, settings.retract_speed2)
        .wait_times(
            settings.rest_time_after_retract,
            settings.rest_time_before_lift,
            settings.rest_time_after_lift,
        )
        .statistics(
            settings.print_time,
            settings.material_ml,
            settings.material_g,
            settings.material_cost,
        )
        .per_layer_settings(per_layer_settings)
        .build()?;

    Ok((metadata, layer_offsets))
}

fn parse_file_header(data: &[u8]) -> Result<FileHeader, ConvertError> {
    let header = read_range(data, 0, FILE_HEADER_SIZE)?;
    Ok(FileHeader {
        magic: read_le_u32(header, 0).unwrap(),
        settings_size: read_le_u32(header, 4).unwrap(),
        settings_offset: read_le_u32(header, 8).unwrap(),
        version: read_le_u32(header, 16).unwrap(),
        signature_size: read_le_u32(header, 20).unwrap(),
        signature_offset: read_le_u32(header, 24).unwrap(),
    })
}

fn parse_settings(data: &[u8]) -> Result<Settings, ConvertError> {
    let settings = read_range(data, 0, SETTINGS_SIZE)?;
    Ok(Settings {
        checksum: read_le_u64(settings, 0).unwrap(),
        layer_pointers_offset: read_le_u32(settings, 8).unwrap(),
        display_width: read_le_f32(settings, 12).unwrap(),
        display_height: read_le_f32(settings, 16).unwrap(),
        machine_z: read_le_f32(settings, 20).unwrap(),
        layer_height: read_le_f32(settings, 36).unwrap(),
        exposure_time: read_le_f32(settings, 40).unwrap(),
        bottom_exposure_time: read_le_f32(settings, 44).unwrap(),
        light_off_delay: read_le_f32(settings, 48).unwrap(),
        bottom_layer_count: read_le_u32(settings, 52).unwrap(),
        resolution_x: read_le_u32(settings, 56).unwrap(),
        resolution_y: read_le_u32(settings, 60).unwrap(),
        layer_count: read_le_u32(settings, 64).unwrap(),
        print_time: read_le_u32(settings, 76).unwrap(),
        projector_type: read_le_u32(settings, 80).unwrap(),
        bottom_lift_height: read_le_f32(settings, 84).unwrap(),
        bottom_lift_speed: read_le_f32(settings, 88).unwrap(),
        lift_height: read_le_f32(settings, 92).unwrap(),
        lift_speed: read_le_f32(settings, 96).unwrap(),
        retract_speed: read_le_f32(settings, 100).unwrap(),
        material_ml: read_le_f32(settings, 104).unwrap(),
        material_g: read_le_f32(settings, 108).unwrap(),
        material_cost: read_le_f32(settings, 112).unwrap(),
        bottom_light_off_delay: read_le_f32(settings, 116).unwrap(),
        light_pwm: read_le_u16(settings, 124).unwrap(),
        bottom_light_pwm: read_le_u16(settings, 126).unwrap(),
        layer_xor_key: read_le_u32(settings, 128).unwrap(),
        lift_height2: read_le_f32(settings, 140).unwrap(),
        lift_speed2: read_le_f32(settings, 144).unwrap(),
        retract_height2: read_le_f32(settings, 148).unwrap(),
        retract_speed2: read_le_f32(settings, 152).unwrap(),
        rest_time_after_lift: read_le_f32(settings, 156).unwrap(),
        machine_name_offset: read_le_u32(settings, 160).unwrap(),
        machine_name_size: read_le_u32(settings, 164).unwrap(),
        anti_alias_level: read_le_u32(settings, 176).unwrap(),
        rest_time_after_retract: read_le_f32(settings, 180).unwrap(),
        transition_layer_count: read_le_u32(settings, 188).unwrap(),
        rest_time_before_lift: read_le_f32(settings, 224).unwrap(),
    })
}

fn parse_layer_pointer(data: &[u8]) -> Result<LayerPointer, ConvertError> {
    Ok(LayerPointer {
        offset: read_le_u32(data, 0)?,
        page: read_le_u32(data, 4)?,
    })
}

fn parse_layer_def(data: &[u8]) -> Result<LayerDef, ConvertError> {
    let def = read_range(data, 0, LAYER_DEF_SIZE)?;
    Ok(LayerDef {
        table_size: read_le_u32(def, 0).unwrap(),
        position_z: read_le_f32(def, 4).unwrap(),
        exposure_time: read_le_f32(def, 8).unwrap(),
        data_offset: read_le_u32(def, 16).unwrap(),
        data_page: read_le_u32(def, 20).unwrap(),
        data_len: read_le_u32(def, 24).unwrap(),
        encrypted_offset: read_le_u32(def, 32).unwrap(),
        encrypted_len: read_le_u32(def, 36).unwrap(),
        lift_height: read_le_f32(def, 40).unwrap(),
        lift_speed: read_le_f32(def, 44).unwrap(),
        retract_speed: read_le_f32(def, 56).unwrap(),
        light_pwm: read_le_f32(def, 80).unwrap(),
    })
}

impl LayerDef {
    fn to_per_layer_settings(&self) -> PerLayerSettings {
        PerLayerSettings {
            position_z: self.position_z,
            exposure_time: Some(self.exposure_time),
            lift_height: Some(self.lift_height),
            lift_speed: Some(self.lift_speed),
            retract_speed: Some(self.retract_speed),
            light_pwm: Some(self.light_pwm.clamp(0.0, u16::MAX as f32) as u16),
        }
    }
}

fn verify_signature(data: &[u8], header: &FileHeader, checksum: u64) -> Result<(), ConvertError> {
    let signature = read_range(
        data,
        header.signature_offset as usize,
        header.signature_size as usize,
    )?;
    let hash = Sha256::digest(checksum.to_le_bytes());
    let expected = aes_decrypt_zero_padded(signature)?;
    if expected.get(..hash.len()) != Some(hash.as_slice()) {
        return Err(ConvertError::BinrwError(
            "encrypted CTB checksum does not match".to_string(),
        ));
    }
    Ok(())
}

fn aes_decrypt_zero_padded(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut padded = data.to_vec();
    let remainder = padded.len() % 16;
    if remainder != 0 {
        padded.resize(padded.len() + (16 - remainder), 0);
    }
    Aes256CbcDec::new(&AES_KEY.into(), &AES_IV.into())
        .decrypt_padded_vec_mut::<NoPadding>(&padded)
        .map_err(|err| ConvertError::BinrwError(format!("encrypted CTB AES decrypt failed: {err}")))
}

fn absolute_offset(page: u32, offset: u32) -> u64 {
    page as u64 * PAGE_SIZE + offset as u64
}

fn read_string(data: &[u8], offset: usize, len: usize) -> Option<String> {
    if len == 0 || offset.checked_add(len)? > data.len() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&data[offset..offset + len])
            .trim_end_matches('\0')
            .to_string(),
    )
}
