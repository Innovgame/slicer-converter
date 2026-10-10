use crate::common::{ConvertError, FileMetadata};

use super::FormatSpec;

/// Optional footer builder for binary formats.
///
/// Most formats have no footer and use `NoFooter`; formats with trailing tables
/// can implement this policy without changing their layer codec.
///
/// 二进制格式的可选 footer 构建策略。大多数格式没有 footer 并使用 `NoFooter`；带尾部
/// table 的格式可实现此策略，而不需要改变 layer codec。
pub trait FooterPolicy: Sync {
    /// Build trailing bytes after the main header and all encoded layers.
    ///
    /// The binary pipeline appends this result once layer offsets and metadata
    /// have already been finalized. Archive formats normally use
    /// `NoArchiveLayout`/`NoFooter` instead of this hook.
    ///
    /// 在 main header 和所有 encoded layers 之后构建尾部字节。binary pipeline 会在 layer
    /// offsets 与 metadata 已确定后追加该结果；archive 格式通常使用 `NoArchiveLayout`/`NoFooter`，
    /// 不通过这个 hook。
    fn build_footer(
        &self,
        metadata: &FileMetadata,
        spec: &FormatSpec,
    ) -> Result<Vec<u8>, ConvertError>;
}

/// Footer policy for formats without trailing bytes.
///
/// 无尾部字节格式使用的 footer 策略。
pub struct NoFooter;

impl FooterPolicy for NoFooter {
    fn build_footer(
        &self,
        _metadata: &FileMetadata,
        _spec: &FormatSpec,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(Vec::new())
    }
}
