//! Checked binary read/write helpers.
//!
//! These helpers are for hand-authored format layouts where using `binrw` would
//! add more machinery than value. All offset-based reads and writes return
//! `ConvertError::BufferTooSmall` instead of panicking.
//!
//! 带边界检查的二进制读写 helper。适用于手写格式 layout，尤其是使用 `binrw` 会显得过重的场景。
//! 所有基于 offset 的读写都会返回 `ConvertError::BufferTooSmall`，而不是 panic。

use crate::common::ConvertError;

/// Reads a checked byte range from a binary buffer.
///
/// 读取带边界检查的字节范围。
#[inline]
pub fn read_range(data: &[u8], offset: usize, len: usize) -> Result<&[u8], ConvertError> {
    let end = checked_end(data, offset, len)?;
    Ok(&data[offset..end])
}

/// Reads a little-endian `u16` from a checked offset.
///
/// 从带边界检查的 offset 读取 little-endian `u16`。
#[inline]
pub fn read_le_u16(data: &[u8], offset: usize) -> Result<u16, ConvertError> {
    read_array::<2>(data, offset).map(u16::from_le_bytes)
}

/// Reads a little-endian `u32` from a checked offset.
///
/// 从带边界检查的 offset 读取 little-endian `u32`。
#[inline]
pub fn read_le_u32(data: &[u8], offset: usize) -> Result<u32, ConvertError> {
    read_array::<4>(data, offset).map(u32::from_le_bytes)
}

/// Reads a little-endian `u64` from a checked offset.
///
/// 从带边界检查的 offset 读取 little-endian `u64`。
#[inline]
pub fn read_le_u64(data: &[u8], offset: usize) -> Result<u64, ConvertError> {
    read_array::<8>(data, offset).map(u64::from_le_bytes)
}

/// Reads a little-endian `f32` from a checked offset.
///
/// 从带边界检查的 offset 读取 little-endian `f32`。
#[inline]
pub fn read_le_f32(data: &[u8], offset: usize) -> Result<f32, ConvertError> {
    read_array::<4>(data, offset).map(f32::from_le_bytes)
}

/// Reads a big-endian `u16` from a checked offset.
///
/// 从带边界检查的 offset 读取 big-endian `u16`。
#[inline]
pub fn read_be_u16(data: &[u8], offset: usize) -> Result<u16, ConvertError> {
    read_array::<2>(data, offset).map(u16::from_be_bytes)
}

/// Reads a big-endian `u32` from a checked offset.
///
/// 从带边界检查的 offset 读取 big-endian `u32`。
#[inline]
pub fn read_be_u32(data: &[u8], offset: usize) -> Result<u32, ConvertError> {
    read_array::<4>(data, offset).map(u32::from_be_bytes)
}

/// Reads a big-endian `f32` from a checked offset.
///
/// 从带边界检查的 offset 读取 big-endian `f32`。
#[inline]
pub fn read_be_f32(data: &[u8], offset: usize) -> Result<f32, ConvertError> {
    read_array::<4>(data, offset).map(f32::from_be_bytes)
}

/// Reads a big-endian `f64` from a checked offset.
///
/// 从带边界检查的 offset 读取 big-endian `f64`。
#[inline]
pub fn read_be_f64(data: &[u8], offset: usize) -> Result<f64, ConvertError> {
    read_array::<8>(data, offset).map(f64::from_be_bytes)
}

/// Writes a little-endian `u16` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 little-endian `u16`。
#[inline]
pub fn put_le_u16(data: &mut [u8], offset: usize, value: u16) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_le_bytes())
}

/// Writes a little-endian `u32` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 little-endian `u32`。
#[inline]
pub fn put_le_u32(data: &mut [u8], offset: usize, value: u32) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_le_bytes())
}

/// Writes a little-endian `u64` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 little-endian `u64`。
#[inline]
pub fn put_le_u64(data: &mut [u8], offset: usize, value: u64) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_le_bytes())
}

/// Writes a little-endian `f32` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 little-endian `f32`。
#[inline]
pub fn put_le_f32(data: &mut [u8], offset: usize, value: f32) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_le_bytes())
}

