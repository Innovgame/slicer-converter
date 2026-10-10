use js_sys::{Error as JsNativeError, Reflect, Uint8Array};
use serde::Serialize;
use slicer_converter_core::common::{self, ConvertError};
use slicer_converter_core::formats::{
    self,
    common::{ArchiveEncodeContext, ExportTargetOptions, FormatSpec},
    LayerDecodeContext, LayerEncodeContext,
};
use slicer_converter_core::transcode;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Metadata entry descriptor returned to the JavaScript archive assembler.
///
/// The JS side owns ZIP streaming, while Rust owns format-specific metadata
/// bytes and storage/compression policy.
///
/// 返回给 JavaScript archive assembler 的 metadata entry 描述；ZIP 流式写入在 JS
/// 侧完成，格式特定 metadata 字节与压缩策略由 Rust 决定。
#[derive(Serialize)]
struct ArchiveMetadataEntryInfo {
    name: String,
    data: Vec<u8>,
    compression_method: u16,
}

/// Layer entry descriptor returned to the JavaScript archive assembler.
///
/// It keeps archive entry naming centralized in the Rust format layout while
/// allowing layer bytes to be produced and streamed independently.
///
/// 返回给 JavaScript archive assembler 的 layer entry 描述；entry 命名集中在 Rust
/// 格式 layout 中，图层字节仍可独立生成和流式写入。
#[derive(Serialize)]
struct ArchiveLayerEntryInfo {
    index: u32,
    name: String,
    compression_method: u16,
}

/// Normalized parameters for layer decode calls crossing the WASM boundary.
///
/// Grouping them avoids carrying JavaScript-facing argument lists deep into the
/// format adapters.
///
/// WASM 边界传入的图层解码参数；集中后避免把 JavaScript-facing 参数列表扩散到具体格式 adapter。
#[derive(Clone, Copy)]
struct LayerApiParams<'a> {
    format: &'a str,
    layer_index: u32,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
}

/// Normalized parameters for the combined decode+encode path.
///
/// This struct mirrors the public WASM function because transcode needs both
/// source decode options and target encode metadata in one call.
///
/// decode+encode 合并路径的参数集合；转码需要在一次调用中同时持有源解码选项与目标编码 metadata。
struct TranscodeApiParams<'a> {
    source_format: &'a str,
    target_format: &'a str,
    target_version: Option<u32>,
    layer_index: u32,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    layer_metadata_json: Option<&'a str>,
}

/// Normalized parameters for encode-only WASM calls.
///
/// Per-layer metadata stays as JSON until the concrete target format is known.
///
/// encode-only WASM 调用的参数集合；逐层 metadata 在确定目标格式前保持 JSON 形式。
struct EncodeLayerApiParams<'a> {
    target_format: &'a str,
    target_version: Option<u32>,
    layer_index: u32,
    width: u32,
    height: u32,
    layer_metadata_json: Option<&'a str>,
}

/// Build a format-layer decode context from WASM-normalized parameters.
///
/// This is where JS anti-alias/encryption arguments become the Rust
/// `LayerOptions` contract used by concrete codecs.
///
/// 从 WASM-normalized 参数构建 format-layer decode context。JS 传入的 anti-alias/encryption
/// 参数在这里转换为具体 codec 使用的 Rust `LayerOptions` 契约。
fn decode_context_from(params: LayerApiParams<'_>) -> LayerDecodeContext {
    LayerDecodeContext::new(params.layer_index, params.width, params.height).with_options(
        formats::LayerOptions::new()
            .with_anti_alias_level(params.anti_alias_level)
            .with_encryption_key(params.encryption_key),
    )
}

/// Convert a core [`ConvertError`] into a structured JavaScript `Error`.
///
/// 将 core [`ConvertError`] 转为结构化 JavaScript `Error`。
fn js_error_from_convert(error: ConvertError) -> JsValue {
    let payload = error.to_structured();
    js_error_from_payload(
        payload.code,
        payload.message,
        payload.detail,
        payload.retryable,
    )
}

/// Create a non-retryable structured JavaScript `Error` from a local message.
///
/// 根据本地 message 创建 non-retryable structured JavaScript `Error`。
fn js_error_from_message(code: &str, message: impl Into<String>) -> JsValue {
    js_error_from_payload(code, message, None, false)
}

/// Create a structured JavaScript `Error` from serde or other external errors.
///
/// 根据 serde 或其他外部错误创建 structured JavaScript `Error`。
fn js_error_from_external(code: &str, error: impl std::fmt::Display) -> JsValue {
    js_error_from_payload(code, error.to_string(), None, false)
}

/// Look up an import-capable format by name through the global registry.
///
/// Missing formats are mapped to structured JavaScript `Error` values so the TS
/// side sees the same error payload shape as real conversion failures.
///
/// 通过全局 `FormatRegistry` 按名称查找格式实现；未注册时映射为结构化 JS `Error`。
fn lookup_format(name: &str) -> Result<&'static FormatSpec, JsValue> {
    formats::global_registry()
        .spec_by_name(name)
        .ok_or_else(|| js_error_from_convert(ConvertError::UnknownFormat))
}

