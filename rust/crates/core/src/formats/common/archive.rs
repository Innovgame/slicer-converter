use std::io::{Cursor, Read};

use serde::Serialize;

use crate::common::{ConvertError, FileMetadata, LayerOffset, Thumbnail};

use super::{ExportTargetOptions, FormatSpec};

/// Directory entry facts extracted from a ZIP archive.
///
/// This is intentionally metadata-only: layer payload bytes are read later by
/// entry index so archive scanners can stay cheap.
///
/// 从 ZIP archive 中提取的 directory entry 事实。它只携带 metadata；layer payload bytes
/// 稍后再按 entry index 读取，从而让 archive 扫描保持轻量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntryInfo {
    /// Entry index in central-directory order.
    ///
    /// central directory 顺序中的 entry index。
    pub index: usize,
    /// UTF-8 archive entry name.
    ///
    /// UTF-8 archive entry 名称。
    pub name: String,
    /// Compressed byte length from the ZIP central directory.
    ///
    /// ZIP central directory 中记录的压缩后字节数。
    pub compressed_size: u64,
    /// Uncompressed byte length from the ZIP central directory.
    ///
    /// ZIP central directory 中记录的解压后字节数。
    pub uncompressed_size: u64,
}

/// Fully materialized archive entry to emit during export.
///
/// Metadata/config/thumbnail entries are usually small enough to build eagerly;
/// layer payloads are streamed separately through `LayerSource`.
///
/// 导出时要写入的、已 materialized 的 archive entry。metadata/config/thumbnail entry
/// 通常足够小，可以提前构建；layer payload 则通过 `LayerSource` 单独流式写入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    /// Archive path to write.
    ///
    /// 要写入的 archive 路径。
    pub name: String,
    /// Already-serialized entry payload.
    ///
    /// 已序列化的 entry payload。
    pub bytes: Vec<u8>,
    /// Whether to store without compression.
    ///
    /// 是否以 stored 方式写入而不压缩。
    pub store: bool,
}

/// Lightweight description of a layer archive entry.
///
/// The descriptor avoids storing layer bytes in the layout policy. The pipeline
/// supplies bytes after asking the layout only for name/compression.
///
/// layer archive entry 的轻量描述。descriptor 不在 layout policy 中保存 layer bytes；
/// pipeline 只向 layout 询问命名/压缩方式，再自行提供 payload。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntryDescriptor {
    /// Archive path for a layer entry.
    ///
    /// layer entry 的 archive 路径。
    pub name: String,
    /// Whether this layer entry should be stored without compression.
    ///
    /// 该 layer entry 是否应以 stored 方式写入。
    pub store: bool,
}

/// Export-time context shared by archive layout builders.
///
/// Keep this small: values here are document-assembly concerns, not codec or
/// registry facts.
///
/// archive layout builder 共享的导出时上下文。这里应保持轻量：只放 document assembly
/// 关心的信息，不放 codec 或 registry facts。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveEncodeContext<'a> {
    /// User-visible job/model name used by manifests that need a filename stem.
    ///
    /// 面向用户的任务/模型名，供需要 filename stem 的 manifest 使用。
    pub job_name: &'a str,
    /// Export target options selected before document assembly.
    ///
    /// document assembly 前已选择的导出目标选项。
    pub export_options: ExportTargetOptions<'a>,
}

impl<'a> ArchiveEncodeContext<'a> {
    /// Creates an archive encode context for a user-visible job/model name.
    ///
    /// 使用面向用户的任务/模型名创建 archive encode context。
    pub fn new(job_name: &'a str) -> Self {
        Self {
            job_name,
            export_options: ExportTargetOptions::default(),
        }
    }

    /// Carries export target options into document assembly.
    ///
    /// 将导出目标选项带入 document assembly。
    pub fn with_export_options(mut self, export_options: ExportTargetOptions<'a>) -> Self {
        self.export_options = export_options;
        self
    }
}

/// Archive layout policy for ZIP-like slicer containers.
///
/// `ArchiveLayout` describes how a format maps normalized layers and metadata to
/// archive entries: which existing ZIP entries are layer payloads, which
/// metadata entries must be emitted during export, and how each layer entry is
/// named/stored.
///
/// ZIP 类切片容器的 archive 布局策略。`ArchiveLayout` 描述格式如何把归一化 layer 与
/// metadata 映射到 archive entries：哪些已有 ZIP entry 是 layer payload，导出时需要
/// 生成哪些 metadata entries，以及每层 entry 的命名/存储方式。
pub trait ArchiveLayout: Sync {
    /// Maps archive directory entries to logical layer offsets.
    ///
    /// Implementations should sort the returned layers in logical layer order,
    /// even if the archive directory order differs.
    ///
    /// 将 archive directory entries 映射为逻辑 layer offsets。即使 archive directory
    /// 顺序不同，返回值也应按逻辑层序排序。
    fn list_layer_entries(
        &self,
        entries: &[ZipEntryInfo],
        spec: &FormatSpec,
    ) -> Result<Vec<LayerOffset>, ConvertError>;

