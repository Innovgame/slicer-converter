//! Format-agnostic layer image and preview helpers.
//!
//! Full-size layers use row-major grayscale bytes. UI preview paths use
//! row-major RGBA bytes because that shape can cross the WASM boundary and
//! become browser `ImageData` without another color conversion step.
//!
//! 格式无关的 layer image 与 preview helper。全尺寸 layer 使用行优先灰度 bytes；
//! UI preview 路径使用行优先 RGBA bytes，这样跨 WASM 边界后可直接转为浏览器 `ImageData`，
//! 不需要再做一次颜色转换。

use serde::Serialize;

use super::Thumbnail;

/// Format-agnostic representation of a single print layer.
///
/// The pixel buffer must contain exactly `width * height` grayscale bytes in
/// row-major order.
///
/// 与格式无关的单层打印图像表示。像素 buffer 必须包含 `width * height` 个行优先灰度 bytes。
pub struct LayerImage {
    /// Row-major grayscale pixels (0 = unexposed, 255 = fully exposed).
    ///
    /// 行优先灰度像素，0 表示未曝光，255 表示完全曝光。
    pub pixels: Vec<u8>,
    /// Layer width in pixels.
    ///
    /// layer 像素宽度。
    pub width: u32,
    /// Layer height in pixels.
    ///
    /// layer 像素高度。
    pub height: u32,
}

impl LayerImage {
    /// Create a blank layer of the given dimensions.
    ///
    /// The layer is initialized to black/unexposed pixels.
    ///
    /// 创建指定尺寸的空白图层，像素初始化为黑色/未曝光。
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixels: vec![0u8; (width * height) as usize],
            width,
            height,
        }
    }

    /// Wrap an existing pixel buffer into a `LayerImage`.
    ///
    /// Debug builds assert that the buffer length matches `width * height`;
    /// release builds trust the caller for hot-path construction.
    ///
    /// 将已有像素缓冲区封装为 `LayerImage`。debug 模式会校验长度匹配 `width * height`；
    /// release 模式为热路径构造信任调用方。
    pub fn from_pixels(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        debug_assert_eq!(pixels.len(), (width * height) as usize);
        Self {
            pixels,
            width,
            height,
        }
    }

    /// Return the total number of pixels in this layer.
    ///
    /// 返回图层总像素数，即 `width * height`。
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
}

/// RGBA preview image returned to UI/WASM callers.
///
/// Pixels are row-major RGBA bytes and are ready to become browser `ImageData`.
///
/// 返回给 UI/WASM 调用方的 RGBA preview image。像素为行优先 RGBA bytes，可直接转成浏览器
/// `ImageData`。
#[derive(Debug, Clone, Serialize)]
pub struct PreviewLayerImage {
    /// Row-major RGBA pixels.
    ///
    /// 行优先 RGBA 像素。
    pub pixels: Vec<u8>,
    /// Preview width in pixels.
    ///
    /// preview 像素宽度。
    pub width: u32,
    /// Preview height in pixels.
    ///
    /// preview 像素高度。
    pub height: u32,
}

/// Precomputed nearest-neighbor sampling plan for direct preview decode paths.
///
/// Hot codecs use this to decode only the source pixels needed for a bounded
/// preview rather than materializing a full grayscale layer first.
///
/// direct preview decode 路径使用的 nearest-neighbor 采样计划。热点 codec 可借此只解码 bounded
/// preview 需要的源像素，而不先 materialize 完整灰度层。
#[derive(Debug, Clone)]
pub struct PreviewDecodePlan {
    target_indices: Vec<usize>,
    /// Destination preview width.
    ///
    /// 目标 preview 宽度。
    pub dst_width: u32,
    /// Destination preview height.
    ///
    /// 目标 preview 高度。
    pub dst_height: u32,
}

