//! Document-level parse and assembly pipelines.
//!
//! A pipeline coordinates already-encoded layer payloads with header, footer,
//! archive, and sink policies. It deliberately sits above `LayerCodec`: layer
//! pixels should already be encoded before document assembly begins.
//!
//! document 级 parse 与 assembly pipeline。pipeline 负责协调已编码 layer payload 与
//! header、footer、archive、sink policy；它刻意位于 `LayerCodec` 之上，document
//! assembly 开始前 layer pixels 应已完成编码。

use std::io::Write;

use crate::common::{ConvertError, FileMetadata, LayerOffset};

use super::{ArchiveEncodeContext, FormatSpec};

/// Parsed document envelope returned by a format pipeline.
///
/// The pipeline exposes normalized metadata plus byte ranges for encoded layer
/// payloads. Pixel decoding remains the responsibility of `LayerCodec`.
///
/// pipeline 解析出的文件外壳：归一化 metadata 与每层已编码 payload 的字节范围。
/// 像素解码仍由 `LayerCodec` 负责。
pub struct ParsedDocument {
    /// Normalized file metadata used by UI, transcode, and export.
    ///
    /// 供 UI、transcode 与 export 使用的归一化文件 metadata。
    pub metadata: FileMetadata,
    /// Encoded layer payload ranges inside the original document bytes.
    ///
    /// 原始文件字节中已编码 layer payload 的范围。
    pub layers: Vec<LayerOffset>,
}

/// Pull-based source for encoded layer payloads.
///
/// Archive pipelines use this to stream layer bytes without materializing every
/// encoded layer in memory at once.
///
/// 已编码 layer payload 的拉取式来源。Archive pipeline 用它按需读取 layer，避免一次性
/// materialize 全部已编码 layer。
pub trait EncodedLayerStream {
    /// Reads one encoded layer into the caller-owned buffer.
    ///
    /// Implementations should clear and refill `output`. The layer index is the
    /// logical zero-based layer order, not an archive entry index.
    ///
    /// 将单个已编码 layer 读入调用方持有的 buffer。实现应清空并重填 `output`。
    /// `layer_index` 是从 0 开始的逻辑层序，不是 archive entry index。
    fn read_layer_into(
        &mut self,
        layer_index: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError>;
}

/// Encoded layer input for document assembly.
///
/// Binary formats currently require in-memory encoded layers because their
/// finalized header may depend on every layer size. Archive formats can also
/// consume a streaming source.
///
/// document 组装时的已编码 layer 输入。二进制格式当前需要内存中的全部 encoded layers，
/// 因为 finalized header 可能依赖所有 layer size；archive 格式也可消费 streaming source。
pub enum LayerSource<'a> {
    /// All encoded layers are already materialized in memory.
    ///
    /// 所有已编码 layers 已经在内存中 materialized。
    Encoded(&'a [Vec<u8>]),
    /// Encoded layers are pulled on demand by layer order.
    ///
    /// 已编码 layers 按层序按需拉取。当前只由 archive pipeline 消费。
    Streaming {
        /// Total logical layer count expected from the stream.
        ///
        /// stream 应提供的逻辑 layer 总数。
        layer_count: usize,
        /// Pull source for each encoded layer payload.
        ///
        /// 每个已编码 layer payload 的拉取来源。
        stream: &'a mut dyn EncodedLayerStream,
    },
}

impl<'a> LayerSource<'a> {
    fn encoded(&self) -> Result<&'a [Vec<u8>], ConvertError> {
        match self {
            Self::Encoded(layers) => Ok(layers),
            Self::Streaming { .. } => Err(ConvertError::ZipError(
                "streaming layer source is only supported by archive pipelines".to_string(),
            )),
        }
    }

    fn layer_count(&self) -> usize {
        match self {
            Self::Encoded(layers) => layers.len(),
            Self::Streaming { layer_count, .. } => *layer_count,
        }
    }
}

/// Summary of bytes and logical layers written by a document pipeline.
///
/// This is intentionally small so both in-memory and streaming output paths can
/// report the same completion facts.
///
/// document pipeline 写出结果的摘要：写入字节数和逻辑层数。它刻意保持很小，让内存输出与
/// streaming 输出路径都能报告同一组完成事实。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentEncodeStats {
    /// Bytes written to the output sink by the pipeline.
    ///
    /// pipeline 写入 output sink 的字节数。
    pub bytes_written: usize,
    /// Logical layer payloads written to the document.
    ///
    /// 写入 document 的逻辑 layer payload 数量。
    pub layers_written: usize,
}

/// Byte sink used by document pipelines.
///
/// Implementations may be an in-memory buffer, a browser file stream, or any
/// other sequential output target.
///
/// document pipeline 的字节输出目标。实现可以是内存 buffer、浏览器文件流，或其它顺序
/// 写入目标。
pub trait OutputSink {
    /// Appends a contiguous byte slice to the output.
    ///
    /// The sink contract is sequential: callers should not seek backward unless
    /// they are using a richer sink type outside this trait.
    ///
    /// 向输出追加一段连续字节。该 sink 契约是顺序写入；除非使用此 trait 之外的更强
    /// sink 类型，否则调用方不应回退 seek。
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), ConvertError>;
}

