/// Creality CXDLP profile facts.
///
/// These facts separate the public catalog/profile matrix from the large binary
/// parser in `cxdlp.rs`.
///
/// Creality CXDLP profile facts。它把公开 catalog/profile 矩阵与 `cxdlp.rs` 中较大的二进制
/// parser 分开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrealityCxdLpSpec {
    /// Human-readable variant label.
    ///
    /// 面向人的 variant label。
    pub variant: &'static str,
    /// CXDLP version number.
    ///
    /// CXDLP version number。
    pub version: u32,
    /// Preferred extension.
    ///
    /// 首选扩展名。
    pub extension: &'static str,
    /// File magic marker.
    ///
    /// 文件 magic marker。
    pub magic: &'static str,
    /// Checksum algorithm label.
    ///
    /// checksum algorithm label。
    pub checksum: &'static str,
    /// Layer definition record size.
    ///
    /// layer definition record size。
    pub layer_definition_size: u32,
    /// Layer parameter record size.
    ///
    /// layer parameter record size。
    pub layer_parameter_size: u32,
    /// Number of preview images expected by this variant.
    ///
    /// 该 variant 预期的 preview image 数量。
    pub preview_count: u32,
    /// Whether layer payload encryption is supported.
    ///
    /// 是否支持 layer payload encryption。
    pub supports_layer_encryption: bool,
}