fn lookup_export_format(
    target: &str,
    metadata: Option<&common::FileMetadata>,
    target_version: Option<u32>,
) -> Result<&'static FormatSpec, JsValue> {
    formats::global_registry()
        .resolve_export_spec(target, metadata, export_options(target_version))
        .map_err(js_error_from_convert)
}

fn export_options(target_version: Option<u32>) -> ExportTargetOptions<'static> {
    match target_version {
        Some(version) => ExportTargetOptions::new().with_format_version(version),
        None => ExportTargetOptions::new(),
    }
}

/// Look up an archive-capable format and reject flat file formats.
///
/// Archive helper exports must fail early here because their JSON descriptors
/// are consumed by the JS ZIP assembler.
///
/// 查找具备 archive 能力的格式，并拒绝普通 flat file 格式；archive helper 的 JSON
/// 描述会被 JS ZIP assembler 使用，因此需要在此处尽早失败。
fn lookup_archive_format(name: &str) -> Result<&'static FormatSpec, JsValue> {
    let fmt = lookup_format(name)?;
    if !fmt.capabilities.is_archive {
        return Err(js_error_from_message(
            "E_UNSUPPORTED_ARCHIVE",
            format!("format '{}' is not an archive format", fmt.identity.name),
        ));
    }
    Ok(fmt)
}

/// Detect file format through the global registry.
///
/// The optional hint is normally a file extension from JavaScript; registry
/// detection may still prefer magic bytes when available.
///
/// 通过全局 registry 检测文件格式。可选 hint 通常是 JavaScript 传入的扩展名；当 magic bytes
/// 可用时 registry detection 仍可优先使用 magic。
fn detect_format_spec(data: &[u8], hint: Option<&str>) -> Result<&'static FormatSpec, JsValue> {
    formats::global_registry()
        .detect_spec(data, hint)
        .ok_or_else(|| js_error_from_convert(ConvertError::UnknownFormat))
}

fn parse_layer_metadata(
    layer_metadata_json: Option<&str>,
) -> Result<Option<common::OwnedLayerEncodeMetadata>, JsValue> {
    layer_metadata_json
        .map(serde_json::from_str::<common::OwnedLayerEncodeMetadata>)
        .transpose()
        .map_err(|e| js_error_from_external("E_LAYER_METADATA_JSON", e))
}

/// Build a layer encode context from JSON-owned metadata.
///
/// The owned metadata lives in the caller frame; the returned context borrows a
/// view only long enough for the immediate encode/transcode call.
///
/// 根据 JSON-owned metadata 构建 layer encode context。owned metadata 存活在调用帧中；
/// 返回的 context 只借用一个 view，用于当前这次 encode/transcode 调用。
fn encode_context_from<'a>(
    layer_index: u32,
    width: u32,
    height: u32,
    owned_metadata: Option<&'a common::OwnedLayerEncodeMetadata>,
) -> LayerEncodeContext<'a> {
    LayerEncodeContext {
        layer_index,
        width,
        height,
        layer_metadata: owned_metadata.map(common::OwnedLayerEncodeMetadata::as_view),
    }
}

/// Copy Rust bytes into a JavaScript-owned [`Uint8Array`].
///
/// The copy is intentional: callers must not observe Rust scratch-buffer
/// lifetimes after a WASM function returns.
///
/// 将 Rust bytes 复制到 JavaScript-owned [`Uint8Array`]。这里有意复制，避免 WASM 函数返回后
/// 调用方依赖 Rust scratch-buffer 生命周期。
fn uint8array_from_slice(bytes: &[u8]) -> Uint8Array {
    let output = Uint8Array::new_with_length(bytes.len() as u32);
    output.copy_from(bytes);
    output
}

/// Decode a raw layer payload into a Rust-owned grayscale pixel buffer.
///
/// This is the testable bytes-returning core behind the `Uint8Array` WASM
/// export; all format errors are translated into structured JavaScript errors.
///
/// 将原始图层 payload 解码为 Rust-owned 灰度像素缓冲区；这是 `Uint8Array`
/// WASM 导出的可测试核心，并会把格式错误转成结构化 JavaScript error。
pub(super) fn decode_layer_bytes(
    data: &[u8],
    layer_index: u32,
    format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
) -> Result<Vec<u8>, JsValue> {
    let params = LayerApiParams {
        format,
        layer_index,
        width,
        height,
        anti_alias_level,
        encryption_key,
    };
    let fmt = lookup_format(params.format)?;
    let ctx = decode_context_from(params);
    fmt.decode_layer(data, &ctx).map_err(js_error_from_convert)
}

/// Decode a raw layer payload and copy the grayscale pixels into `Uint8Array`.
///
/// JavaScript callers receive an owned array so the internal Rust buffer
/// lifetime never leaks across the WASM boundary.
///
/// 解码原始图层 payload 并复制到 `Uint8Array`；JavaScript 调用方获得独立数组，
/// Rust 内部 buffer 生命周期不会泄漏到 WASM 边界外。
pub(super) fn decode_layer_uint8array(
    data: &[u8],
    layer_index: u32,
    format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
) -> Result<Uint8Array, JsValue> {
    decode_layer_bytes(
        data,
        layer_index,
        format,
        width,
        height,
        anti_alias_level,
        encryption_key,
    )
    .map(|bytes| uint8array_from_slice(&bytes))
}

