use programmer_core::MarkerDetector;

#[test]
fn matches_across_chunk_boundary() {
    let mut detector = MarkerDetector::new("APP_READY").unwrap();
    assert!(!detector.feed(b"boot APP_"));
    assert!(detector.feed(b"READY\r\n"));
    assert!(detector.matched());
}

#[test]
fn ignores_binary_noise() {
    let mut detector = MarkerDetector::new("OK").unwrap();
    assert!(!detector.feed(&[0xFF, 0x00, b'O']));
    assert!(detector.feed(b"K"));
}

#[test]
fn rejects_empty_marker() {
    assert!(MarkerDetector::new([]).is_err());
}
