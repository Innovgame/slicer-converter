use crate::common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage};

use super::{
    keys, ExportTargetOptions, FormatFamilySpec, FormatSpec, LayerDecodeContext, LayerEncodeContext,
};

/// Header and document-layout contract for one concrete format profile.
///
/// `HeaderLayout` detects and parses file-level metadata, builds the initial
/// header, and fills offset/size/table fields after every layer size is known.
///
/// 具体格式 profile 的头部与文档布局契约。`HeaderLayout` 负责识别/解析文件级
/// metadata、构建初始 header，并在所有 layer 编码完成后用真实 layer size 回填
/// offset/size/table 等布局字段。
pub trait HeaderLayout: Sync {
    /// Returns whether this layout can handle the given bytes and optional extension/name hint.
    ///
    /// Implementations should perform lightweight detection only, not full parsing.
    ///
    /// 判断该 layout 是否能处理给定字节与可选扩展名/格式名提示。实现应只做轻量识别，
    /// 不应执行完整解析。
    fn try_detect(&self, data: &[u8], hint: Option<&str>) -> bool;

    /// Parses file-level metadata and layer byte ranges from the full document.
    ///
    /// This does not decode layer pixels; it only returns offset/size ranges for `LayerCodec`.
    ///
    /// 从完整文件中解析文件级 metadata 与每层数据范围。这里不解码 layer 像素，只返回
    /// 后续 `LayerCodec` 需要读取的 offset/size。
    fn parse_header(
        &self,
        data: &[u8],
        spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError>;

    /// Builds an initial header from normalized metadata.
    ///
    /// At this stage the real layer payload sizes are usually unknown, so layer-table
    /// offset/size fields may be placeholders.
    ///
    /// 基于归一化 metadata 构建初始 header。此阶段通常还不知道真实 layer payload
    /// size，因此 layer table 中的 offset/size 可以先写占位值。
    fn build_header(
        &self,
        metadata: &FileMetadata,
        spec: &FormatSpec,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError>;

    /// Patches or rebuilds the header after layer encoding has produced final sizes.
    ///
    /// Some formats only patch the layer table; formats such as CTB v3+ may also
    /// need to rearrange extended layer blocks inside the header area.
    ///
    /// 在 layer 编码完成并得到真实大小后，回填或重建 header。某些格式只 patch
    /// layer table；CTB v3+ 这类格式还可能需要重新安排 header 内部的扩展 layer block。
    fn finalize_header(
        &self,
        header: &[u8],
        layer_sizes: &[u32],
        spec: &FormatSpec,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError>;

    /// Finalize variant that accepts an owned header buffer.
    ///
    /// Implementations may override this to reuse the buffer; the default delegates
    /// to `finalize_header` to keep the contract simple.
    ///
    /// 接收 owned header 的 finalize 变体。实现可在需要时覆写以复用缓冲区，默认委托给
    /// `finalize_header` 保持契约简单。
    fn finalize_header_owned(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
        spec: &FormatSpec,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        self.finalize_header(&header, layer_sizes, spec, options)
    }
}

/// Per-layer image codec contract for one concrete format profile.
///
/// `LayerCodec` only converts between one layer payload and grayscale pixels;
/// encryption, archive entry lookup, and file-level header/finalize work belong to
/// the outer `FormatSpec` and pipeline.
///
/// 具体格式 profile 的逐层图像编解码契约。`LayerCodec` 只处理单层 payload 与灰度像素
/// 之间的转换；加密、archive entry 查找、文件级 header/finalize 由外层 `FormatSpec`
/// 和 pipeline 负责。
pub trait LayerCodec: Sync {
    /// Decodes one encoded layer payload into 8-bit grayscale pixels.
    ///
    /// Implementations should clear and reuse `output`, and ensure it matches
    /// `ctx.width * ctx.height`.
    ///
    /// 将单层编码 payload 解码为 8-bit 灰度像素。实现应清空并复用 `output`，并保证输出
    /// 与 `ctx.width * ctx.height` 对应。
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError>;

    /// Encodes one 8-bit grayscale layer into the format-specific payload bytes.
    ///
    /// Implementations should write to `output`; callers handle encryption or
    /// archive wrapping at the outer profile layer.
    ///
    /// 将单层 8-bit 灰度像素编码为格式特定 payload。实现应写入 `output`，调用方会在
    /// 外层根据 profile 策略处理加密或 archive 包装。
    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError>;

    /// Optionally decodes directly into a bounded RGBA preview.
    ///
    /// Return `Ok(None)` when the codec has no fast path or this input is not suitable;
    /// callers should fall back to `decode_layer_into` plus shared downsampling. Return
    /// `Err` only for real decode failures.
    ///
    /// 可选的 preview 快路径：直接从单层 payload 解码出不超过 `max_preview_size` 的
    /// RGBA 预览图。返回 `Ok(None)` 表示该 codec 不提供快路径或当前输入不适合快路径，
    /// 调用方应回退到 `decode_layer_into` 后统一 downsample；只有真实解码错误才返回
    /// `Err`。
    fn decode_preview_rgba(
        &self,
        _data: &[u8],
        _ctx: &LayerDecodeContext,
        _max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        Ok(None)
    }
}

impl FormatSpec {
    /// Returns whether a request names this concrete profile.
    ///
    /// The request may be a canonical name, alias, extension, or catalog export
    /// target. Leading dots are ignored so callers can pass either `"ctb"` or
    /// `".ctb"`.
    ///
    /// 判断请求是否指向该具体 profile。请求可以是规范名、别名、扩展名或 catalog export
    /// target；开头的点会被忽略，因此调用方可传 `"ctb"` 或 `".ctb"`。
    pub fn matches_name(&self, request: &str) -> bool {
        let request = request.trim_start_matches('.');
        self.identity.name.eq_ignore_ascii_case(request)
            || self
                .identity
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(request))
            || self.matches_extension(request)
            || self
                .catalog
                .target_names
                .iter()
                .any(|target| target.eq_ignore_ascii_case(request))
    }

    /// Returns whether an extension belongs to this profile.
    ///
    /// Only `identity.extensions` participate here; aliases and catalog target
    /// names are handled by `matches_name`.
    ///
    /// 判断扩展名是否属于该 profile。这里仅检查 `identity.extensions`；别名和 catalog target
    /// names 由 `matches_name` 处理。
    pub fn matches_extension(&self, extension: &str) -> bool {
        let extension = extension.trim_start_matches('.');
        self.identity
            .extensions
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    }

    /// Decodes one encoded layer into a newly allocated grayscale buffer.
    ///
    /// Call `decode_layer_into` when the caller can reuse an output buffer.
    ///
    /// 将单层 encoded payload 解码到新分配的灰度 buffer。调用方可复用输出 buffer 时应使用
    /// `decode_layer_into`。
    pub fn decode_layer(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
    ) -> Result<Vec<u8>, ConvertError> {
        let mut output = Vec::new();
        self.decode_layer_into(data, ctx, &mut output)?;
        Ok(output)
    }

    /// Decodes one encoded layer into caller-owned output.
    ///
    /// The profile applies crypto first when needed, then delegates pixel decode
    /// to `LayerCodec`.
    ///
    /// 将单层 encoded payload 解码到调用方持有的输出 buffer。profile 会在需要时先应用
    /// crypto，再委托 `LayerCodec` 做像素解码。
    pub fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        if self.keys.crypto_key == keys::NO_CRYPTO {
            return self.layer_codec.decode_layer_into(data, ctx, output);
        }

        let mut payload = data.to_vec();
        self.crypto
            .decrypt_layer_in_place(&mut payload, ctx.layer_index, ctx)?;
        self.layer_codec.decode_layer_into(&payload, ctx, output)
    }

    /// Attempts a direct RGBA preview decode for one layer.
    ///
    /// `Ok(None)` means callers should fall back to full grayscale decode plus
    /// shared preview downsampling; it is not an error.
    ///
    /// 尝试对单层直接解码 RGBA preview。`Ok(None)` 表示调用方应回退到完整灰度解码再使用共享
    /// preview downsampling；这不是错误。
    pub fn decode_layer_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        if self.keys.crypto_key == keys::NO_CRYPTO {
            return self
                .layer_codec
                .decode_preview_rgba(data, ctx, max_preview_size);
        }

        let mut payload = data.to_vec();
        self.crypto
            .decrypt_layer_in_place(&mut payload, ctx.layer_index, ctx)?;
        self.layer_codec
            .decode_preview_rgba(&payload, ctx, max_preview_size)
    }

