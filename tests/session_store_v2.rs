#![forbid(unsafe_code)]

use pi::PiResult;
use pi::session_store_v2::SessionStoreV2;
use serde_json::json;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use tempfile::tempdir;

#[test]
fn segmented_append_and_index_round_trip() -> PiResult<()> {
    let dir = tempdir()?;
    let mut store = SessionStoreV2::create(dir.path(), 4 * 1024)?;

    store.append_entry(
        "entry_00000001",
        None,
        "message",
        json!({"role":"user","text":"a"}),
    )?;
    store.append_entry(
        "entry_00000002",
        Some("entry_00000001".to_string()),
        "message",
        json!({"role":"assistant","text":"b"}),
    )?;

    let index = store.read_index()?;
    assert_eq!(index.len(), 2);
    assert_eq!(index[0].entry_seq, 1);
    assert_eq!(index[1].entry_seq, 2);

    let segment_one = store.read_segment(1)?;
    assert_eq!(segment_one.len(), 2);
    assert_eq!(segment_one[0].entry_id, "entry_00000001");
    assert_eq!(segment_one[1].entry_id, "entry_00000002");

    store.validate_integrity()?;
    Ok(())
}

#[test]
fn rotates_segment_when_threshold_is_hit() -> PiResult<()> {
    let dir = tempdir()?;
    let mut store = SessionStoreV2::create(dir.path(), 220)?;
    let payload = json!({
        "kind": "message",
        "text": "x".repeat(180)
    });

    store.append_entry("entry_00000001", None, "message", payload.clone())?;
    store.append_entry("entry_00000002", None, "message", payload)?;

    let index = store.read_index()?;
    assert_eq!(index.len(), 2);
    assert!(index[1].segment_seq > index[0].segment_seq);
    Ok(())
}

#[test]
fn append_path_preserves_prior_bytes_prefix() -> PiResult<()> {
    let dir = tempdir()?;
    let mut store = SessionStoreV2::create(dir.path(), 4 * 1024)?;

    let first = store.append_entry(
        "entry_00000001",
        None,
        "message",
        json!({"kind":"message","text":"first"}),
    )?;
    let first_segment = store.segment_file_path(first.segment_seq);
    let before = fs::read(&first_segment)?;

    store.append_entry(
        "entry_00000002",
        Some("entry_00000001".to_string()),
        "message",
        json!({"kind":"message","text":"second"}),
    )?;
    let after = fs::read(&first_segment)?;

    assert!(
        after.starts_with(&before),
        "append should preserve existing segment prefix bytes"
    );
    Ok(())
}

#[test]
fn corruption_is_detected_from_indexed_checksum() -> PiResult<()> {
    let dir = tempdir()?;
    let mut store = SessionStoreV2::create(dir.path(), 4 * 1024)?;

    let row = store.append_entry("entry_00000001", None, "message", json!({"text":"hello"}))?;
    let segment_path = store.segment_file_path(row.segment_seq);

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&segment_path)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(b"[")?;
    file.flush()?;

    let err = store
        .validate_integrity()
        .expect_err("checksum mismatch should be detected");
    assert!(
        err.to_string().contains("checksum mismatch"),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
fn bootstrap_fails_if_index_points_to_missing_segment() -> PiResult<()> {
    let dir = tempdir()?;
    let mut store = SessionStoreV2::create(dir.path(), 4 * 1024)?;
    let row = store.append_entry("entry_00000001", None, "message", json!({"text":"hello"}))?;

    let segment_path = store.segment_file_path(row.segment_seq);
    fs::remove_file(&segment_path)?;

    let err = SessionStoreV2::create(dir.path(), 4 * 1024)
        .expect_err("bootstrap should fail when active segment is missing");
    assert!(
        err.to_string().contains("failed to stat active segment"),
        "unexpected error: {err}"
    );
    Ok(())
}
