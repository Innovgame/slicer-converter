use serde::Serialize;
use thiserror::Error;

/// Structured error payload for JS/CLI-facing diagnostics.
///
/// `ConvertError` remains the Rust error type; this serializable shape provides
/// stable machine-readable fields for callers that cannot inspect Rust enum
/// variants directly.
///
/// 面向 JS/CLI 诊断的结构化错误 payload。`ConvertError` 仍是 Rust 内部错误类型；该可序列化结构
/// 为无法直接检查 Rust enum variant 的调用方提供稳定的机器可读字段。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructuredConvertError {
    /// Stable error code.
    ///
    /// 稳定错误码。
    pub code: &'static str,
    /// Human-readable error message.
    ///
    /// 面向人的错误信息。
    pub message: String,
    /// Whether retrying the same operation is expected to help.
    ///
    /// 重试同一操作是否可能有帮助。
    pub retryable: bool,
    /// Optional machine-readable or diagnostic detail.
    ///
    /// 可选的机器可读或诊断细节。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Unified error type for all slicer converter operations.
/// 切片转换器统一错误类型，涵盖格式解析、RLE 编解码、ZIP、PNG 等所有操作。
#[derive(Error, Debug)]
pub enum ConvertError {
    /// File magic did not match the expected format signature.
    ///
    /// 文件 magic 与目标格式签名不匹配。
    #[error("Invalid magic number: expected {expected:#010x}, got {actual:#010x}")]
    InvalidMagic { expected: u32, actual: u32 },

    /// Format version is recognized as a version field but not supported.
    ///
    /// 版本字段可识别，但当前实现不支持该版本。
    #[error("Unsupported format version: {0}")]
    UnsupportedVersion(u32),

    /// No registered format/profile matched the input or request.
    ///
    /// 输入或请求没有匹配到任何已注册格式/profile。
    #[error("Unknown file format")]
    UnknownFormat,

    /// Export requested a concrete target/version pair that this profile cannot write.
    ///
    /// 导出请求的目标/version 组合不是该 profile 可写组合。
    #[error("Anycubic target {target} does not support version {version}. Supported versions: {supported}.")]
    UnsupportedFormatVersion {
        target: String,
        version: u32,
        supported: String,
    },

    /// A parser or writer needed more bytes than the provided buffer contains.
    ///
    /// parser 或 writer 需要的字节数超过了提供的 buffer。
    #[error("Buffer too small: need {need} bytes, got {got}")]
    BufferTooSmall { need: usize, got: usize },

    /// RLE payload could not be decoded at the reported byte offset.
    ///
    /// RLE payload 在指定 byte offset 处无法解码。
    #[error("RLE decode error at byte {offset}: {detail}")]
    RleDecodeError { offset: usize, detail: String },

    /// RLE encoder rejected the input pixels or options.
    ///
    /// RLE encoder 拒绝了输入像素或选项。
    #[error("RLE encode error: {0}")]
    RleEncodeError(String),

    /// PNG codec failed while reading or writing an image payload.
    ///
    /// PNG codec 读写图像 payload 时失败。
    #[error("PNG error: {0}")]
    PngError(String),

    /// ZIP/archive container operation failed.
    ///
    /// ZIP/archive 容器操作失败。
    #[error("ZIP error: {0}")]
    ZipError(String),

    /// INI-style metadata could not be parsed.
    ///
    /// INI 风格 metadata 解析失败。
    #[error("INI parse error: {0}")]
    IniError(String),

    /// Filesystem or stream I/O failed.
    ///
    /// 文件系统或 stream I/O 失败。
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// `binrw` failed while parsing or writing a binary structure.
    ///
    /// `binrw` 解析或写入二进制结构时失败。
    #[error("Binary parse error: {0}")]
    BinrwError(String),

    /// Anycubic export target could not be inferred from machine metadata.
    ///
    /// 无法从机器 metadata 推断 Anycubic 导出目标。
    #[error(
        "No Anycubic profile matches resolution {resolution_x}x{resolution_y} @ {display_width_mm}x{display_height_mm} mm"
    )]
    NoMatchingAnycubicProfile {
        resolution_x: u32,
        resolution_y: u32,
        display_width_mm: f32,
        display_height_mm: f32,
    },

    /// Anycubic machine metadata matches more than one profile.
    ///
    /// Anycubic 机器 metadata 同时匹配多个 profile。
    #[error("Anycubic profile match is ambiguous for resolution {resolution_x}x{resolution_y}: {candidates:?}")]
    AmbiguousAnycubicProfile {
        resolution_x: u32,
        resolution_y: u32,
        candidates: Vec<&'static str>,
    },

    /// Normalized metadata failed cross-field validation.
    ///
    /// 归一化 metadata 未通过跨字段校验。
    #[error("FileMetadata validation failed: {0}")]
    MetadataValidation(String),
}