/// In-memory output sink used by tests and non-streaming callers.
///
/// 测试与非 streaming 调用方使用的内存 output sink。
pub struct VecOutputSink {
    bytes: Vec<u8>,
}

impl VecOutputSink {
    /// Creates an empty in-memory sink.
    ///
    /// 创建空的内存 sink。
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Returns the accumulated document bytes.
    ///
    /// 返回已累计的 document bytes。
    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl Default for VecOutputSink {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for VecOutputSink {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), ConvertError> {
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

/// File-level parser and assembler for a format container.
///
/// `FormatPipeline` sits above `HeaderLayout` and `LayerCodec`: it parses a full
/// document into metadata/layer ranges, and assembles a full document from an
/// initial header, encoded layers, optional archive metadata, and footer policy.
///
/// 格式容器级 parser/assembler。`FormatPipeline` 位于 `HeaderLayout` 与
/// `LayerCodec` 之上：负责把完整文件解析为 metadata/layer ranges，也负责用 header、
/// 已编码 layers、可选 archive metadata 与 footer policy 组装完整文件。
pub trait FormatPipeline: Sync {
    /// Parses a complete document into metadata and encoded layer ranges.
    ///
    /// 解析完整 document，返回 metadata 与已编码 layer ranges。
    fn parse_document(
        &self,
        spec: &FormatSpec,
        data: &[u8],
    ) -> Result<ParsedDocument, ConvertError>;

    /// Writes a complete document to `sink` from already-encoded layers.
    ///
    /// The pipeline decides whether to use binary header/footer layout or archive
    /// entries, but it should not decode/re-encode pixels.
    ///
    /// 根据已编码 layers 写出完整 document 到 `sink`。pipeline 决定使用二进制
    /// header/footer 布局还是 archive entries，但不应解码/重新编码像素。
    fn encode_document_into(
        &self,
        spec: &FormatSpec,
        metadata: &FileMetadata,
        layers: LayerSource<'_>,
        sink: &mut dyn OutputSink,
        ctx: &ArchiveEncodeContext<'_>,
    ) -> Result<DocumentEncodeStats, ConvertError>;
}

/// Pipeline for single-file binary formats.
///
/// It finalizes the header after layer sizes are known, then writes payloads at
/// the offsets declared by the finalized header. This supports CTB-style layouts
/// where metadata blocks can live between the main header and layer payloads.
///
/// 单文件二进制格式 pipeline。它在 layer size 已知后 finalize header，并按 finalized
/// header 声明的 offset 写入 payload；这支持 CTB 这类 header 与 payload 之间还存在
/// 扩展 metadata block 的布局。
pub struct DefaultBinaryPipeline;

/// Pipeline for ZIP/archive-based formats.
///
/// It writes metadata entries first and then layer entries in layer-index order,
/// optionally pulling each encoded layer from a streaming source.
///
/// ZIP/archive 格式 pipeline。先写 metadata entries，再按 layer index 写入 layer
/// entries；layer payload 可以来自 streaming source。
pub struct DefaultArchivePipeline;

struct OutputSinkWriter<'a> {
    sink: &'a mut dyn OutputSink,
    bytes_written: usize,
}

impl<'a> OutputSinkWriter<'a> {
    fn new(sink: &'a mut dyn OutputSink) -> Self {
        Self {
            sink,
            bytes_written: 0,
        }
    }
}

impl Write for OutputSinkWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink
            .write_all(buf)
            .map_err(|err| std::io::Error::other(err.to_string()))?;
        self.bytes_written += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl FormatPipeline for DefaultBinaryPipeline {
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
        ctx: &ArchiveEncodeContext<'_>,
    ) -> Result<DocumentEncodeStats, ConvertError> {
        let layers = layers.encoded()?;
        let layer_sizes = layers
            .iter()
            .map(|layer| layer.len() as u32)
            .collect::<Vec<_>>();
        let header = spec
            .header
            .build_header(metadata, spec, ctx.export_options)?;
        let header =
            spec.header
                .finalize_header_owned(header, &layer_sizes, spec, ctx.export_options)?;
        let footer = spec.footer.build_footer(metadata, spec)?;

        write_binary_document_from_finalized_layout(spec, &header, layers, sink)?;
        sink.write_all(&footer)?;

        Ok(DocumentEncodeStats {
            bytes_written: header.len() + layers.iter().map(Vec::len).sum::<usize>() + footer.len(),
            layers_written: layers.len(),
        })
    }
}

fn write_binary_document_from_finalized_layout(
    spec: &FormatSpec,
    header: &[u8],
    layers: &[Vec<u8>],
    sink: &mut dyn OutputSink,
) -> Result<(), ConvertError> {
    // Some binary formats place metadata tables between the main header and
    // layer payloads. Re-parse the finalized header so layer payloads are
    // written at the exact offsets that external tools will read.
    let Ok((_, layer_offsets)) = spec.header.parse_header(header, spec) else {
        sink.write_all(header)?;
        for layer in layers {
            sink.write_all(layer)?;
        }
        return Ok(());
    };
    if layer_offsets.len() != layers.len() {
        return Err(ConvertError::ZipError(format!(
            "finalized header has {} layer offsets but {} layers were encoded",
            layer_offsets.len(),
            layers.len()
        )));
    }

    let mut ordered = layer_offsets.into_iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by_key(|(_, layer)| layer.offset);

    let mut cursor = 0usize;
    for (index, layer_offset) in ordered {
        let start = usize::try_from(layer_offset.offset).map_err(|_| {
            ConvertError::ZipError(format!("layer {index} offset does not fit in usize"))
        })?;
        let size = usize::try_from(layer_offset.size).map_err(|_| {
            ConvertError::ZipError(format!("layer {index} size does not fit in usize"))
        })?;
        let end = start.checked_add(size).ok_or_else(|| {
            ConvertError::ZipError(format!("layer {index} offset plus size overflows"))
        })?;
        if size != layers[index].len() {
            return Err(ConvertError::ZipError(format!(
                "layer {index} finalized size {size} does not match encoded size {}",
                layers[index].len()
            )));
        }
        if start < cursor {
            return Err(ConvertError::ZipError(format!(
                "layer {index} overlaps previously written binary document region"
            )));
        }
        if cursor < start {
            write_binary_gap(header, cursor, start, sink)?;
        }
        sink.write_all(&layers[index])?;
        cursor = end;
    }

    if cursor < header.len() {
        sink.write_all(&header[cursor..])?;
    }
    Ok(())
}

fn write_binary_gap(
    header: &[u8],
    cursor: usize,
    start: usize,
    sink: &mut dyn OutputSink,
) -> Result<(), ConvertError> {
    // Preserve finalized-header bytes when the gap overlaps the header buffer;
    // pad with zeroes only when a layer offset points beyond it.
    if cursor < header.len() {
        sink.write_all(&header[cursor..header.len().min(start)])?;
    }
    if start > header.len() {
        let mut remaining = start - cursor.max(header.len());
        const ZERO_CHUNK: [u8; 1024] = [0; 1024];
        while remaining > 0 {
            let count = remaining.min(ZERO_CHUNK.len());
            sink.write_all(&ZERO_CHUNK[..count])?;
            remaining -= count;
        }
    }
    Ok(())
}

impl FormatPipeline for DefaultArchivePipeline {
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
        ctx: &ArchiveEncodeContext<'_>,
    ) -> Result<DocumentEncodeStats, ConvertError> {
        let total_layers = layers.layer_count();
        let mut writer = OutputSinkWriter::new(sink);

        {
            // `ZipWriter::new_stream` keeps this path compatible with browser
            // streaming sinks: each layer can be pulled, written, and dropped.
            let mut zip = zip::ZipWriter::new_stream(&mut writer);
            let metadata_entries =
                spec.archive
                    .build_metadata_entries(ctx, metadata, total_layers as u32, spec)?;

            for entry in metadata_entries {
                zip.start_file(entry.name, zip_options(entry.store))
                    .map_err(|err| ConvertError::ZipError(err.to_string()))?;
                zip.write_all(&entry.bytes)
                    .map_err(|err| ConvertError::ZipError(err.to_string()))?;
            }

            match layers {
                LayerSource::Encoded(encoded_layers) => {
                    for (layer_index, layer) in encoded_layers.iter().enumerate() {
                        let descriptor =
                            spec.archive
                                .describe_layer_entry(ctx, layer_index as u32, spec);
                        zip.start_file(descriptor.name, zip_options(descriptor.store))
                            .map_err(|err| ConvertError::ZipError(err.to_string()))?;
                        zip.write_all(layer)
                            .map_err(|err| ConvertError::ZipError(err.to_string()))?;
                    }
                }
                LayerSource::Streaming {
                    layer_count,
                    stream,
                } => {
                    let mut layer_buf = Vec::new();
                    for layer_index in 0..layer_count {
                        layer_buf.clear();
                        stream.read_layer_into(layer_index, &mut layer_buf)?;
                        let descriptor =
                            spec.archive
                                .describe_layer_entry(ctx, layer_index as u32, spec);
                        zip.start_file(descriptor.name, zip_options(descriptor.store))
                            .map_err(|err| ConvertError::ZipError(err.to_string()))?;
                        zip.write_all(&layer_buf)
                            .map_err(|err| ConvertError::ZipError(err.to_string()))?;
                    }
                }
            }

            zip.finish()
                .map_err(|err| ConvertError::ZipError(err.to_string()))?;
        }

        Ok(DocumentEncodeStats {
            bytes_written: writer.bytes_written,
            layers_written: total_layers,
        })
    }
}

fn zip_options(store: bool) -> zip::write::SimpleFileOptions {
    let method = if store {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Deflated
    };
    zip::write::SimpleFileOptions::default().compression_method(method)
}