/// Writes a big-endian `u16` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 big-endian `u16`。
#[inline]
pub fn put_be_u16(data: &mut [u8], offset: usize, value: u16) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_be_bytes())
}

/// Writes a big-endian `u32` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 big-endian `u32`。
#[inline]
pub fn put_be_u32(data: &mut [u8], offset: usize, value: u32) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_be_bytes())
}

/// Writes a big-endian `f64` into an existing checked buffer.
///
/// 向已有 buffer 的带边界检查 offset 写入 big-endian `f64`。
#[inline]
pub fn put_be_f64(data: &mut [u8], offset: usize, value: f64) -> Result<(), ConvertError> {
    put_bytes(data, offset, &value.to_be_bytes())
}

/// Append-only helper for writing small binary structures into a `Vec<u8>`.
///
/// This is useful for hand-authored container tables where `binrw` would be too
/// heavy or where fields are assembled conditionally.
///
/// 用于向 `Vec<u8>` 追加小型二进制结构的 helper。适合手写 container table，尤其是
/// `binrw` 过重或字段需要条件组装的场景。
pub struct BinaryWriter<'a> {
    output: &'a mut Vec<u8>,
}

/// Cursor-style checked reader over a binary slice.
///
/// All reads advance the internal offset and return `BufferTooSmall` instead of
/// panicking when the requested bytes are not available.
///
/// 基于 cursor 的二进制 slice 读取器。所有读取都会推进内部 offset；请求字节不足时返回
/// `BufferTooSmall`，而不是 panic。
pub struct BinaryReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    /// Creates a reader positioned at the start of `data`.
    ///
    /// 创建从 `data` 起点开始读取的 reader。
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Returns the current cursor offset.
    ///
    /// 返回当前 cursor offset。
    #[inline]
    pub fn position(&self) -> usize {
        self.offset
    }

    /// Moves the cursor to an absolute checked offset.
    ///
    /// 将 cursor 移动到带边界检查的绝对 offset。
    #[inline]
    pub fn set_position(&mut self, offset: usize) -> Result<(), ConvertError> {
        checked_end(self.data, offset, 0)?;
        self.offset = offset;
        Ok(())
    }

    /// Advances the cursor by `len` bytes after a bounds check.
    ///
    /// 经边界检查后将 cursor 向前推进 `len` bytes。
    #[inline]
    pub fn skip(&mut self, len: usize) -> Result<(), ConvertError> {
        let end = checked_end(self.data, self.offset, len)?;
        self.offset = end;
        Ok(())
    }

    /// Reads a borrowed range at the cursor and advances by `len`.
    ///
    /// 在 cursor 处读取借用 range，并向前推进 `len`。
    #[inline]
    pub fn read_range(&mut self, len: usize) -> Result<&'a [u8], ConvertError> {
        let end = checked_end(self.data, self.offset, len)?;
        let range = &self.data[self.offset..end];
        self.offset = end;
        Ok(range)
    }

    /// Reads exactly `N` bytes at the cursor.
    ///
    /// 在 cursor 处读取固定长度 `N` bytes。
    #[inline]
    pub fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], ConvertError> {
        let bytes = read_array::<N>(self.data, self.offset)?;
        self.offset += N;
        Ok(bytes)
    }

    /// Reads one byte at the cursor.
    ///
    /// 在 cursor 处读取一个 byte。
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, ConvertError> {
        checked_end(self.data, self.offset, 1)?;
        let value = self.data[self.offset];
        self.offset += 1;
        Ok(value)
    }

    /// Reads a little-endian `u16` at the cursor.
    ///
    /// 在 cursor 处读取 little-endian `u16`。
    #[inline]
    pub fn read_le_u16(&mut self) -> Result<u16, ConvertError> {
        self.read_bytes::<2>().map(u16::from_le_bytes)
    }

    /// Reads a little-endian `u32` at the cursor.
    ///
    /// 在 cursor 处读取 little-endian `u32`。
    #[inline]
    pub fn read_le_u32(&mut self) -> Result<u32, ConvertError> {
        self.read_bytes::<4>().map(u32::from_le_bytes)
    }

    /// Reads a little-endian `u64` at the cursor.
    ///
    /// 在 cursor 处读取 little-endian `u64`。
    #[inline]
    pub fn read_le_u64(&mut self) -> Result<u64, ConvertError> {
        self.read_bytes::<8>().map(u64::from_le_bytes)
    }

    /// Reads a little-endian `f32` at the cursor.
    ///
    /// 在 cursor 处读取 little-endian `f32`。
    #[inline]
    pub fn read_le_f32(&mut self) -> Result<f32, ConvertError> {
        self.read_bytes::<4>().map(f32::from_le_bytes)
    }

    /// Reads a big-endian `u16` at the cursor.
    ///
    /// 在 cursor 处读取 big-endian `u16`。
    #[inline]
    pub fn read_be_u16(&mut self) -> Result<u16, ConvertError> {
        self.read_bytes::<2>().map(u16::from_be_bytes)
    }

    /// Reads a big-endian `u32` at the cursor.
    ///
    /// 在 cursor 处读取 big-endian `u32`。
    #[inline]
    pub fn read_be_u32(&mut self) -> Result<u32, ConvertError> {
        self.read_bytes::<4>().map(u32::from_be_bytes)
    }

    /// Reads a big-endian `f32` at the cursor.
    ///
    /// 在 cursor 处读取 big-endian `f32`。
    #[inline]
    pub fn read_be_f32(&mut self) -> Result<f32, ConvertError> {
        self.read_bytes::<4>().map(f32::from_be_bytes)
    }

    /// Reads a big-endian `f64` at the cursor.
    ///
    /// 在 cursor 处读取 big-endian `f64`。
    #[inline]
    pub fn read_be_f64(&mut self) -> Result<f64, ConvertError> {
        self.read_bytes::<8>().map(f64::from_be_bytes)
    }
}

