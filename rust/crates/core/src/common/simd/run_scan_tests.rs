use super::run_scan::RunScanner;

#[test]
fn scan_u8_run_end_groups_exact_matching_bytes() {
    let pixels = [7u8, 7, 7, 8, 8, 9, 9, 9, 9, 10];
    let scanner = RunScanner::detect();

    assert_eq!(
        unsafe { scanner.scan_u8_run_end(pixels.as_ptr(), pixels.len(), 0, pixels[0]) },
        3
    );
    assert_eq!(
        unsafe { scanner.scan_u8_run_end(pixels.as_ptr(), pixels.len(), 3, pixels[3]) },
        5
    );
    assert_eq!(
        unsafe { scanner.scan_u8_run_end(pixels.as_ptr(), pixels.len(), 5, pixels[5]) },
        9
    );
    assert_eq!(
        unsafe { scanner.scan_u8_run_end(pixels.as_ptr(), pixels.len(), 9, pixels[9]) },
        10
    );
}