    /// Encodes one grayscale layer into a newly allocated payload buffer.
    ///
    /// Call `encode_layer_into` when the caller can reuse an output buffer.
    ///
    /// 将单层灰度像素编码到新分配的 payload buffer。调用方可复用输出 buffer 时应使用
    /// `encode_layer_into`。
    pub fn encode_layer(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        let mut output = Vec::new();
        self.encode_layer_into(pixels, ctx, &mut output)?;
        Ok(output)
    }

    /// Encodes one grayscale layer into caller-owned output.
    ///
    /// The codec writes plain encoded bytes first; the profile applies crypto as
    /// the final per-layer transform.
    ///
    /// 将单层灰度像素编码到调用方持有的输出 buffer。codec 先写入明文 encoded bytes；
    /// profile 再把 crypto 作为最后的逐层转换应用。
    pub fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        self.layer_codec.encode_layer_into(pixels, ctx, output)?;
        self.crypto
            .encrypt_layer_in_place(output, ctx.layer_index, ctx)?;
        Ok(())
    }

    /// Builds this profile's initial header from normalized metadata.
    ///
    /// Layer payload sizes may still be placeholders until `finalize_header`.
    ///
    /// 根据归一化 metadata 构建该 profile 的初始 header。layer payload size 可能仍是占位值，
    /// 直到 `finalize_header` 才确定。
    pub fn build_header(&self, metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
        self.build_header_with_options(metadata, ExportTargetOptions::default())
    }

    /// Builds this profile's initial header with export target options.
    ///
    /// 使用导出目标选项构建该 profile 的初始 header。
    pub fn build_header_with_options(
        &self,
        metadata: &FileMetadata,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        self.header.build_header(metadata, self, options)
    }

    /// Finalizes this profile's header once encoded layer sizes are known.
    ///
    /// 二进制组装阶段在 encoded layer size 已知后调用，用于回填 offset/size/table 等字段。
    pub fn finalize_header(
        &self,
        header: &[u8],
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError> {
        self.header
            .finalize_header(header, layer_sizes, self, ExportTargetOptions::default())
    }

    /// Finalizes this profile's header with export target options.
    ///
    /// 使用导出目标选项回填该 profile 的 header。
    pub fn finalize_header_with_options(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        self.header
            .finalize_header_owned(header, layer_sizes, self, options)
    }
}

impl FormatFamilySpec {
    /// Returns whether a request names this family.
    ///
    /// Family matching accepts the family name, aliases, and broad extensions.
    /// Concrete profile target names are intentionally handled by profile
    /// resolvers instead.
    ///
    /// 判断请求是否指向该 family。family matching 接受 family name、aliases 与粗粒度
    /// extensions；具体 profile target names 则交给 profile resolver 处理。
    pub fn matches_name(&self, request: &str) -> bool {
        let request = request.trim_start_matches('.');
        self.identity.name.eq_ignore_ascii_case(request)
            || self
                .identity
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(request))
            || self
                .identity
                .extensions
                .iter()
                .any(|extension| extension.eq_ignore_ascii_case(request))
    }
}