/// Decode a layer into a packed preview buffer.
///
/// The returned `Uint8Array` starts with `width` and `height` as little-endian
/// `u32`, followed by RGBA pixels. Scratch buffers are thread-local so repeated
/// preview calls in the same worker avoid reallocating large images.
///
/// 将图层解码为打包预览 buffer。返回的 `Uint8Array` 前 8 bytes 是 little-endian
/// `u32` width/height，随后是 RGBA pixels。scratch buffer 为 thread-local，
/// 同一个 worker 内重复预览时可避免大图反复分配。
#[allow(clippy::too_many_arguments)]
pub(super) fn decode_layer_preview_uint8array(
    data: &[u8],
    layer_index: u32,
    format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    max_preview_size: u32,
) -> Result<Uint8Array, JsValue> {
    thread_local! {
        static PREVIEW_PIXELS_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static PREVIEW_PACKED_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    if max_preview_size == 0 {
        return Err(js_error_from_message(
            "E_INVALID_ARGUMENT",
            "max_preview_size must be > 0",
        ));
    }

    let params = LayerApiParams {
        format,
        layer_index,
        width,
        height,
        anti_alias_level,
        encryption_key,
    };
    let fmt = lookup_format(params.format)?;
    let ctx = decode_context_from(params);
    PREVIEW_PIXELS_BUF.with(|pixels_buf| {
        let preview = if let Some(fast) = fmt
            .decode_layer_preview_rgba(data, &ctx, max_preview_size)
            .map_err(js_error_from_convert)?
        {
            fast
        } else {
            let mut pixels = pixels_buf.borrow_mut();
            pixels.clear();
            fmt.decode_layer_into(data, &ctx, &mut pixels)
                .map_err(js_error_from_convert)?;
            common::downsample_to_rgba_nearest(&pixels, ctx.width, ctx.height, max_preview_size)
        };

        PREVIEW_PACKED_BUF.with(|packed_buf| {
            let mut packed = packed_buf.borrow_mut();
            packed.clear();
            packed.reserve(8 + preview.pixels.len());
            packed.extend_from_slice(&preview.width.to_le_bytes());
            packed.extend_from_slice(&preview.height.to_le_bytes());
            packed.extend_from_slice(&preview.pixels);
            Ok(uint8array_from_slice(&packed))
        })
    })
}

/// Encode one grayscale layer and hand the scratch bytes to an output adapter.
///
/// This shared helper keeps `Vec<u8>` return paths and `Uint8Array` return paths
/// on the same encode implementation, while preserving the thread-local encode
/// buffer between calls.
///
/// 编码单个 grayscale layer，并把 scratch bytes 交给输出 adapter。该 shared helper 让
/// `Vec<u8>` 返回路径与 `Uint8Array` 返回路径复用同一 encode 实现，同时保留 thread-local
/// encode buffer 以跨调用复用。
#[allow(clippy::too_many_arguments)]
fn encode_layer_with_output<T>(
    pixels: &[u8],
    layer_index: u32,
    target_format: &str,
    width: u32,
    height: u32,
    layer_metadata_json: Option<&str>,
    target_version: Option<u32>,
    output: impl FnOnce(&[u8]) -> T,
) -> Result<T, JsValue> {
    thread_local! {
        static ENCODE_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    let params = EncodeLayerApiParams {
        target_format,
        target_version,
        layer_index,
        width,
        height,
        layer_metadata_json,
    };
    let fmt = lookup_export_format(params.target_format, None, params.target_version)?;
    let owned = parse_layer_metadata(params.layer_metadata_json)?;
    let ctx = encode_context_from(
        params.layer_index,
        params.width,
        params.height,
        owned.as_ref(),
    );
    ENCODE_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        fmt.encode_layer_into(pixels, &ctx, &mut buf)
            .map_err(js_error_from_convert)?;
        Ok(output(&buf))
    })
}

/// Encode grayscale pixels and return a JavaScript-owned `Uint8Array`.
///
/// The underlying encoder writes into a thread-local scratch buffer and copies
/// into the final JS array before returning.
///
/// 编码灰度像素并返回 JavaScript-owned `Uint8Array`；底层 encoder 写入
/// thread-local scratch buffer，返回前复制到最终 JS 数组。
pub(super) fn encode_layer_uint8array(
    pixels: &[u8],
    layer_index: u32,
    target_format: &str,
    width: u32,
    height: u32,
    layer_metadata_json: Option<&str>,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    encode_layer_with_output(
        pixels,
        layer_index,
        target_format,
        width,
        height,
        layer_metadata_json,
        target_version,
        uint8array_from_slice,
    )
}

/// Serialize the Rust format catalog for the TypeScript surface.
///
/// This is the single WASM source for app-visible format capabilities and
/// profile metadata.
///
/// 序列化 Rust 格式目录，作为 TypeScript 侧可见 format capabilities 与 profile
/// metadata 的单一 WASM 来源。
pub(super) fn supported_formats_json() -> Result<String, JsValue> {
    let infos =
        formats::catalog::supported_format_infos(formats::global_registry().catalog_profiles());
    serde_json::to_string(&infos).map_err(|e| js_error_from_external("E_JSON_SERIALIZE", e))
}