impl From<binrw::Error> for ConvertError {
    fn from(e: binrw::Error) -> Self {
        ConvertError::BinrwError(e.to_string())
    }
}

impl ConvertError {
    /// Returns a stable machine-readable error code.
    ///
    /// 返回稳定的机器可读错误码。
    pub fn code(&self) -> &'static str {
        match self {
            ConvertError::InvalidMagic { .. } => "E_INVALID_MAGIC",
            ConvertError::UnsupportedVersion(_) => "E_UNSUPPORTED_VERSION",
            ConvertError::UnknownFormat => "E_UNKNOWN_FORMAT",
            ConvertError::UnsupportedFormatVersion { .. } => "E_UNSUPPORTED_FORMAT_VERSION",
            ConvertError::BufferTooSmall { .. } => "E_BUFFER_TOO_SMALL",
            ConvertError::RleDecodeError { .. } => "E_RLE_DECODE",
            ConvertError::RleEncodeError(_) => "E_RLE_ENCODE",
            ConvertError::PngError(_) => "E_PNG",
            ConvertError::ZipError(_) => "E_ZIP",
            ConvertError::IniError(_) => "E_INI",
            ConvertError::IoError(_) => "E_IO",
            ConvertError::BinrwError(_) => "E_BINRW",
            ConvertError::NoMatchingAnycubicProfile { .. } => "E_ANYCUBIC_PROFILE_NOT_FOUND",
            ConvertError::AmbiguousAnycubicProfile { .. } => "E_ANYCUBIC_PROFILE_AMBIGUOUS",
            ConvertError::MetadataValidation(_) => "E_METADATA_VALIDATION",
        }
    }

    /// Returns whether retrying the same operation is expected to help.
    ///
    /// 返回重试同一操作是否可能有帮助。当前转换错误均视为不可重试。
    pub fn retryable(&self) -> bool {
        false
    }

    /// Returns optional structured detail beyond the display message.
    ///
    /// 返回 display message 之外的可选结构化细节。
    pub fn detail(&self) -> Option<String> {
        match self {
            ConvertError::InvalidMagic { expected, actual } => {
                Some(format!("expected {expected:#010x}, got {actual:#010x}"))
            }
            ConvertError::UnsupportedVersion(version) => Some(version.to_string()),
            ConvertError::UnknownFormat => None,
            ConvertError::UnsupportedFormatVersion {
                target,
                version,
                supported,
            } => Some(format!("{target}:{version}:{supported}")),
            ConvertError::BufferTooSmall { need, got } => {
                Some(format!("need {need} bytes, got {got}"))
            }
            ConvertError::RleDecodeError { detail, .. } => Some(detail.clone()),
            ConvertError::RleEncodeError(detail)
            | ConvertError::PngError(detail)
            | ConvertError::ZipError(detail)
            | ConvertError::IniError(detail)
            | ConvertError::BinrwError(detail)
            | ConvertError::MetadataValidation(detail) => Some(detail.clone()),
            ConvertError::IoError(detail) => Some(detail.to_string()),
            ConvertError::NoMatchingAnycubicProfile {
                resolution_x,
                resolution_y,
                display_width_mm,
                display_height_mm,
            } => Some(format!(
                "{resolution_x}x{resolution_y} @ {display_width_mm}x{display_height_mm} mm"
            )),
            ConvertError::AmbiguousAnycubicProfile {
                resolution_x,
                resolution_y,
                candidates,
            } => Some(format!("{resolution_x}x{resolution_y}: {candidates:?}")),
        }
    }

    /// Converts this Rust error into the serializable JS/CLI payload.
    ///
    /// 将 Rust 错误转换为可序列化的 JS/CLI payload。
    pub fn to_structured(&self) -> StructuredConvertError {
        StructuredConvertError {
            code: self.code(),
            message: self.to_string(),
            retryable: self.retryable(),
            detail: self.detail(),
        }
    }
}