    /// Builds non-layer archive entries needed for export.
    ///
    /// These entries include manifests, configs, thumbnails, or other metadata.
    /// Layer payload entries are described separately by `describe_layer_entry`.
    ///
    /// 构建导出所需的非 layer archive entries，例如 manifest、config、thumbnail 或其它
    /// metadata。layer payload entries 由 `describe_layer_entry` 单独描述。
    fn build_metadata_entries(
        &self,
        ctx: &ArchiveEncodeContext<'_>,
        metadata: &FileMetadata,
        total_layers: u32,
        spec: &FormatSpec,
    ) -> Result<Vec<ArchiveEntry>, ConvertError>;

    /// Describes the archive entry for one logical layer.
    ///
    /// The returned descriptor controls naming and compression only; the encoded
    /// payload bytes are supplied by the pipeline's `LayerSource`.
    ///
    /// 描述一个逻辑 layer 对应的 archive entry。返回值只控制命名与压缩方式；已编码
    /// payload bytes 由 pipeline 的 `LayerSource` 提供。
    fn describe_layer_entry(
        &self,
        ctx: &ArchiveEncodeContext<'_>,
        layer_index: u32,
        spec: &FormatSpec,
    ) -> ArchiveEntryDescriptor;
}

/// Builds a compressed archive metadata entry from bytes.
///
/// 从字节构建会被压缩的 archive metadata entry。
pub(crate) fn bytes_archive_entry(name: impl Into<String>, bytes: Vec<u8>) -> ArchiveEntry {
    ArchiveEntry {
        name: name.into(),
        bytes,
        store: false,
    }
}

/// Builds a compressed layer-entry descriptor with default compression.
///
/// 构建默认压缩的 layer entry descriptor。
pub(crate) fn archive_entry_descriptor(name: impl Into<String>) -> ArchiveEntryDescriptor {
    ArchiveEntryDescriptor {
        name: name.into(),
        store: false,
    }
}

/// Builds a UTF-8 text archive entry.
///
/// 构建 UTF-8 文本 archive entry。
pub(crate) fn text_archive_entry(name: impl Into<String>, text: impl Into<String>) -> ArchiveEntry {
    bytes_archive_entry(name, text.into().into_bytes())
}

/// Serializes JSON with stable pretty formatting for manifests.
///
/// 以稳定 pretty JSON 格式序列化 manifest 等 metadata。
pub(crate) fn json_archive_entry(
    name: impl Into<String>,
    value: impl Serialize,
) -> Result<ArchiveEntry, ConvertError> {
    let bytes = serde_json::to_vec_pretty(&value)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    Ok(bytes_archive_entry(name, bytes))
}

/// Checks the ZIP local-file-header magic at the start of a byte slice.
///
/// 检查字节开头是否为 ZIP local file header magic。
pub(crate) fn has_zip_magic(data: &[u8]) -> bool {
    data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4B
}

/// Searches for a marker anywhere in archive bytes.
///
/// Used only as a cheap family-level hint before profile-specific parsing.
///
/// 在 archive 字节中搜索 marker。仅作为 profile-specific parsing 前的轻量 family hint。
pub(crate) fn contains_archive_marker(data: &[u8], marker: impl AsRef<[u8]>) -> bool {
    let marker = marker.as_ref();
    !marker.is_empty() && data.windows(marker.len()).any(|window| window == marker)
}

/// Returns true when every marker exists somewhere in the archive bytes.
///
/// 当所有 marker 都能在 archive 字节中找到时返回 true。
pub(crate) fn contains_archive_markers<'a>(
    data: &[u8],
    markers: impl IntoIterator<Item = &'a str>,
) -> bool {
    markers
        .into_iter()
        .all(|marker| contains_archive_marker(data, marker))
}