impl<'a> BinaryWriter<'a> {
    /// Wraps an existing vector as an append-only binary writer.
    ///
    /// 将已有 vector 包装成 append-only binary writer。
    #[inline]
    pub fn new(output: &'a mut Vec<u8>) -> Self {
        Self { output }
    }

    /// Appends one byte.
    ///
    /// 追加一个 byte。
    #[inline]
    pub fn write_u8(&mut self, value: u8) {
        self.output.push(value);
    }

    /// Appends raw bytes without interpretation.
    ///
    /// 不做解释地追加 raw bytes。
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
    }

    /// Appends a little-endian `u16`.
    ///
    /// 追加 little-endian `u16`。
    #[inline]
    pub fn write_le_u16(&mut self, value: u16) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    /// Appends a little-endian `u32`.
    ///
    /// 追加 little-endian `u32`。
    #[inline]
    pub fn write_le_u32(&mut self, value: u32) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    /// Appends a little-endian `u64`.
    ///
    /// 追加 little-endian `u64`。
    #[inline]
    pub fn write_le_u64(&mut self, value: u64) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    /// Appends a little-endian `f32`.
    ///
    /// 追加 little-endian `f32`。
    #[inline]
    pub fn write_le_f32(&mut self, value: f32) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    /// Appends a big-endian `u16`.
    ///
    /// 追加 big-endian `u16`。
    #[inline]
    pub fn write_be_u16(&mut self, value: u16) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a big-endian `u32`.
    ///
    /// 追加 big-endian `u32`。
    #[inline]
    pub fn write_be_u32(&mut self, value: u32) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a big-endian `f32`.
    ///
    /// 追加 big-endian `f32`。
    #[inline]
    pub fn write_be_f32(&mut self, value: f32) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a big-endian `f64`.
    ///
    /// 追加 big-endian `f64`。
    #[inline]
    pub fn write_be_f64(&mut self, value: f64) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }
}

#[inline]
fn read_array<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N], ConvertError> {
    let end = checked_end(data, offset, N)?;
    Ok(data[offset..end].try_into().expect("slice length checked"))
}

#[inline]
fn put_bytes(data: &mut [u8], offset: usize, bytes: &[u8]) -> Result<(), ConvertError> {
    let end = checked_end(data, offset, bytes.len())?;
    data[offset..end].copy_from_slice(bytes);
    Ok(())
}

#[inline]
fn checked_end(data: &[u8], offset: usize, len: usize) -> Result<usize, ConvertError> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: data.len(),
        })?;
    if end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: end,
            got: data.len(),
        });
    }
    Ok(end)
}