/// Parse a file header and serialize the common parse result as JSON.
///
/// Format detection uses magic bytes first with the optional extension hint as
/// a fallback.
///
/// 解析文件头并把通用 parse result 序列化为 JSON；格式检测优先使用 magic bytes，
/// 可选 extension hint 作为补充。
pub(super) fn parse_header_json(
    data: &[u8],
    extension_hint: Option<&str>,
) -> Result<String, JsValue> {
    let fmt = detect_format_spec(data, extension_hint)?;
    let parsed = fmt
        .pipeline
        .parse_document(fmt, data)
        .map_err(js_error_from_convert)?;

    let result = common::ParseResult {
        format: fmt.identity.name.to_string(),
        metadata: parsed.metadata,
        layers: parsed.layers,
    };

    serde_json::to_string(&result).map_err(|e| js_error_from_external("E_JSON_SERIALIZE", e))
}

/// Build archive metadata entries and serialize their descriptors as JSON.
///
/// The descriptors intentionally contain bytes plus ZIP compression method so
/// JS can stream the archive without reimplementing format-specific metadata.
///
/// 构建 archive metadata entries，并将描述序列化为 JSON。描述中包含 bytes 与 ZIP
/// compression method，使 JS 能流式写 archive 而无需重写格式特定 metadata。
pub(super) fn build_archive_metadata_entries_json(
    metadata_json: &str,
    target_format: &str,
    job_name: &str,
    total_layers: u32,
) -> Result<String, JsValue> {
    let metadata: common::FileMetadata = serde_json::from_str(metadata_json)
        .map_err(|e| js_error_from_external("E_METADATA_JSON", e))?;
    let fmt = lookup_archive_format(target_format)?;
    let ctx = ArchiveEncodeContext::new(job_name);
    let entries = fmt
        .archive
        .build_metadata_entries(&ctx, &metadata, total_layers, fmt)
        .map_err(js_error_from_convert)?
        .into_iter()
        .map(|entry| ArchiveMetadataEntryInfo {
            name: entry.name,
            data: entry.bytes,
            compression_method: zip_compression_method(entry.store),
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&entries).map_err(|e| js_error_from_external("E_JSON_SERIALIZE", e))
}

/// Build a target format header as Rust-owned bytes.
///
/// Metadata JSON is parsed before export-format resolution so profile-specific
/// export aliases can use metadata during lookup.
///
/// 构建目标格式 header 字节；先解析 metadata JSON 再解析 export format，使特定
/// profile alias 可在 lookup 时使用 metadata。
pub(super) fn build_header_bytes(
    metadata_json: &str,
    target_format: &str,
    target_version: Option<u32>,
) -> Result<Vec<u8>, JsValue> {
    let metadata: common::FileMetadata = serde_json::from_str(metadata_json)
        .map_err(|e| js_error_from_external("E_METADATA_JSON", e))?;
    let fmt = lookup_export_format(target_format, Some(&metadata), target_version)?;
    fmt.build_header_with_options(&metadata, export_options(target_version))
        .map_err(js_error_from_convert)
}

/// Diagnose non-fatal export issues and serialize them as JSON.
///
/// The concrete warning rules live in core format families; this WASM layer
/// only parses metadata, resolves the target profile, and serializes results.
///
/// 诊断非致命导出问题并序列化为 JSON。具体 warning 规则位于 core format family；
/// WASM 层只负责解析 metadata、解析目标 profile 与序列化结果。
pub(super) fn diagnose_export_json(
    metadata_json: &str,
    target_format: &str,
) -> Result<String, JsValue> {
    let metadata: common::FileMetadata = serde_json::from_str(metadata_json)
        .map_err(|e| js_error_from_external("E_METADATA_JSON", e))?;
    let fmt = lookup_export_format(target_format, Some(&metadata), None)?;
    let report = formats::diagnose_export(&metadata, fmt);

    serde_json::to_string(&report).map_err(|e| js_error_from_external("E_JSON_SERIALIZE", e))
}

/// Build a target format header and copy it into `Uint8Array`.
///
/// 构建目标格式 header，并复制为 `Uint8Array` 返回给 JavaScript。
pub(super) fn build_header_uint8array(
    metadata_json: &str,
    target_format: &str,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    build_header_bytes(metadata_json, target_format, target_version)
        .map(|bytes| uint8array_from_slice(&bytes))
}

/// Build a target format footer as Rust-owned bytes.
///
/// Formats without footers return an empty vector through the same contract.
///
/// 构建目标格式 footer 字节；没有 footer 的格式通过同一契约返回空 `Vec`。
pub(super) fn build_footer_bytes(target_format: &str) -> Result<Vec<u8>, JsValue> {
    let fmt = lookup_export_format(target_format, None, None)?;
    fmt.footer
        .build_footer(&common::FileMetadata::default(), fmt)
        .map_err(js_error_from_convert)
}

/// Build a target format footer and copy it into `Uint8Array`.
///
/// 构建目标格式 footer，并复制为 `Uint8Array` 返回给 JavaScript。
pub(super) fn build_footer_uint8array(target_format: &str) -> Result<Uint8Array, JsValue> {
    build_footer_bytes(target_format).map(|bytes| uint8array_from_slice(&bytes))
}

/// Assemble a complete document from materialized encoded layer payloads.
///
/// This adapter is for formats whose file layout is document-level rather than
/// header/finalize based, such as encrypted CTB.
///
/// 从已 materialized 的 encoded layer payloads 组装完整 document。该 adapter 用于
/// encrypted CTB 这类 document-level layout，而不是 header/finalize layout。
pub(super) fn build_document_from_layers_bytes(
    metadata_json: &str,
    target_format: &str,
    target_version: Option<u32>,
    layers: &[Vec<u8>],
) -> Result<Vec<u8>, JsValue> {
    let metadata: common::FileMetadata = serde_json::from_str(metadata_json)
        .map_err(|e| js_error_from_external("E_METADATA_JSON", e))?;
    let fmt = lookup_export_format(target_format, Some(&metadata), target_version)?;
    let mut sink = slicer_converter_core::formats::common::VecOutputSink::new();
    fmt.pipeline
        .encode_document_into(
            fmt,
            &metadata,
            slicer_converter_core::formats::common::LayerSource::Encoded(layers),
            &mut sink,
            &ArchiveEncodeContext::new("browser")
                .with_export_options(export_options(target_version)),
        )
        .map_err(js_error_from_convert)?;
    Ok(sink.into_inner())
}

/// Assemble a complete document and copy it into `Uint8Array`.
///
/// 组装完整 document，并复制为 `Uint8Array`。
pub(super) fn build_document_from_layers_uint8array(
    metadata_json: &str,
    target_format: &str,
    layers: &[Vec<u8>],
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    build_document_from_layers_bytes(metadata_json, target_format, target_version, layers)
        .map(|bytes| uint8array_from_slice(&bytes))
}

/// Build the Creality CXDLP v4 document header bytes.
///
/// This helper is intentionally format-gated because the CXDLP v4 document
/// layout is not represented by the generic header/finalize contract.
///
/// 构建 Creality CXDLP v4 document header 字节。该 helper 明确限制格式，因为
/// CXDLP v4 document layout 不适合通用 header/finalize 契约。
pub(super) fn build_cxdlpv4_document_header_bytes(
    metadata_json: &str,
    target_format: &str,
    layer_sizes: &[u32],
) -> Result<Vec<u8>, JsValue> {
    if target_format != "cxdlpv4" {
        return Err(js_error_from_message(
            "E_UNSUPPORTED_FORMAT",
            format!("CXDLPv4 document helpers do not support '{target_format}'"),
        ));
    }
    let metadata: common::FileMetadata = serde_json::from_str(metadata_json)
        .map_err(|e| js_error_from_external("E_METADATA_JSON", e))?;
    slicer_converter_core::formats::creality::cxdlp::build_cxdlp_v4_document_header(
        &metadata,
        layer_sizes,
    )
    .map_err(js_error_from_convert)
}

/// Build the Creality CXDLP v4 document header as `Uint8Array`.
///
/// 构建 Creality CXDLP v4 document header，并复制为 `Uint8Array`。
pub(super) fn build_cxdlpv4_document_header_uint8array(
    metadata_json: &str,
    target_format: &str,
    layer_sizes: &[u32],
) -> Result<Uint8Array, JsValue> {
    build_cxdlpv4_document_header_bytes(metadata_json, target_format, layer_sizes)
        .map(|bytes| uint8array_from_slice(&bytes))
}

/// Build one Creality CXDLP v4 layer definition block.
///
/// The caller supplies final data address and size because JavaScript owns the
/// surrounding document assembly order.
///
/// 构建单个 Creality CXDLP v4 layer definition block；调用方提供最终 data address
/// 与 size，因为外围 document 组装顺序由 JavaScript 控制。
pub(super) fn build_cxdlpv4_layer_def_bytes(
    metadata_json: &str,
    layer_index: u32,
    data_address: u32,
    data_size: u32,
) -> Result<Vec<u8>, JsValue> {
    let metadata: common::FileMetadata = serde_json::from_str(metadata_json)
        .map_err(|e| js_error_from_external("E_METADATA_JSON", e))?;
    Ok(
        slicer_converter_core::formats::creality::cxdlp::build_cxdlp_v4_layer_def(
            &metadata,
            layer_index as usize,
            data_address,
            data_size,
        ),
    )
}

/// Build one Creality CXDLP v4 layer definition block as `Uint8Array`.
///
/// 构建单个 Creality CXDLP v4 layer definition block，并复制为 `Uint8Array`。
pub(super) fn build_cxdlpv4_layer_def_uint8array(
    metadata_json: &str,
    layer_index: u32,
    data_address: u32,
    data_size: u32,
) -> Result<Uint8Array, JsValue> {
    build_cxdlpv4_layer_def_bytes(metadata_json, layer_index, data_address, data_size)
        .map(|bytes| uint8array_from_slice(&bytes))
}

/// Compute the Creality CXDLP checksum for a complete byte slice.
///
/// 计算完整 byte slice 的 Creality CXDLP checksum。
pub(super) fn cxdlp_crc32_bytes(data: &[u8]) -> u32 {
    slicer_converter_core::formats::creality::cxdlp::cxdlp_crc32(data)
}

/// Continue the Creality CXDLP checksum with another byte slice.
///
/// 使用另一个 byte slice 继续累加 Creality CXDLP checksum。
pub(super) fn cxdlp_crc32_update_bytes(previous: u32, data: &[u8]) -> u32 {
    slicer_converter_core::formats::creality::cxdlp::cxdlp_crc32_update(previous, data)
}

/// Patch a previously-built header with encoded layer sizes.
///
/// This is the bytes-returning core used by the public WASM finalize helper.
///
/// 使用已编码图层大小回写之前构建的 header；这是 public WASM finalize helper
/// 背后的 bytes-returning 核心。
pub(super) fn finalize_header_bytes(
    header_bytes: &[u8],
    target_format: &str,
    target_version: Option<u32>,
    layer_sizes: &[u32],
) -> Result<Vec<u8>, JsValue> {
    let fmt = lookup_export_format(target_format, None, target_version)?;
    fmt.finalize_header_with_options(
        header_bytes.to_vec(),
        layer_sizes,
        export_options(target_version),
    )
    .map_err(js_error_from_convert)
}

/// Patch a previously-built header and return a `Uint8Array`.
///
/// 回写之前构建的 header，并复制为 `Uint8Array`。
pub(super) fn finalize_header_uint8array(
    header_bytes: &[u8],
    target_format: &str,
    layer_sizes: &[u32],
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    finalize_header_bytes(header_bytes, target_format, target_version, layer_sizes)
        .map(|bytes| uint8array_from_slice(&bytes))
}

/// Merge concatenated CTB/CBDDLP anti-alias bitplanes into grayscale pixels.
///
/// The input contract is flat because it crosses WASM cheaply; this function
/// validates the shape before creating temporary plane slices.
///
/// 将拼接后的 CTB/CBDDLP 抗锯齿 bitplanes 合并为灰度像素。输入使用 flat buffer
/// 以降低 WASM 边界成本；这里先校验形状，再临时切成 plane slices。
pub(super) fn merge_bitplanes_bytes(
    planes_flat: &[u8],
    plane_count: u32,
    pixel_count: u32,
) -> Result<Vec<u8>, JsValue> {
    if plane_count == 0 {
        return Err(js_error_from_message(
            "E_INVALID_ARGUMENT",
            "plane_count must be > 0",
        ));
    }
    let pc = pixel_count as usize;
    let expected = pc * plane_count as usize;
    if planes_flat.len() != expected {
        return Err(js_error_from_message(
            "E_INVALID_ARGUMENT",
            format!(
                "expected {} bytes ({} planes × {} pixels), got {}",
                expected,
                plane_count,
                pixel_count,
                planes_flat.len()
            ),
        ));
    }

    let planes: Vec<&[u8]> = (0..plane_count as usize)
        .map(|i| &planes_flat[i * pc..(i + 1) * pc])
        .collect();

    slicer_converter_core::formats::ctb::merge_bitplane_slices(&planes, 0, 0)
        .map_err(js_error_from_convert)
}

/// Merge anti-alias bitplanes and copy the grayscale pixels into `Uint8Array`.
///
/// 合并抗锯齿 bitplanes，并复制灰度像素为 `Uint8Array`。
pub(super) fn merge_bitplanes_uint8array(
    planes_flat: &[u8],
    plane_count: u32,
    pixel_count: u32,
) -> Result<Uint8Array, JsValue> {
    merge_bitplanes_bytes(planes_flat, plane_count, pixel_count)
        .map(|bytes| uint8array_from_slice(&bytes))
}

/// Describe archive layer entries as JSON.
///
/// The returned names and compression methods let JavaScript stream a layer
/// range without knowing the target archive's internal layout.
///
/// 将 archive layer entries 描述为 JSON；返回的名称与压缩方式让 JavaScript 可按范围
/// 流式写入图层，而无需了解目标 archive 的内部 layout。
pub(super) fn describe_archive_layer_entries_json(
    target_format: &str,
    job_name: &str,
    start_index: u32,
    count: u32,
) -> Result<String, JsValue> {
    let fmt = lookup_archive_format(target_format)?;
    let end_index = start_index.checked_add(count).ok_or_else(|| {
        js_error_from_message(
            "E_INVALID_ARGUMENT",
            "describe_archive_layer_entries: start_index + count overflowed u32",
        )
    })?;
    let ctx = ArchiveEncodeContext::new(job_name);
    let entries = (start_index..end_index)
        .map(|index| {
            let descriptor = fmt.archive.describe_layer_entry(&ctx, index, fmt);
            ArchiveLayerEntryInfo {
                index,
                name: descriptor.name,
                compression_method: zip_compression_method(descriptor.store),
            }
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&entries).map_err(|e| js_error_from_external("E_JSON_SERIALIZE", e))
}

/// Convert the Rust archive storage flag to the ZIP method id expected by JS.
///
/// `0` means stored and `8` means deflated, matching the ZIP specification and
/// the browser-side assembler contract.
///
/// 将 Rust archive storage flag 转为 JS 侧期望的 ZIP method id。`0` 表示 stored，`8`
/// 表示 deflated，对齐 ZIP spec 与 browser-side assembler contract。
fn zip_compression_method(store: bool) -> u16 {
    if store {
        0
    } else {
        8
    }
}

/// Thread-local scratch buffers used by combined decode+encode WASM calls.
///
/// 组合 decode+encode WASM 调用使用的 thread-local scratch buffers。
struct LayerBufferPool<'a> {
    pixel_buf: &'a mut Vec<u8>,
    aux_buf: &'a mut Vec<u8>,
    encode_buf: &'a mut Vec<u8>,
}

/// Borrow per-worker scratch buffers for combined layer transcode calls.
///
/// The buffers are thread-local to avoid reallocating pixel, auxiliary, and
/// encoded-output storage across layer calls inside one Web Worker.
///
/// 借用单个 worker 内的 combined layer transcode scratch buffers；这些 buffer 为
/// thread-local，可避免每层调用都重新分配 pixel、aux 与 encoded output storage。
fn with_layer_buffer_pool<T>(
    f: impl FnOnce(&mut LayerBufferPool<'_>) -> Result<T, JsValue>,
) -> Result<T, JsValue> {
    thread_local! {
        static PIXEL_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static AUX_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static ENCODE_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    PIXEL_BUF.with(|pixel_cell| {
        AUX_BUF.with(|aux_cell| {
            ENCODE_BUF.with(|encode_cell| {
                let mut pixel_buf = pixel_cell.borrow_mut();
                let mut aux_buf = aux_cell.borrow_mut();
                let mut encode_buf = encode_cell.borrow_mut();
                let mut buffers = LayerBufferPool {
                    pixel_buf: &mut pixel_buf,
                    aux_buf: &mut aux_buf,
                    encode_buf: &mut encode_buf,
                };
                f(&mut buffers)
            })
        })
    })
}

/// Execute one combined decode+encode call using an already-borrowed buffer pool.
///
/// Keeping this separate from the public `Uint8Array`/stats wrappers ensures
/// both wrappers use the exact same transcode path.
///
/// 使用已借出的 buffer pool 执行一次组合 decode+encode 调用。该函数与 public
/// `Uint8Array`/stats wrappers 分离，确保两个 wrapper 使用完全相同的 transcode path。
fn decode_encode_layer_with_buffers(
    data: &[u8],
    params: TranscodeApiParams<'_>,
    buffers: &mut LayerBufferPool<'_>,
) -> Result<transcode::LayerTranscodeStats, JsValue> {
    let src_fmt = lookup_format(params.source_format)?;
    let tgt_fmt = lookup_export_format(params.target_format, None, params.target_version)?;
    let decode_ctx = decode_context_from(LayerApiParams {
        format: params.source_format,
        layer_index: params.layer_index,
        width: params.width,
        height: params.height,
        anti_alias_level: params.anti_alias_level,
        encryption_key: params.encryption_key,
    });
    let owned = parse_layer_metadata(params.layer_metadata_json)?;
    let encode_ctx = encode_context_from(
        params.layer_index,
        params.width,
        params.height,
        owned.as_ref(),
    );

    transcode::execute_layer_into_with_stats(
        src_fmt,
        tgt_fmt,
        data,
        &decode_ctx,
        &encode_ctx,
        transcode::LayerExecutionBuffers {
            pixel_buf: buffers.pixel_buf,
            aux_buf: buffers.aux_buf,
            out: buffers.encode_buf,
        },
    )
    .map_err(js_error_from_convert)
}

/// Decode and encode one layer in a single WASM call.
///
/// The returned bytes are copied from the reusable encode buffer into a
/// JavaScript-owned `Uint8Array` before crossing the boundary.
///
/// 在一次 WASM 调用中解码并编码单层；返回前会从可复用 encode buffer 复制到
/// JavaScript-owned `Uint8Array`。
#[allow(clippy::too_many_arguments)]
pub(super) fn decode_encode_layer_uint8array(
    data: &[u8],
    layer_index: u32,
    source_format: &str,
    target_format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    layer_metadata_json: Option<&str>,
    target_version: Option<u32>,
) -> Result<Uint8Array, JsValue> {
    let params = TranscodeApiParams {
        source_format,
        target_format,
        target_version,
        layer_index,
        width,
        height,
        anti_alias_level,
        encryption_key,
        layer_metadata_json,
    };
    with_layer_buffer_pool(|buffers| {
        let _stats = decode_encode_layer_with_buffers(data, params, buffers)?;
        Ok(uint8array_from_slice(buffers.encode_buf))
    })
}

/// Decode and encode one layer, returning route statistics as JSON.
///
/// This uses the same execution path as `decode_encode_layer_uint8array` but
/// serializes stats instead of returning encoded layer bytes.
///
/// 解码并编码单层，返回 route statistics JSON；执行路径与
/// `decode_encode_layer_uint8array` 相同，但序列化统计信息而不是返回图层字节。
#[allow(clippy::too_many_arguments)]
pub(super) fn decode_encode_layer_stats_json(
    data: &[u8],
    layer_index: u32,
    source_format: &str,
    target_format: &str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    layer_metadata_json: Option<&str>,
    target_version: Option<u32>,
) -> Result<String, JsValue> {
    let params = TranscodeApiParams {
        source_format,
        target_format,
        target_version,
        layer_index,
        width,
        height,
        anti_alias_level,
        encryption_key,
        layer_metadata_json,
    };
    let stats: transcode::LayerTranscodeStats =
        with_layer_buffer_pool(|buffers| decode_encode_layer_with_buffers(data, params, buffers))?;
    serde_json::to_string(&stats).map_err(|e| js_error_from_external("E_JSON_SERIALIZE", e))
}

/// Build the structured JavaScript `Error` payload used by every WASM export.
///
/// The TS side relies on `code`, `message`, optional `detail`, and `retryable`
/// fields rather than parsing free-form strings.
///
/// 构建所有 WASM export 共用的 structured JavaScript `Error` payload。TS 侧依赖
/// `code`、`message`、可选 `detail` 与 `retryable` 字段，而不是解析自由文本。
fn js_error_from_payload(
    code: &str,
    message: impl Into<String>,
    detail: Option<String>,
    retryable: bool,
) -> JsValue {
    let message = message.into();
    let error = JsNativeError::new(&message);
    let error_value: JsValue = error.into();

    Reflect::set(
        &error_value,
        &JsValue::from_str("code"),
        &JsValue::from_str(code),
    )
    .expect("setting code on JS error should succeed");
    Reflect::set(
        &error_value,
        &JsValue::from_str("retryable"),
        &JsValue::from_bool(retryable),
    )
    .expect("setting retryable on JS error should succeed");

    if let Some(detail) = detail {
        Reflect::set(
            &error_value,
            &JsValue::from_str("detail"),
            &JsValue::from_str(&detail),
        )
        .expect("setting detail on JS error should succeed");
    }

    error_value
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_converter_core::common::ParseResult;

    fn encode_layer_bytes(
        pixels: &[u8],
        layer_index: u32,
        target_format: &str,
        width: u32,
        height: u32,
        layer_metadata_json: Option<&str>,
    ) -> Result<Vec<u8>, JsValue> {
        encode_layer_with_output(
            pixels,
            layer_index,
            target_format,
            width,
            height,
            layer_metadata_json,
            None,
            |bytes| bytes.to_vec(),
        )
    }

    #[test]
    fn cxdlpv4_document_helpers_build_parseable_prefix_and_checksum() {
        let metadata = common::FileMetadata::builder()
            .machine_name("CL-TEST")
            .resolution(8, 1)
            .display_size_mm(8.0, 1.0)
            .machine_z_mm(100.0)
            .mirror(true, false)
            .layer_height_mm(0.05)
            .layer_count(2)
            .bottom_layer_count(1)
            .transition_layer_count(1)
            .anti_alias_level(1)
            .exposure(2.0, 20.0)
            .light_pwm(120, 240)
            .build()
            .expect("metadata");
        let metadata_json = serde_json::to_string(&metadata).expect("metadata json");
        let first_layer = encode_layer_bytes(
            &[0, 0, 255, 255, 255, 2, 2, 0],
            0,
            "cxdlpv4",
            8,
            1,
            Some(&metadata_json),
        )
        .expect("encode first layer");
        let second_layer = encode_layer_bytes(
            &[255, 255, 0, 0, 0, 0, 4, 4],
            1,
            "cxdlpv4",
            8,
            1,
            Some(&metadata_json),
        )
        .expect("encode second layer");
        let layer_sizes = [first_layer.len() as u32, second_layer.len() as u32];
        let header = build_cxdlpv4_document_header_bytes(&metadata_json, "cxdlpv4", &layer_sizes)
            .expect("build header");
        let first_def = build_cxdlpv4_layer_def_bytes(
            &metadata_json,
            0,
            (header.len() + 80) as u32,
            layer_sizes[0],
        )
        .expect("first layer def");
        let second_def = build_cxdlpv4_layer_def_bytes(
            &metadata_json,
            1,
            (header.len() + 80 + first_layer.len()) as u32,
            layer_sizes[1],
        )
        .expect("second layer def");

        let mut document = Vec::new();
        let mut checksum = 0;
        for chunk in [
            &header,
            &first_def,
            &second_def,
            &first_layer,
            &second_layer,
        ] {
            checksum = cxdlp_crc32_update_bytes(checksum, chunk);
            document.extend_from_slice(chunk);
        }
        document.extend_from_slice(&checksum.to_be_bytes());
        let parsed: ParseResult =
            serde_json::from_str(&parse_header_json(&document, Some("cxdlpv4")).expect("parse"))
                .expect("parse result json");

        assert_eq!(parsed.format, "cxdlpv4");
        assert_eq!(parsed.metadata.printer.machine_name, "CL-TEST");
        assert_eq!(parsed.layers.len(), 2);
        assert_eq!(
            parsed.layers[1].offset + parsed.layers[1].size,
            document.len() as u64 - 4
        );
    }
}
