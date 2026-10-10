use crate::common::ConvertError;

use super::{LayerDecodeContext, LayerEncodeContext};

/// Per-layer encryption policy for a concrete format profile.
///
/// The policy is applied outside `LayerCodec` so codecs can operate on plain
/// payload bytes. No-op formats use `NoCrypto`.
///
/// 具体格式 profile 的逐层加密策略。该策略在 `LayerCodec` 外层执行，使 codec 始终处理
/// 明文 payload bytes；无加密格式使用 `NoCrypto`。
pub trait CryptoPolicy: Sync {
    /// Decrypt an encoded layer payload before it is handed to `LayerCodec`.
    ///
    /// Implementations mutate the archive/header-provided bytes in place. The
    /// layer index and decode context are both available because several
    /// Chitubox-family algorithms derive their key stream from layer position
    /// and file metadata.
    ///
    /// 在 payload 交给 `LayerCodec` 之前进行逐层解密。实现会原地修改来自 archive/header 的
    /// encoded bytes；提供 layer index 与 decode context 是因为部分 Chitubox-family 算法会从层序号
    /// 和文件 metadata 派生 key stream。
    fn decrypt_layer_in_place(
        &self,
        data: &mut [u8],
        layer_index: u32,
        ctx: &LayerDecodeContext,
    ) -> Result<(), ConvertError>;

    /// Encrypt an encoded layer payload after `LayerCodec` has produced it.
    ///
    /// The codec should emit plain encoded bytes first; encryption is the last
    /// per-layer transform before the binary/archive layout records the payload.
    ///
    /// 在 `LayerCodec` 产出 payload 之后进行逐层加密。codec 应先生成明文 encoded bytes；
    /// encryption 是 binary/archive layout 记录 payload 前的最后一层转换。
    fn encrypt_layer_in_place(
        &self,
        data: &mut [u8],
        layer_index: u32,
        ctx: &LayerEncodeContext<'_>,
    ) -> Result<(), ConvertError>;
}

/// No-op crypto policy for plain formats.
///
/// 明文格式使用的空加密策略。
pub struct NoCrypto;

impl CryptoPolicy for NoCrypto {
    fn decrypt_layer_in_place(
        &self,
        _data: &mut [u8],
        _layer_index: u32,
        _ctx: &LayerDecodeContext,
    ) -> Result<(), ConvertError> {
        Ok(())
    }

    fn encrypt_layer_in_place(
        &self,
        _data: &mut [u8],
        _layer_index: u32,
        _ctx: &LayerEncodeContext<'_>,
    ) -> Result<(), ConvertError> {
        Ok(())
    }
}
