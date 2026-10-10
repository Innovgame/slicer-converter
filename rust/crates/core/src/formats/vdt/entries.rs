//! VDT ZIP layer entry naming helpers.
//!
//! Current exports write numeric PNG names such as `0.png`; imports also accept
//! the older `layer_0000.png` style found in legacy archives.
//!
//! VDT ZIP layer entry naming helpers。当前导出写入 `0.png` 这类 numeric PNG names；
//! 导入也接受旧 archive 中出现过的 `layer_0000.png` 风格。

/// Return the canonical VDT layer PNG entry name for a zero-based layer index.
///
/// 返回 0-based layer index 对应的 canonical VDT layer PNG entry name。
pub(super) fn layer_entry_name(layer_index: u32) -> String {
    format!("{layer_index}.png")
}

/// Extract a zero-based layer index from a VDT ZIP entry name.
///
/// Preview entries are ignored, and both current numeric names and legacy
/// `layer_`-prefixed names are accepted.
///
/// 从 VDT ZIP entry name 提取 0-based layer index。preview entries 会被忽略；当前 numeric
/// names 与旧版 `layer_` 前缀命名都会被接受。
pub(super) fn layer_entry_index(name: &str) -> Option<u32> {
    let file_name = name.rsplit('/').next().unwrap_or(name);
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".png") || lower.contains("preview") {
        return None;
    }

    let stem = lower.strip_suffix(".png")?;
    if stem.chars().all(|ch| ch.is_ascii_digit()) {
        return stem.parse().ok();
    }

    stem.strip_prefix("layer_")
        .filter(|suffix| suffix.chars().all(|ch| ch.is_ascii_digit()))
        .and_then(|suffix| suffix.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{layer_entry_index, layer_entry_name};

    #[test]
    fn layer_entry_name_uses_numeric_png_names() {
        assert_eq!(layer_entry_name(0), "0.png");
        assert_eq!(layer_entry_name(12), "12.png");
    }

    #[test]
    fn layer_entry_index_accepts_current_and_legacy_vdt_names() {
        assert_eq!(layer_entry_index("0.png"), Some(0));
        assert_eq!(layer_entry_index("layers/12.png"), Some(12));
        assert_eq!(layer_entry_index("layer_0007.png"), Some(7));
        assert_eq!(layer_entry_index("Layer_0008.PNG"), Some(8));
        assert_eq!(layer_entry_index("preview.png"), None);
        assert_eq!(layer_entry_index("not-a-layer.png"), None);
    }
}
