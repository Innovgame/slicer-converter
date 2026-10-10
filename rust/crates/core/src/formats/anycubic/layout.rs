use crate::common::{ConvertError, FileMetadata, LayerOffset};

use super::codec::AnycubicLayerCodec;
use super::{AnycubicSpec, AnycubicVersion};

/// Runtime strategy for one Anycubic binary table layout.
///
/// Anycubic binary versions share layer codecs but differ in file-mark/table
/// ordering, fixed table sizes, and header finalization rules. A layout strategy
/// owns those binary-container details for one version.
///
/// 单个 Anycubic binary table layout 的运行期策略。Anycubic binary versions 共享 layer
/// codecs，但 file-mark/table 顺序、固定 table sizes 与 header finalization 规则不同；
/// layout strategy 负责某个版本的 binary-container 细节。
pub trait AnycubicLayoutStrategy: Sync {
    /// Return the Anycubic file version implemented by this layout.
    ///
    /// 返回该 layout 实现的 Anycubic file version。
    fn version(&self) -> AnycubicVersion;

    /// Build an Anycubic header with placeholder layer offsets.
    ///
    /// The final layer sizes are not known at this stage; callers must invoke
    /// [`AnycubicLayoutStrategy::finalize_header`] after encoding layer payloads.
    ///
    /// 构建带 placeholder layer offsets 的 Anycubic header。此阶段尚不知道最终 layer
    /// sizes；调用方必须在编码 layer payloads 后调用
    /// [`AnycubicLayoutStrategy::finalize_header`]。
    fn build_header(
        &self,
        metadata: &FileMetadata,
        specifics: &AnycubicSpec,
        codec: &dyn AnycubicLayerCodec,
    ) -> Result<Vec<u8>, ConvertError>;

    /// Patch a header with final layer offsets and sizes.
    ///
    /// Implementations may also append/patch version-specific auxiliary tables
    /// such as sub-layer definition tables.
    ///
    /// 使用最终 layer offsets 与 sizes patch header。实现也可能追加或修正版本专属辅助表，
    /// 例如 sub-layer definition tables。
    fn finalize_header(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
    ) -> Result<Vec<u8>, ConvertError>;

    /// Parse header bytes into `(FileMetadata, layer offsets)`.
    ///
    /// Any registered Anycubic binary layout must provide a real parser; import
    /// capability is no longer represented by a shared unsupported default.
    ///
    /// 将 header bytes 解析为 `(FileMetadata, layer offsets)`。所有注册的 Anycubic
    /// binary layout 都必须提供真实 parser；import 能力不再由共享 unsupported default 表达。
    fn parse_header(&self, data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError>;
}