impl PreviewDecodePlan {
    /// Builds a downsample plan when the source exceeds `max_preview_size`.
    ///
    /// Returns `None` when no downsampling is needed or the requested max size is
    /// zero. The plan stores source indices for every destination pixel.
    ///
    /// 当源图超过 `max_preview_size` 时构建 downsample plan。无需 downsample 或 max size 为 0 时返回
    /// `None`；plan 会为每个目标像素保存源像素 index。
    pub fn for_downsample(src_width: u32, src_height: u32, max_preview_size: u32) -> Option<Self> {
        if max_preview_size == 0
            || (src_width <= max_preview_size && src_height <= max_preview_size)
        {
            return None;
        }

        let scale = f64::min(
            max_preview_size as f64 / src_width as f64,
            max_preview_size as f64 / src_height as f64,
        );
        let dst_width = ((src_width as f64) * scale).round().max(1.0) as u32;
        let dst_height = ((src_height as f64) * scale).round().max(1.0) as u32;

        let dst_pixels = (dst_width as usize) * (dst_height as usize);
        let mut target_indices = Vec::with_capacity(dst_pixels);
        for y in 0..dst_height {
            let src_y = ((y as f64) / scale).floor().min((src_height - 1) as f64) as u32;
            let src_row = (src_y as usize) * (src_width as usize);
            for x in 0..dst_width {
                let src_x = ((x as f64) / scale).floor().min((src_width - 1) as f64) as u32;
                target_indices.push(src_row + src_x as usize);
            }
        }

        Some(Self {
            target_indices,
            dst_width,
            dst_height,
        })
    }

    /// Returns source pixel indices for destination pixels in row-major order.
    ///
    /// 返回目标像素对应的源像素 indices，顺序为行优先。
    pub fn target_indices(&self) -> &[usize] {
        &self.target_indices
    }

    /// Returns the source pixel row sampled for a destination row.
    ///
    /// 返回目标 row 对应采样的源图 row。
    pub fn source_row_for_dst_y(&self, dst_y: u32, src_width: u32) -> Option<u32> {
        if dst_y >= self.dst_height || src_width == 0 {
            return None;
        }

        self.target_indices
            .get((dst_y as usize) * (self.dst_width as usize))
            .map(|index| (index / (src_width as usize)) as u32)
    }

    /// Returns the source pixel column sampled for a destination column.
    ///
    /// 返回目标 column 对应采样的源图 column。
    pub fn source_col_for_dst_x(&self, dst_x: u32, src_width: u32) -> Option<u32> {
        if dst_x >= self.dst_width || src_width == 0 {
            return None;
        }

        self.target_indices
            .get(dst_x as usize)
            .map(|index| (index % (src_width as usize)) as u32)
    }

    /// Returns byte capacity needed for an RGBA buffer using this plan.
    ///
    /// 返回该 plan 所需 RGBA buffer 的字节容量。
    pub fn rgba_capacity(&self) -> usize {
        self.target_indices.len() * 4
    }

    /// Creates a zeroed RGBA buffer with alpha prefilled to 255.
    ///
    /// 创建 alpha 预填为 255 的零值 RGBA buffer。
    pub fn new_rgba_buffer(&self) -> Vec<u8> {
        let mut rgba = vec![0u8; self.rgba_capacity()];
        for idx in 0..self.target_indices.len() {
            rgba[idx * 4 + 3] = 255;
        }
        rgba
    }
}

