#[test]
fn test_list_codecs() {
    let codecs = vidformer::codecs();
    assert!(!codecs.is_empty());

    let found_h264 = codecs.iter().find(|c| c.name == "h264");
    let found_h264 = found_h264.unwrap();
    assert!(found_h264.has_decoder && found_h264.has_encoder);

    let found_vp9 = codecs.iter().find(|c| c.name == "vp9");
    let found_vp9 = found_vp9.unwrap();
    assert!(found_vp9.has_decoder && found_vp9.has_encoder);

    let found_vp8 = codecs.iter().find(|c| c.name == "vp8");
    let found_vp8 = found_vp8.unwrap();
    assert!(found_vp8.has_decoder && found_vp8.has_encoder);
}
