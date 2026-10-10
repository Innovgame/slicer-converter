use crate::common::{ConvertError, PreviewLayerImage};

/// Runtime layer codec contract for Anycubic binary and archive profiles.
///
/// Different Anycubic families use different RLE payload encodings while sharing
/// the same higher-level format pipeline. Implementors expose decode/encode
/// operations plus a display name used in manifest/machine tables.
///
/// Anycubic binary/archive profiles 的运行期 layer codec 契约。不同 Anycubic family
/// 使用不同 RLE payload encoding，但共享上层 format pipeline；实现方暴露 encode/decode
/// 操作以及写入 manifest/machine table 的 codec name。
pub trait AnycubicLayerCodec: Sync {
    /// Return a stable codec identifier for diagnostics and catalog output.
    ///
    /// 返回用于 diagnostics 与 catalog output 的稳定 codec identifier。
    fn codec_name(&self) -> &'static str;

    /// Decode one encoded layer payload into grayscale pixels.
    ///
    /// 将单个 encoded layer payload 解码为 grayscale pixels。
    fn decode_layer(&self, data: &[u8]) -> Result<Vec<u8>, ConvertError>;

    /// Decode into a caller-provided buffer.
    ///
    /// Hot codecs override this to reuse capacity and avoid assigning a freshly
    /// allocated vector into `output`.
    ///
    /// 解码到调用方提供的 buffer。热路径 codec 会 override 该方法以复用 capacity，避免每层新建
    /// vector 后再赋值给 `output`。
    fn decode_layer_into(&self, data: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
        *output = self.decode_layer(data)?;
        Ok(())
    }

    /// Decode one layer payload directly into a browser-ready RGBA preview.
    ///
    /// Implementations may downsample while decoding to avoid materializing a
    /// full grayscale layer when only preview pixels are needed.
    ///
    /// 将单个 layer payload 直接解码为浏览器可用的 RGBA preview。实现可在 decode 过程中
    /// downsample，避免仅需 preview 时 materialize 完整 grayscale layer。
    fn decode_layer_preview_rgba(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        max_preview_size: u32,
    ) -> Result<PreviewLayerImage, ConvertError>;

    /// Encode grayscale pixels into this profile's layer payload format.
    ///
    /// 将 grayscale pixels 编码为该 profile 的 layer payload format。
    fn encode_layer(&self, pixels: &[u8]) -> Result<Vec<u8>, ConvertError>;

    /// Encode into a caller-provided buffer.
    ///
    /// Hot codecs override this to reuse capacity across layers.
    ///
    /// 编码到调用方提供的 buffer。热路径 codec 会 override 该方法，以便跨 layer 复用 capacity。
    fn encode_layer_into(&self, pixels: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
        *output = self.encode_layer(pixels)?;
        Ok(())
    }

    /// Return the layer image format string written into Anycubic metadata.
    ///
    /// 返回写入 Anycubic metadata 的 layer image format 字符串。
    fn layer_image_format(&self) -> &'static str;
}
