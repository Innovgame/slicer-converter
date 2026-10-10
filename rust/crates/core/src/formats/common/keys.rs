//! Stable keys used to describe format-kernel implementation facts.
//!
//! These keys are not user-facing labels. They are compact, comparable facts
//! used by route selection, architecture tests, and Rust/TypeScript drift checks
//! to say which family, container, header, layer codec, pixel model, crypto,
//! footer, and archive implementation a profile is wired to.
//!
//! 这些 key 不是展示给用户看的名称，而是稳定的实现事实。路由选择、架构测试以及 Rust/TypeScript
//! 漂移检查会用它们描述某个 profile 绑定到哪个 family、container、header、layer codec、
//! pixel model、crypto、footer 和 archive 实现。

/// Stable identifier for a format-kernel implementation fact.
///
/// The inner string is intentionally static and human-readable so test failures
/// and generated contracts stay easy to inspect.
///
/// 格式内核实现事实的稳定标识。内部字符串保持静态且可读，便于检查测试失败和生成的契约文件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormatKey(pub &'static str);

// Format family keys.
pub const FAMILY_CHITUBOX: FormatKey = FormatKey("family/chitubox");
pub const FAMILY_ANYCUBIC: FormatKey = FormatKey("family/anycubic");
pub const FAMILY_GOO: FormatKey = FormatKey("family/goo");
pub const FAMILY_PHROZEN: FormatKey = FormatKey("family/phrozen");
pub const FAMILY_SL1: FormatKey = FormatKey("family/sl1");
pub const FAMILY_VDT: FormatKey = FormatKey("family/vdt");
pub const FAMILY_CREALITY: FormatKey = FormatKey("family/creality");
pub const FAMILY_OTHER: FormatKey = FormatKey("family/other");

// Outer container keys.
pub const CONTAINER_BINARY: FormatKey = FormatKey("container/binary");
pub const CONTAINER_ARCHIVE: FormatKey = FormatKey("container/archive");

// Header/layout keys. Archive formats use these for manifest/config semantics
// even when there is no fixed binary header block.
pub const HEADER_CHITU_CTB_V1: FormatKey = FormatKey("header/chitu-ctb-v1");
pub const HEADER_CHITU_CTB_V2: FormatKey = FormatKey("header/chitu-ctb-v2");
pub const HEADER_CHITU_CTB_V3: FormatKey = FormatKey("header/chitu-ctb-v3");
pub const HEADER_CHITU_CTB_V4: FormatKey = FormatKey("header/chitu-ctb-v4");
pub const HEADER_CHITU_CTB_V5: FormatKey = FormatKey("header/chitu-ctb-v5");
pub const HEADER_CHITU_LEGACY: FormatKey = FormatKey("header/chitu-legacy");
pub const HEADER_CHITU_PHZ: FormatKey = FormatKey("header/chitu-phz");
pub const HEADER_CHITU_FDG: FormatKey = FormatKey("header/chitu-fdg");
pub const HEADER_CHITU_PHOTONS: FormatKey = FormatKey("header/chitu-photons");
pub const HEADER_CHITU_CTB_ENCRYPTED: FormatKey = FormatKey("header/chitu-ctb-encrypted");
pub const HEADER_CHITUBOX_ZIP: FormatKey = FormatKey("header/chitubox-zip");
pub const HEADER_ANYCUBIC_V1: FormatKey = FormatKey("header/anycubic-v1");
pub const HEADER_ANYCUBIC_V516: FormatKey = FormatKey("header/anycubic-v516");
pub const HEADER_ANYCUBIC_V517: FormatKey = FormatKey("header/anycubic-v517");
pub const HEADER_ANYCUBIC_V518: FormatKey = FormatKey("header/anycubic-v518");
pub const HEADER_ANYCUBIC_ZIP: FormatKey = FormatKey("header/anycubic-zip");
pub const HEADER_GOO_FAMILY: FormatKey = FormatKey("header/goo-family");
pub const HEADER_SL1_ARCHIVE: FormatKey = FormatKey("header/sl1-archive");
pub const HEADER_VDT_ARCHIVE: FormatKey = FormatKey("header/vdt-archive");
pub const HEADER_CREALITY_CXDLP: FormatKey = FormatKey("header/creality-cxdlp");
pub const HEADER_CREALITY_CXDLP_V4: FormatKey = FormatKey("header/creality-cxdlp-v4");
pub const GENERIC_HEADER_FALLBACK: FormatKey = FormatKey("header/generic-fallback");

// Layer payload codec keys.
pub const CTB_RLE: FormatKey = FormatKey("layer/ctb-rle");
pub const CBDDLP_BITPLANE: FormatKey = FormatKey("layer/cbddlp-bitplane");
pub const PHZ_RLE: FormatKey = FormatKey("layer/phz-rle");
pub const FDG_RLE: FormatKey = FormatKey("layer/fdg-rle");
pub const PHOTONS_RLE: FormatKey = FormatKey("layer/photons-rle");
pub const CTB_ENCRYPTED_RLE: FormatKey = FormatKey("layer/chitu-ctb-encrypted-rle");
pub const GOO_RLE: FormatKey = FormatKey("layer/goo-rle");
pub const ANYCUBIC_PW0_RLE: FormatKey = FormatKey("layer/anycubic-pw0-rle");
pub const ANYCUBIC_PWS_RLE: FormatKey = FormatKey("layer/anycubic-pws-rle");
pub const ANYCUBIC_ZIP_RLE: FormatKey = FormatKey("layer/anycubic-zip-rle");
pub const PNG_GRAYSCALE: FormatKey = FormatKey("layer/png-grayscale");
pub const CREALITY_CXDLP_LINES: FormatKey = FormatKey("layer/creality-cxdlp-lines");
pub const CREALITY_CXDLP_RLE: FormatKey = FormatKey("layer/creality-cxdlp-rle");
pub const GENERIC_LAYER_FALLBACK: FormatKey = FormatKey("layer/generic-fallback");

// Decoded pixel model keys.
pub const PIXEL_GRAYSCALE_8: FormatKey = FormatKey("pixel/grayscale-8");
pub const PIXEL_BITPLANE_AA: FormatKey = FormatKey("pixel/bitplane-aa");
pub const PIXEL_PNG_GRAYSCALE: FormatKey = FormatKey("pixel/png-grayscale");

// Per-layer crypto and final footer policy keys.
pub const NO_CRYPTO: FormatKey = FormatKey("crypto/none");
pub const CHITU_XOR: FormatKey = FormatKey("crypto/chitu-xor");
pub const PHZ_XOR: FormatKey = FormatKey("crypto/phz-xor");
pub const FDG_XOR: FormatKey = FormatKey("crypto/fdg-xor");
pub const CHITU_AES_BLOCK: FormatKey = FormatKey("crypto/chitu-aes-block");
pub const NO_FOOTER: FormatKey = FormatKey("footer/none");
pub const GOO_FAMILY_FOOTER: FormatKey = FormatKey("footer/goo-family");
pub const GENERIC_FOOTER_FALLBACK: FormatKey = FormatKey("footer/generic-fallback");

// Archive layout keys.
pub const NO_ARCHIVE: FormatKey = FormatKey("archive/none");
pub const ZIP_ARCHIVE: FormatKey = FormatKey("archive/zip");
