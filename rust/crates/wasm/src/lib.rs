mod wasm_api;
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

/// Compute the standard CRC32 checksum for a byte slice.
///
/// Exposed for JavaScript callers that need to validate container payloads
/// without reimplementing the checksum outside WASM.
///
/// 计算标准 CRC32 校验值，供 JavaScript 调用方在 WASM 外无需重复实现 checksum。
#[wasm_bindgen]
pub fn crc32_bytes(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Compute the Creality CXDLP CRC32 checksum for a byte slice.
///
/// CXDLP stores the final checksum with format-specific byte order and update
/// semantics; use this helper instead of the generic CRC32 export for CXDLP
/// document assembly.
///
/// 计算 Creality CXDLP 专用 CRC32。CXDLP 的最终 checksum 有格式特定的字节序与
/// update 语义，组装 CXDLP 文档时应使用这个 helper。
#[wasm_bindgen]
pub fn cxdlp_crc32_bytes(data: &[u8]) -> u32 {
    wasm_api::cxdlp_crc32_bytes(data)
}

/// Continue a Creality CXDLP CRC32 checksum with another byte slice.
///
/// This keeps streaming archive assembly on the JavaScript side aligned with
/// the Rust CXDLP parser and writer.
///
/// 继续累加 Creality CXDLP CRC32，用于 JavaScript 侧流式组装 archive 时保持与
/// Rust parser/writer 一致。
#[wasm_bindgen]
pub fn cxdlp_crc32_update(previous: u32, data: &[u8]) -> u32 {
    wasm_api::cxdlp_crc32_update_bytes(previous, data)
}

/// Return the supported format catalog as JSON.
///
/// The JSON is the public TypeScript-facing catalog: format identity,
/// capabilities, and profile metadata are serialized from the Rust registry.
///
/// 返回面向 TypeScript 的格式目录 JSON；内容来自 Rust registry，包含格式 identity、
/// capabilities 与 profile metadata。
#[wasm_bindgen]
pub fn list_supported_formats() -> Result<String, JsValue> {
    wasm_api::supported_formats_json()
}

/// Parse file header, returning JSON with metadata + layer offset table.
/// Automatically detects format by magic bytes or file extension hint.
///
/// 解析切片文件头，自动通过魔数字节或文件扩展名推断格式，
/// 返回包含通用元数据与图层偏移表的 JSON 字符串。
///
/// # Arguments
/// * `data` - Full file bytes / 文件完整字节
/// * `extension_hint` - Optional format extension hint, e.g. `"ctb"` / 可选的格式扩展名提示
#[wasm_bindgen]
pub fn parse_header(data: &[u8], extension_hint: Option<String>) -> Result<String, JsValue> {
    wasm_api::parse_header_json(data, extension_hint.as_deref())
}

/// Decode a single layer's raw bytes (RLE/PNG) into grayscale pixels.
///
/// 将单个图层的原始字节（RLE 或 PNG 编码）解码为灰度像素缓冲区。
///
/// # Arguments
/// * `data` - Raw layer bytes / 原始图层字节
/// * `layer_index` - Zero-based layer index / 从零开始的图层索引
/// * `format` - Source format name (e.g. `"ctb"`) / 源格式名称
/// * `width` / `height` - Printer resolution / 打印机分辨率
/// * `anti_alias_level` - Anti-alias level; 1 = none, >1 = CBDDLP bitplane count / 抗锯齿级别
/// * `encryption_key` - CTB encryption key; 0 = unencrypted / CTB 加密密钥，0 表示未加密
#[wasm_bindgen]
pub fn decode_layer(
    data: &[u8],
    layer_index: u32,
    format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
) -> Result<Uint8Array, JsValue> {
    wasm_api::decode_layer_uint8array(
        data,
        layer_index,
        format,
        width,
        height,
        anti_alias_level,
        encryption_key,
    )
}

/// Decode a single layer and downsample for preview display.
/// Returns a packed byte buffer: [width_u32_le, height_u32_le, rgba_pixels...].
/// JS side should read the first 8 bytes as two u32 LE values for dimensions.
///
/// 解码单个图层并缩放为预览图。返回打包字节：[宽度 u32 LE, 高度 u32 LE, 像素数据...]。
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn decode_layer_preview(
    data: &[u8],
    layer_index: u32,
    format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    max_preview_size: u32,
) -> Result<Uint8Array, JsValue> {
    wasm_api::decode_layer_preview_uint8array(
        data,
        layer_index,
        format,
        width,
        height,
        anti_alias_level,
        encryption_key,
        max_preview_size,
    )
}

/// Encode grayscale pixels into target format's layer data.
///
/// 将灰度像素编码为目标格式的图层数据。
///
/// # Arguments
/// * `pixels` - Grayscale pixel buffer / 灰度像素缓冲区
/// * `layer_index` - Zero-based layer index / 从零开始的图层索引
/// * `target_format` - Target format name / 目标格式名称
/// * `width` / `height` - Printer resolution / 打印机分辨率
/// * `layer_metadata_json` - Optional per-layer metadata JSON (from `buildLayerMetadataJson`) /
///   可选的逐层元数据 JSON（由 `buildLayerMetadataJson` 生成）
/// * `target_version` - Optional Anycubic numeric target version / 可选 Anycubic 数字目标版本
#[wasm_bindgen]
pub fn encode_layer(
    pixels: &[u8],
    layer_index: u32,
    target_format: &str,
    width: u32,
    height: u32,
    layer_metadata_json: Option<String>,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    wasm_api::encode_layer_uint8array(
        pixels,
        layer_index,
        target_format,
        width,
        height,
        layer_metadata_json.as_deref(),
        target_version,
    )
}

/// Build target format file header from metadata JSON.
///
/// 根据元数据 JSON 构建目标格式的文件头字节。
///
/// # Arguments
/// * `metadata_json` - Serialised `FileMetadata` JSON / 序列化的 `FileMetadata` JSON
/// * `target_format` - Target format name / 目标格式名称
/// * `target_version` - Optional Anycubic numeric target version / 可选 Anycubic 数字目标版本
#[wasm_bindgen]
pub fn build_header(
    metadata_json: &str,
    target_format: &str,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    wasm_api::build_header_uint8array(metadata_json, target_format, target_version)
}

/// Diagnose non-fatal export issues for a target format.
///
/// The returned JSON contains a `warnings` array. Conversion may continue when
/// warnings are present; callers should display them to users before or after export.
///
/// 诊断目标格式的非致命导出问题。返回 JSON 包含 `warnings` 数组；存在 warning 时转换仍可继续，
/// 调用方应在导出前后展示给用户。
#[wasm_bindgen]
pub fn diagnose_export(metadata_json: &str, target_format: &str) -> Result<String, JsValue> {
    wasm_api::diagnose_export_json(metadata_json, target_format)
}

/// Build archive metadata entry descriptors as JSON.
///
/// Archive formats such as Chitubox Zip need sidecar config/manifest entries
/// before layer payloads are streamed by the JavaScript ZIP assembler.
///
/// 构建 archive metadata entry 描述 JSON。Chitubox Zip 等 archive 格式在
/// JavaScript ZIP assembler 流式写入图层前，需要先生成 config/manifest 等旁路条目。
#[wasm_bindgen]
pub fn build_archive_metadata_entries(
    metadata_json: &str,
    target_format: &str,
    job_name: &str,
    total_layers: u32,
) -> Result<String, JsValue> {
    wasm_api::build_archive_metadata_entries_json(
        metadata_json,
        target_format,
        job_name,
        total_layers,
    )
}

/// Build a Creality CXDLP v4 document header.
///
/// CXDLP v4 stores layer sizes in the document header, so this helper is called
/// after layer encoding but before final byte assembly.
///
/// 构建 Creality CXDLP v4 document header。CXDLP v4 会把图层大小写入 document
/// header，因此需要在图层编码完成后、最终字节组装前调用。
#[wasm_bindgen]
pub fn build_cxdlpv4_document_header(
    metadata_json: &str,
    target_format: &str,
    layer_sizes: &[u32],
) -> Result<Uint8Array, JsValue> {
    wasm_api::build_cxdlpv4_document_header_uint8array(metadata_json, target_format, layer_sizes)
}

/// Build one Creality CXDLP v4 layer definition block.
///
/// The block points at the already-planned layer data address and size; callers
/// are responsible for matching those offsets to the final assembled document.
///
/// 构建单个 Creality CXDLP v4 layer definition block。该 block 指向已经规划好的图层
/// data address 与 size，调用方需要保证这些 offset 与最终文档布局一致。
#[wasm_bindgen]
pub fn build_cxdlpv4_layer_def(
    metadata_json: &str,
    layer_index: u32,
    data_address: u32,
    data_size: u32,
) -> Result<Uint8Array, JsValue> {
    wasm_api::build_cxdlpv4_layer_def_uint8array(
        metadata_json,
        layer_index,
        data_address,
        data_size,
    )
}

/// Describe archive layer entry names and compression methods as JSON.
///
/// JavaScript ZIP assembly uses this to stream layer payloads without knowing
/// each archive format's naming convention.
///
/// 以 JSON 返回 archive layer entry 的名称与压缩方式；JavaScript ZIP 组装可借此流式写入图层，
/// 不需要了解每种 archive 格式的命名规则。
#[wasm_bindgen]
pub fn describe_archive_layer_entries(
    target_format: &str,
    job_name: &str,
    start_index: u32,
    count: u32,
) -> Result<String, JsValue> {
    wasm_api::describe_archive_layer_entries_json(target_format, job_name, start_index, count)
}

/// Build target format file footer (GOO/PRZ need this, CTB/SL1 return empty).
///
/// 构建目标格式的文件尾（GOO/PRZ 需要此字段，CTB/SL1 返回空字节）。
///
/// # Arguments
/// * `target_format` - Target format name / 目标格式名称
#[wasm_bindgen]
pub fn build_footer(target_format: &str) -> Result<Uint8Array, JsValue> {
    wasm_api::build_footer_uint8array(target_format)
}

/// Build a complete target document from already encoded layer payloads.
///
/// This is intentionally separate from `build_header`/`finalize_header` for
/// formats such as encrypted CTB whose layout is finalized by the document
/// pipeline.
///
/// 从已经编码好的 layer payloads 构建完整目标 document。该函数刻意独立于
/// `build_header`/`finalize_header`，用于 encrypted CTB 这类由 document pipeline
/// 最终确定 layout 的格式。
#[wasm_bindgen]
pub fn build_document_from_layers(
    metadata_json: &str,
    target_format: &str,
    layers: Array,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    let mut layer_bytes = Vec::with_capacity(layers.length() as usize);
    for layer in layers.iter() {
        let layer = Uint8Array::new(&layer);
        layer_bytes.push(layer.to_vec());
    }
    wasm_api::build_document_from_layers_uint8array(
        metadata_json,
        target_format,
        &layer_bytes,
        target_version,
    )
}

/// Merge CBDDLP anti-alias bitplanes into a single grayscale image.
/// `planes_flat` contains all bitplanes concatenated (each plane_count pixels long).
/// Returns a single grayscale pixel buffer of `pixel_count` bytes.
///
/// 将 CBDDLP 抗锯齿位平面合并为单张灰度图像。
/// `planes_flat` 为所有位平面顺序拼接的字节（每个平面 `pixel_count` 字节长），
/// 返回 `pixel_count` 字节的灰度像素缓冲区。
///
/// # Arguments
/// * `planes_flat` - All bitplanes concatenated / 所有位平面顺序拼接的字节
/// * `plane_count` - Number of anti-alias planes / 抗锯齿平面数量
/// * `pixel_count` - Pixels per plane / 每个平面的像素数
#[wasm_bindgen]
pub fn merge_bitplanes(
    planes_flat: &[u8],
    plane_count: u32,
    pixel_count: u32,
) -> Result<Uint8Array, JsValue> {
    wasm_api::merge_bitplanes_uint8array(planes_flat, plane_count, pixel_count)
}

/// Patch the file header with actual layer data offsets/sizes.
/// Call this after encoding all layers and before assembling the final file.
/// `header_bytes` is the output of `build_header`, `layer_sizes` is each layer's encoded byte count.
///
/// 将实际图层数据偏移量/大小回写到文件头中。
/// 需在所有图层编码完成后、组装最终文件前调用。
///
/// # Arguments
/// * `header_bytes` - Output of `build_header` / `build_header` 的返回值
/// * `target_format` - Target format name / 目标格式名称
/// * `layer_sizes` - Each layer's encoded byte count / 每个图层编码后的字节数
/// * `target_version` - Optional Anycubic numeric target version / 可选 Anycubic 数字目标版本
#[wasm_bindgen]
pub fn finalize_header(
    header_bytes: &[u8],
    target_format: &str,
    layer_sizes: &[u32],
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    wasm_api::finalize_header_uint8array(header_bytes, target_format, layer_sizes, target_version)
}

/// Combined decode + encode in a single WASM call.
/// Avoids the JS↔WASM roundtrip for the intermediate pixel buffer and reuses
/// an internal buffer across calls within the same worker instance.
///
/// 在单次 WASM 调用中完成解码+编码，省去中间像素缓冲区的 JS↔WASM 往返开销，
/// 并在同一 Worker 实例的调用间复用内部缓冲区。
///
/// # Arguments
/// * `data` - Raw source layer bytes / 原始源图层字节
/// * `layer_index` - Zero-based layer index / 从零开始的图层索引
/// * `source_format` - Source format name / 源格式名称
/// * `target_format` - Target format name / 目标格式名称
/// * `width` / `height` - Printer resolution / 打印机分辨率
/// * `anti_alias_level` - Anti-alias level / 抗锯齿级别
/// * `encryption_key` - CTB encryption key; 0 = none / CTB 加密密钥，0 表示无加密
/// * `layer_metadata_json` - Optional per-layer metadata JSON / 可选的逐层元数据 JSON
/// * `target_version` - Optional Anycubic numeric target version / 可选 Anycubic 数字目标版本
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn decode_encode_layer(
    data: &[u8],
    layer_index: u32,
    source_format: &str,
    target_format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    layer_metadata_json: Option<String>,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    wasm_api::decode_encode_layer_uint8array(
        data,
        layer_index,
        source_format,
        target_format,
        width,
        height,
        anti_alias_level,
        encryption_key,
        layer_metadata_json.as_deref(),
        target_version,
    )
}

/// Combined decode + encode stats in a single WASM call.
///
/// 返回单层转码的 fast path / fallback 统计信息 JSON，不返回图层字节。
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn decode_encode_layer_stats(
    data: &[u8],
    layer_index: u32,
    source_format: &str,
    target_format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    layer_metadata_json: Option<String>,
    target_version: Option<u32>,
) -> Result<String, JsValue> {
    wasm_api::decode_encode_layer_stats_json(
        data,
        layer_index,
        source_format,
        target_format,
        width,
        height,
        anti_alias_level,
        encryption_key,
        layer_metadata_json.as_deref(),
        target_version,
    )
}