/// Downsample grayscale pixels to RGBA using nearest-neighbor sampling.
///
/// Each grayscale value `v` maps to RGBA `(v, v, v, 255)`.
/// Output is ready for direct use as `ImageData` in the browser.
///
/// 使用 nearest-neighbor 将灰度像素 downsample 为 RGBA。每个灰度值 `v` 映射为
/// `(v, v, v, 255)`，输出可直接用于浏览器 `ImageData`。
pub fn downsample_to_rgba_nearest(
    pixels: &[u8],
    src_width: u32,
    src_height: u32,
    max_preview_size: u32,
) -> PreviewLayerImage {
    if max_preview_size == 0 || (src_width <= max_preview_size && src_height <= max_preview_size) {
        let mut rgba = Vec::with_capacity(pixels.len() * 4);
        for &v in pixels {
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
        return PreviewLayerImage {
            pixels: rgba,
            width: src_width,
            height: src_height,
        };
    }

    let scale = f64::min(
        max_preview_size as f64 / src_width as f64,
        max_preview_size as f64 / src_height as f64,
    );
    let dst_width = ((src_width as f64) * scale).round().max(1.0) as u32;
    let dst_height = ((src_height as f64) * scale).round().max(1.0) as u32;
    let pixel_count = (dst_width * dst_height) as usize;
    let mut output = vec![0u8; pixel_count * 4];

    for y in 0..dst_height {
        let src_y = ((y as f64) / scale).floor().min((src_height - 1) as f64) as u32;
        let src_row = (src_y * src_width) as usize;
        let dst_row = (y * dst_width) as usize;
        for x in 0..dst_width {
            let src_x = ((x as f64) / scale).floor().min((src_width - 1) as f64) as u32;
            let v = pixels[src_row + src_x as usize];
            let out_idx = (dst_row + x as usize) * 4;
            output[out_idx] = v;
            output[out_idx + 1] = v;
            output[out_idx + 2] = v;
            output[out_idx + 3] = 255;
        }
    }

    PreviewLayerImage {
        pixels: output,
        width: dst_width,
        height: dst_height,
    }
}

/// Downsample grayscale pixels while preserving grayscale output.
///
/// This is used when callers want a smaller grayscale layer rather than browser
/// RGBA bytes.
///
/// 使用 nearest-neighbor downsample 灰度像素并保持灰度输出。适用于调用方需要较小灰度层、
/// 而不是浏览器 RGBA bytes 的场景。
pub fn downsample_grayscale_nearest(
    pixels: &[u8],
    src_width: u32,
    src_height: u32,
    max_preview_size: u32,
) -> PreviewLayerImage {
    if max_preview_size == 0 || (src_width <= max_preview_size && src_height <= max_preview_size) {
        return PreviewLayerImage {
            pixels: pixels.to_vec(),
            width: src_width,
            height: src_height,
        };
    }

    let scale = f64::min(
        max_preview_size as f64 / src_width as f64,
        max_preview_size as f64 / src_height as f64,
    );
    let dst_width = ((src_width as f64) * scale).round().max(1.0) as u32;
    let dst_height = ((src_height as f64) * scale).round().max(1.0) as u32;
    let mut output = vec![0u8; (dst_width * dst_height) as usize];

    for y in 0..dst_height {
        let src_y = ((y as f64) / scale).floor() as u32;
        let src_y = src_y.min(src_height - 1);
        let src_row = (src_y * src_width) as usize;
        let dst_row = (y * dst_width) as usize;
        for x in 0..dst_width {
            let src_x = ((x as f64) / scale).floor() as u32;
            let src_x = src_x.min(src_width - 1);
            output[dst_row + x as usize] = pixels[src_row + src_x as usize];
        }
    }

    PreviewLayerImage {
        pixels: output,
        width: dst_width,
        height: dst_height,
    }
}

/// Select an RGBA thumbnail at the requested size, resizing the closest match
/// with nearest-neighbor sampling when no exact thumbnail exists.
///
/// 选择请求尺寸的 RGBA thumbnail；没有精确尺寸时，用 nearest-neighbor resize 最接近的有效缩略图。
pub fn thumbnail_rgba_for_size(thumbnails: &[Thumbnail], target_w: u32, target_h: u32) -> Vec<u8> {
    if target_w == 0 || target_h == 0 {
        return Vec::new();
    }

    if let Some(thumb) = thumbnails
        .iter()
        .filter(|thumb| is_valid_rgba_thumbnail(thumb))
        .find(|t| t.width == target_w && t.height == target_h)
    {
        return thumb.pixels_rgba.clone();
    }

    if let Some(thumb) = thumbnails
        .iter()
        .filter(|thumb| is_valid_rgba_thumbnail(thumb))
        .min_by_key(|t| {
            let dw = (t.width as i64 - target_w as i64).unsigned_abs();
            let dh = (t.height as i64 - target_h as i64).unsigned_abs();
            dw + dh
        })
    {
        return resize_rgba_nearest(
            &thumb.pixels_rgba,
            thumb.width,
            thumb.height,
            target_w,
            target_h,
        );
    }

    Vec::new()
}

fn is_valid_rgba_thumbnail(thumbnail: &Thumbnail) -> bool {
    thumbnail.width > 0
        && thumbnail.height > 0
        && thumbnail.pixels_rgba.len()
            == (thumbnail.width as usize)
                .saturating_mul(thumbnail.height as usize)
                .saturating_mul(4)
}

fn resize_rgba_nearest(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dw * dh * 4) as usize];
    for dy in 0..dh {
        let sy = (dy * sh / dh).min(sh - 1);
        for dx in 0..dw {
            let sx = (dx * sw / dw).min(sw - 1);
            let si = (sy * sw + sx) as usize * 4;
            let di = (dy * dw + dx) as usize * 4;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_decode_plan_matches_generic_sampling_coordinates() {
        let src_width = 16;
        let src_height = 12;
        let max_preview_size = 6;
        let plan = PreviewDecodePlan::for_downsample(src_width, src_height, max_preview_size)
            .expect("downsample plan should exist");
        let pixels: Vec<u8> = (0..(src_width * src_height) as usize)
            .map(|index| (index % 251) as u8)
            .collect();
        let expected = downsample_to_rgba_nearest(&pixels, src_width, src_height, max_preview_size);

        let mut actual = plan.new_rgba_buffer();
        for (target_i, &src_index) in plan.target_indices().iter().enumerate() {
            let value = pixels[src_index];
            let out = target_i * 4;
            actual[out] = value;
            actual[out + 1] = value;
            actual[out + 2] = value;
        }

        assert_eq!(plan.dst_width, expected.width);
        assert_eq!(plan.dst_height, expected.height);
        assert_eq!(actual, expected.pixels);
    }

    #[test]
    fn thumbnail_rgba_for_size_uses_exact_thumbnail_without_resizing() {
        let exact = Thumbnail {
            width: 2,
            height: 1,
            pixels_rgba: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let fallback = Thumbnail {
            width: 1,
            height: 1,
            pixels_rgba: vec![9, 10, 11, 12],
        };

        assert_eq!(
            thumbnail_rgba_for_size(&[fallback, exact.clone()], 2, 1),
            exact.pixels_rgba
        );
    }

    #[test]
    fn thumbnail_rgba_for_size_resizes_closest_thumbnail_and_ignores_invalid_sources() {
        let invalid = Thumbnail {
            width: 4,
            height: 4,
            pixels_rgba: vec![255; 4],
        };
        let closest_valid = Thumbnail {
            width: 2,
            height: 2,
            pixels_rgba: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        };

        assert_eq!(
            thumbnail_rgba_for_size(&[invalid, closest_valid], 1, 1),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn thumbnail_rgba_for_size_ignores_invalid_exact_match_and_zero_sized_sources() {
        let invalid_exact = Thumbnail {
            width: 2,
            height: 1,
            pixels_rgba: vec![1, 2, 3, 4],
        };
        let zero_width = Thumbnail {
            width: 0,
            height: 1,
            pixels_rgba: Vec::new(),
        };
        let valid = Thumbnail {
            width: 1,
            height: 1,
            pixels_rgba: vec![5, 6, 7, 8],
        };

        assert_eq!(
            thumbnail_rgba_for_size(&[invalid_exact, zero_width, valid], 2, 1),
            vec![5, 6, 7, 8, 5, 6, 7, 8]
        );
    }

    #[test]
    fn thumbnail_rgba_for_size_returns_empty_for_zero_target_size() {
        let thumbnail = Thumbnail {
            width: 1,
            height: 1,
            pixels_rgba: vec![1, 2, 3, 4],
        };

        assert!(thumbnail_rgba_for_size(&[thumbnail], 0, 1).is_empty());
        assert!(thumbnail_rgba_for_size(&[], 1, 0).is_empty());
        assert!(thumbnail_rgba_for_size(&[], 1, 1).is_empty());
    }
}