/// Encodes a normalized RGBA thumbnail as PNG.
///
/// The input must already be RGBA8 with exactly `width * height * 4` bytes.
///
/// 将归一化 RGBA thumbnail 编码为 PNG。输入必须已经是 RGBA8，且字节数精确等于
/// `width * height * 4`。
pub(crate) fn encode_png_thumbnail(thumbnail: &Thumbnail) -> Result<Vec<u8>, ConvertError> {
    let expected = (thumbnail.width as usize)
        .saturating_mul(thumbnail.height as usize)
        .saturating_mul(4);
    if thumbnail.width == 0 || thumbnail.height == 0 || thumbnail.pixels_rgba.len() != expected {
        return Err(ConvertError::PngError(format!(
            "invalid thumbnail dimensions {}x{} with {} rgba bytes",
            thumbnail.width,
            thumbnail.height,
            thumbnail.pixels_rgba.len()
        )));
    }

    let mut bytes = Vec::new();
    {
        let mut encoder =
            png::Encoder::new(Cursor::new(&mut bytes), thumbnail.width, thumbnail.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.set_filter(png::Filter::Sub);
        let mut writer = encoder
            .write_header()
            .map_err(|error| ConvertError::PngError(error.to_string()))?;
        writer
            .write_image_data(&thumbnail.pixels_rgba)
            .map_err(|error| ConvertError::PngError(error.to_string()))?;
    }
    Ok(bytes)
}

/// Decodes a PNG thumbnail into normalized RGBA pixels.
///
/// Grayscale/RGB/RGBA PNGs are accepted and converted to RGBA8; indexed PNGs are
/// rejected because the current thumbnail path does not expand palettes.
///
/// 将 PNG thumbnail 解码为归一化 RGBA pixels。支持 grayscale/RGB/RGBA 并统一转成
/// RGBA8；当前 thumbnail 路径不展开 palette，因此拒绝 indexed PNG。
pub(crate) fn decode_png_thumbnail(png_data: &[u8]) -> Result<Thumbnail, ConvertError> {
    let decoder = png::Decoder::new(Cursor::new(png_data));
    let mut reader = decoder
        .read_info()
        .map_err(|error| ConvertError::PngError(error.to_string()))?;
    let mut buffer = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|error| ConvertError::PngError(error.to_string()))?;
    let used = frame.buffer_size();
    buffer.truncate(used);

    if frame.bit_depth != png::BitDepth::Eight {
        return Err(ConvertError::PngError(format!(
            "thumbnail bit depth {:?} is not supported",
            frame.bit_depth
        )));
    }

    let pixel_count = (frame.width as usize).saturating_mul(frame.height as usize);
    let mut pixels_rgba = Vec::with_capacity(pixel_count * 4);

    match frame.color_type {
        png::ColorType::Grayscale => {
            if buffer.len() < pixel_count {
                return Err(ConvertError::PngError(
                    "grayscale thumbnail data is truncated".to_string(),
                ));
            }
            for &value in buffer.iter().take(pixel_count) {
                pixels_rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            if buffer.len() < pixel_count * 2 {
                return Err(ConvertError::PngError(
                    "grayscale-alpha thumbnail data is truncated".to_string(),
                ));
            }
            for chunk in buffer.as_chunks::<2>().0.iter().take(pixel_count) {
                pixels_rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
        }
        png::ColorType::Rgb => {
            if buffer.len() < pixel_count * 3 {
                return Err(ConvertError::PngError(
                    "rgb thumbnail data is truncated".to_string(),
                ));
            }
            for chunk in buffer.as_chunks::<3>().0.iter().take(pixel_count) {
                pixels_rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        png::ColorType::Rgba => {
            if buffer.len() < pixel_count * 4 {
                return Err(ConvertError::PngError(
                    "rgba thumbnail data is truncated".to_string(),
                ));
            }
            pixels_rgba.extend_from_slice(&buffer[..pixel_count * 4]);
        }
        png::ColorType::Indexed => {
            return Err(ConvertError::PngError(
                "indexed thumbnail color type is not supported".to_string(),
            ));
        }
    }

    Ok(Thumbnail {
        width: frame.width,
        height: frame.height,
        pixels_rgba,
    })
}

/// Collects central-directory entry facts from a ZIP archive.
///
/// Returned `ZipEntryInfo.index` values are later used as logical offsets for
/// archive layer reads.
///
/// 从 ZIP archive 收集 central directory entry 事实。返回的 `ZipEntryInfo.index` 后续会
/// 作为 archive layer 读取用的逻辑 offset。
pub(crate) fn collect_zip_entries(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
) -> Result<Vec<ZipEntryInfo>, ConvertError> {
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|error| ConvertError::ZipError(error.to_string()))?;
        entries.push(ZipEntryInfo {
            index,
            name: file
                .name()
                .map_err(|error| ConvertError::ZipError(error.to_string()))?
                .into_owned(),
            compressed_size: file.compressed_size(),
            uncompressed_size: file.size(),
        });
    }
    Ok(entries)
}

/// Reads a required UTF-8 text entry from a ZIP archive.
///
/// Missing entries are errors and include the requested name in the message.
///
/// 从 ZIP archive 读取必需的 UTF-8 文本 entry。缺失是错误，错误消息包含请求名称。
pub(crate) fn read_required_zip_text(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<String, ConvertError> {
    let mut content = String::new();
    archive
        .by_name(name)
        .map_err(|error| ConvertError::ZipError(format!("{name} not found: {error}")))?
        .read_to_string(&mut content)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    Ok(content)
}

/// Reads an optional UTF-8 text entry from a ZIP archive.
///
/// Missing entries return `Ok(None)`; malformed or unreadable entries still
/// return an error.
///
/// 从 ZIP archive 读取可选 UTF-8 文本 entry。缺失返回 `Ok(None)`；存在但格式错误或读取
/// 失败仍返回错误。
pub(crate) fn read_optional_zip_text(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<String>, ConvertError> {
    let Ok(mut file) = archive.by_name(name) else {
        return Ok(None);
    };
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    Ok(Some(content))
}

/// Reads an optional binary entry from a ZIP archive.
///
/// Missing entries return `Ok(None)`.
///
/// 从 ZIP archive 读取可选二进制 entry。缺失返回 `Ok(None)`。
pub(crate) fn read_optional_zip_bytes(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<Vec<u8>>, ConvertError> {
    let Ok(mut file) = archive.by_name(name) else {
        return Ok(None);
    };
    let mut content = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut content)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    Ok(Some(content))
}

/// Reads a ZIP entry payload by central-directory index.
///
/// Archive `LayerOffset.offset` stores this entry index rather than a byte
/// offset, so callers should pass the value through unchanged.
///
/// 按 central directory index 读取 ZIP entry payload。Archive 的 `LayerOffset.offset`
/// 存的是该 entry index，而不是字节偏移；调用方应原样传入。
pub(crate) fn read_zip_entry_bytes(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    entry_index: usize,
) -> Result<Vec<u8>, ConvertError> {
    let mut file = archive
        .by_index(entry_index)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    Ok(bytes)
}

/// Builds logical layer offsets from archive entries and a name-to-layer parser.
///
/// The returned offsets use ZIP entry indexes as `offset` values, and are sorted
/// by the parsed logical layer index.
///
/// 根据 archive entries 与 name-to-layer parser 构建逻辑 layer offsets。返回值的
/// `offset` 使用 ZIP entry index，并按解析出的逻辑 layer index 排序。
pub(crate) fn sorted_layer_offsets<'a>(
    entries: impl IntoIterator<Item = &'a ZipEntryInfo>,
    mut layer_index: impl FnMut(&str) -> Option<u32>,
) -> Vec<LayerOffset> {
    let mut layers = entries
        .into_iter()
        .filter_map(|entry| Some((layer_index(&entry.name)?, entry)))
        .collect::<Vec<_>>();
    layers.sort_by_key(|(layer_index, _)| *layer_index);

    layers
        .into_iter()
        .map(|(_, entry)| LayerOffset {
            offset: entry.index as u64,
            size: entry.uncompressed_size,
        })
        .collect()
}

/// Archive layout used by non-archive formats.
///
/// This keeps `FormatSpec` composition uniform: binary formats can still carry
/// an archive policy without special-casing every call site.
///
/// 非 archive 格式使用的 archive layout。它让 `FormatSpec` 组合保持统一：二进制格式也能
/// 携带 archive policy，而调用点不需要到处特判。
pub struct NoArchiveLayout;

impl ArchiveLayout for NoArchiveLayout {
    fn list_layer_entries(
        &self,
        _entries: &[ZipEntryInfo],
        _spec: &FormatSpec,
    ) -> Result<Vec<LayerOffset>, ConvertError> {
        Ok(Vec::new())
    }

    fn build_metadata_entries(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        _metadata: &FileMetadata,
        _total_layers: u32,
        _spec: &FormatSpec,
    ) -> Result<Vec<ArchiveEntry>, ConvertError> {
        Ok(Vec::new())
    }

    fn describe_layer_entry(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        layer_index: u32,
        _spec: &FormatSpec,
    ) -> ArchiveEntryDescriptor {
        ArchiveEntryDescriptor {
            name: layer_index.to_string(),
            store: false,
        }
    }
}
