use x86::{Image, ImageKind, Machine, MachineConfig, MachineStatus, SavedState};

fn minimal_state() -> Vec<u8> {
    let metadata = br#"{"state":[65536],"buffer_infos":[]}"#;
    let total = 16 + metadata.len();
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(&0x8676_8676u32.to_le_bytes());
    bytes.extend_from_slice(&6u32.to_le_bytes());
    bytes.extend_from_slice(&(total as u32).to_le_bytes());
    bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    bytes.extend_from_slice(metadata);
    bytes
}

#[test]
fn image_reports_size_and_sha256() {
    let image = Image::from_bytes(ImageKind::RawDisk, "disk", vec![1, 2, 3]);
    assert_eq!(image.len(), 3);
    assert_eq!(
        image.sha256(),
        "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81"
    );
}

#[test]
fn state_header_and_summary_are_available() {
    let state = SavedState::from_bytes(minimal_state()).expect("valid state");
    assert_eq!(state.header().version, 6);
    assert_eq!(state.buffer_count(), 0);
    assert_eq!(state.memory_bytes(), Some(65536));
    assert!(!state.is_compressed());
}

#[cfg(feature = "zstd")]
#[test]
fn zstd_state_is_decoded() {
    let compressed = zstd::stream::encode_all(minimal_state().as_slice(), 1).expect("compress");
    let state = SavedState::from_bytes(compressed).expect("valid compressed state");
    assert!(state.is_compressed());
    assert_eq!(state.header().version, 6);
}

#[test]
fn machine_starts_without_backend() {
    let machine = Machine::new(MachineConfig::default().with_ram_bytes(64 * 1024 * 1024));
    assert_eq!(machine.status(), MachineStatus::Created);
    assert_eq!(machine.config().ram_bytes, 64 * 1024 * 1024);
}
