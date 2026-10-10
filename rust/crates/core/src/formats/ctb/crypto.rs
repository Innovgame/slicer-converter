//! CTB layer payload XOR encryption/decryption.
//!
//! CTB encryption is applied to the encoded layer RLE bytes, not to decoded
//! pixels. The algorithm is symmetric (encrypt == decrypt): use
//! `header.encryption_key` as `seed` and the zero-based layer number as
//! `layer_index`.
//!
//! CTB 加密作用在 encoded layer RLE bytes 上，而不是 decoded pixels 上。算法是对称的，
//! `seed` 来自 `header.encryption_key`，`layer_index` 为 0-based layer number。
//!
//! ```text
//! init = seed.wrapping_mul(0x2d83cdac).wrapping_add(0xd8a83423)
//! key  = (layer_index.wrapping_mul(0x1e1530cd).wrapping_add(0xec3d47cd)).wrapping_mul(init)
//!
//! for i in 0..data.len():
//!     shift = (i & 3) * 8
//!     k = (key >> shift) & 0xFF
//!     output[i] = data[i] ^ k
//!     if (i + 1) & 3 == 0:
//!         key = key.wrapping_add(init)
//! ```

/// Encrypt or decrypt CTB layer RLE bytes in-place.
///
/// Passing `seed = 0` leaves the buffer unchanged, matching unencrypted CTB
/// files. The layer index participates in key generation, so callers must use
/// the file-order layer number when decoding or encoding.
///
/// 就地加密或解密 CTB layer RLE bytes。`seed = 0` 表示未加密并保持 buffer 不变；
/// layer index 会参与 key generation，因此调用方必须传入文件顺序中的 0-based layer number。
#[inline]
pub fn crypt_layer_data(data: &mut [u8], seed: u32, layer_index: u32) {
    if seed == 0 {
        return; // no encryption
    }

    let init = seed.wrapping_mul(0x2d83cdac).wrapping_add(0xd8a83423);
    let mut key = layer_index
        .wrapping_mul(0x1e1530cd)
        .wrapping_add(0xec3d47cd)
        .wrapping_mul(init);

    for (i, byte) in data.iter_mut().enumerate() {
        let shift = ((i & 3) * 8) as u32;
        let k = ((key >> shift) & 0xFF) as u8;
        *byte ^= k;
        if (i + 1) & 3 == 0 {
            key = key.wrapping_add(init);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypt_roundtrip() {
        let original = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let mut encrypted = original.clone();
        crypt_layer_data(&mut encrypted, 0xDEAD_BEEF, 5);
        // encrypted should differ from original (non-zero seed)
        assert_ne!(encrypted, original);

        // decrypt with same seed/layer
        let mut decrypted = encrypted.clone();
        crypt_layer_data(&mut decrypted, 0xDEAD_BEEF, 5);
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_crypt_zero_seed_noop() {
        let original = vec![0x12, 0x34, 0x56, 0x78];
        let mut data = original.clone();
        crypt_layer_data(&mut data, 0, 0);
        assert_eq!(data, original);
    }

    #[test]
    fn test_crypt_different_layers() {
        let original = vec![0xAA; 16];
        let mut enc_layer0 = original.clone();
        let mut enc_layer1 = original.clone();
        crypt_layer_data(&mut enc_layer0, 42, 0);
        crypt_layer_data(&mut enc_layer1, 42, 1);
        // Different layers produce different ciphertext
        assert_ne!(enc_layer0, enc_layer1);
    }
}
