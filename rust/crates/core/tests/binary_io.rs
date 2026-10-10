use slicer_converter_core::{
    common::ConvertError,
    formats::common::binary::{
        put_be_f64, put_le_f32, put_le_u16, put_le_u32, read_be_f64, read_le_f32, read_le_u16,
        read_le_u32, read_range, BinaryReader, BinaryWriter,
    },
};

#[test]
fn binary_io_writes_offset_fields_without_resizing_target() {
    let mut data = [0u8; 16];

    put_le_u16(&mut data, 1, 0x1234).unwrap();
    put_le_u32(&mut data, 4, 0x89AB_CDEF).unwrap();
    put_le_f32(&mut data, 8, 1.25).unwrap();
    put_be_f64(&mut data, 8, 2.5).unwrap();

    assert_eq!(read_le_u16(&data, 1).unwrap(), 0x1234);
    assert_eq!(read_le_u32(&data, 4).unwrap(), 0x89AB_CDEF);
    assert_eq!(read_le_f32(&1.25f32.to_le_bytes(), 0).unwrap(), 1.25);
    assert_eq!(read_be_f64(&data, 8).unwrap(), 2.5);
    assert_eq!(data.len(), 16);
}

#[test]
fn binary_io_appends_directly_to_existing_buffer() {
    let mut data = Vec::with_capacity(64);
    let original_capacity = data.capacity();

    let mut writer = BinaryWriter::new(&mut data);
    writer.write_u8(0x7F);
    writer.write_bytes(b"AB");
    writer.write_le_u16(0x1234);
    writer.write_le_u32(0x89AB_CDEF);
    writer.write_le_u64(0x0123_4567_89AB_CDEF);
    writer.write_le_f32(1.25);
    writer.write_be_f32(2.5);
    writer.write_be_f64(3.5);

    assert_eq!(data.capacity(), original_capacity);
    assert_eq!(data.len(), 33);
    assert_eq!(&data[0..3], b"\x7FAB");
    assert_eq!(read_le_u16(&data, 3).unwrap(), 0x1234);
    assert_eq!(read_le_u32(&data, 5).unwrap(), 0x89AB_CDEF);
    assert_eq!(&data[9..17], &0x0123_4567_89AB_CDEFu64.to_le_bytes());
    assert_eq!(read_le_f32(&data, 17).unwrap(), 1.25);
    assert_eq!(&data[21..25], &2.5f32.to_be_bytes());
    assert_eq!(read_be_f64(&data, 25).unwrap(), 3.5);
}

#[test]
fn binary_io_returns_borrowed_ranges_and_checked_bounds() {
    let data = [1u8, 2, 3, 4];

    let borrowed = read_range(&data, 1, 2).unwrap();
    assert!(std::ptr::eq(borrowed.as_ptr(), data[1..].as_ptr()));
    assert_eq!(borrowed, &[2, 3]);

    let error = put_le_u32(&mut [0u8; 2], 0, 1).unwrap_err();
    assert!(matches!(
        error,
        ConvertError::BufferTooSmall { need: 4, got: 2 }
    ));
}

#[test]
fn binary_reader_advances_without_copying_ranges() {
    let data = [0x12, 0x34, 0x89, 0xAB, 0xCD, 0xEF, 0x3F, 0xA0, 0x00, 0x00];

    let mut reader = BinaryReader::new(&data);

    assert_eq!(reader.read_be_u16().unwrap(), 0x1234);
    assert_eq!(reader.read_be_u32().unwrap(), 0x89AB_CDEF);
    assert_eq!(reader.read_be_f32().unwrap(), 1.25);
    assert_eq!(reader.position(), data.len());

    reader.set_position(2).unwrap();
    let borrowed = reader.read_range(4).unwrap();
    assert!(std::ptr::eq(borrowed.as_ptr(), data[2..].as_ptr()));
    assert_eq!(borrowed, &[0x89, 0xAB, 0xCD, 0xEF]);
    assert_eq!(reader.position(), 6);
}

#[test]
fn binary_reader_reports_checked_bounds() {
    let mut reader = BinaryReader::new(&[0u8; 3]);

    let error = reader.read_be_u32().unwrap_err();

    assert!(matches!(
        error,
        ConvertError::BufferTooSmall { need: 4, got: 3 }
    ));
    assert_eq!(reader.position(), 0);
}
