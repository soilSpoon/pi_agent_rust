//! Session Store V2 segmented append log + sidecar index primitives.
//!
//! This module provides the storage core requested by Phase-2 performance work:
//! - Segment append writer
//! - Sidecar offset index rows
//! - Reader helpers
//! - Integrity validation (checksum + payload hash)

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const SEGMENT_FRAME_SCHEMA: &str = "pi.session_store_v2.segment_frame.v1";
pub const OFFSET_INDEX_SCHEMA: &str = "pi.session_store_v2.offset_index.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentFrame {
    pub schema: String,
    pub segment_seq: u64,
    pub frame_seq: u64,
    pub entry_seq: u64,
    pub entry_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_entry_id: Option<String>,
    pub entry_type: String,
    pub timestamp: String,
    pub payload_sha256: String,
    pub payload_bytes: u64,
    pub payload: Value,
}

impl SegmentFrame {
    fn new(
        segment_seq: u64,
        frame_seq: u64,
        entry_seq: u64,
        entry_id: String,
        parent_entry_id: Option<String>,
        entry_type: String,
        payload: Value,
    ) -> Result<Self> {
        let (payload_sha256, payload_bytes) = payload_hash_and_size(&payload)?;
        Ok(Self {
            schema: SEGMENT_FRAME_SCHEMA.to_string(),
            segment_seq,
            frame_seq,
            entry_seq,
            entry_id,
            parent_entry_id,
            entry_type,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            payload_sha256,
            payload_bytes,
            payload,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OffsetIndexEntry {
    pub schema: String,
    pub entry_seq: u64,
    pub entry_id: String,
    pub segment_seq: u64,
    pub frame_seq: u64,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub crc32c: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct SessionStoreV2 {
    root: PathBuf,
    max_segment_bytes: u64,
    next_segment_seq: u64,
    next_frame_seq: u64,
    next_entry_seq: u64,
    current_segment_bytes: u64,
}

impl SessionStoreV2 {
    pub fn create(root: impl AsRef<Path>, max_segment_bytes: u64) -> Result<Self> {
        if max_segment_bytes == 0 {
            return Err(Error::validation("max_segment_bytes must be > 0"));
        }

        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("segments"))?;
        fs::create_dir_all(root.join("index"))?;

        let mut store = Self {
            root,
            max_segment_bytes,
            next_segment_seq: 1,
            next_frame_seq: 1,
            next_entry_seq: 1,
            current_segment_bytes: 0,
        };
        store.bootstrap_from_disk()?;
        Ok(store)
    }

    pub fn segment_file_path(&self, segment_seq: u64) -> PathBuf {
        self.root
            .join("segments")
            .join(format!("{segment_seq:016}.seg"))
    }

    pub fn index_file_path(&self) -> PathBuf {
        self.root.join("index").join("offsets.jsonl")
    }

    pub fn append_entry(
        &mut self,
        entry_id: impl Into<String>,
        parent_entry_id: Option<String>,
        entry_type: impl Into<String>,
        payload: Value,
    ) -> Result<OffsetIndexEntry> {
        let entry_id = entry_id.into();
        let entry_type = entry_type.into();

        let mut frame = SegmentFrame::new(
            self.next_segment_seq,
            self.next_frame_seq,
            self.next_entry_seq,
            entry_id,
            parent_entry_id,
            entry_type,
            payload,
        )?;
        let mut encoded = serde_json::to_vec(&frame)?;
        let mut line_len = line_length_u64(&encoded)?;

        if self.current_segment_bytes > 0
            && self.current_segment_bytes.saturating_add(line_len) > self.max_segment_bytes
        {
            self.next_segment_seq = self
                .next_segment_seq
                .checked_add(1)
                .ok_or_else(|| Error::session("segment sequence overflow"))?;
            self.next_frame_seq = 1;
            self.current_segment_bytes = 0;

            frame = SegmentFrame::new(
                self.next_segment_seq,
                self.next_frame_seq,
                self.next_entry_seq,
                frame.entry_id.clone(),
                frame.parent_entry_id.clone(),
                frame.entry_type.clone(),
                frame.payload.clone(),
            )?;
            encoded = serde_json::to_vec(&frame)?;
            line_len = line_length_u64(&encoded)?;
        }

        let segment_path = self.segment_file_path(self.next_segment_seq);
        let byte_offset = fs::metadata(&segment_path).map_or(0, |meta| meta.len());

        let mut segment = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&segment_path)?;
        segment.write_all(&encoded)?;
        segment.write_all(b"\n")?;
        segment.flush()?;

        let mut persisted = encoded;
        persisted.push(b'\n');
        let index_entry = OffsetIndexEntry {
            schema: OFFSET_INDEX_SCHEMA.to_string(),
            entry_seq: frame.entry_seq,
            entry_id: frame.entry_id.clone(),
            segment_seq: frame.segment_seq,
            frame_seq: frame.frame_seq,
            byte_offset,
            byte_length: line_len,
            crc32c: crc32c_upper(&persisted),
            state: "active".to_string(),
        };
        append_jsonl_line(&self.index_file_path(), &index_entry)?;

        self.next_entry_seq = self
            .next_entry_seq
            .checked_add(1)
            .ok_or_else(|| Error::session("entry sequence overflow"))?;
        self.next_frame_seq = self
            .next_frame_seq
            .checked_add(1)
            .ok_or_else(|| Error::session("frame sequence overflow"))?;
        self.current_segment_bytes = self.current_segment_bytes.saturating_add(line_len);

        Ok(index_entry)
    }

    pub fn read_segment(&self, segment_seq: u64) -> Result<Vec<SegmentFrame>> {
        let path = self.segment_file_path(segment_seq);
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_jsonl::<SegmentFrame>(&path)
    }

    pub fn read_index(&self) -> Result<Vec<OffsetIndexEntry>> {
        let path = self.index_file_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_jsonl::<OffsetIndexEntry>(&path)
    }

    pub fn validate_integrity(&self) -> Result<()> {
        let index_rows = self.read_index()?;
        let mut last_entry_seq = 0;

        for row in index_rows {
            if row.entry_seq <= last_entry_seq {
                return Err(Error::session(format!(
                    "entry sequence is not strictly increasing at entry_seq={}",
                    row.entry_seq
                )));
            }
            last_entry_seq = row.entry_seq;

            let segment_path = self.segment_file_path(row.segment_seq);
            let segment_len =
                fs::metadata(&segment_path)
                    .map(|meta| meta.len())
                    .map_err(|err| {
                        Error::session(format!(
                            "failed to stat segment {}: {err}",
                            segment_path.display()
                        ))
                    })?;
            let end = row
                .byte_offset
                .checked_add(row.byte_length)
                .ok_or_else(|| Error::session("index byte range overflow"))?;
            if end > segment_len {
                return Err(Error::session(format!(
                    "index out of bounds for segment {}: end={} len={segment_len}",
                    segment_path.display(),
                    end
                )));
            }

            let mut file = File::open(&segment_path)?;
            file.seek(SeekFrom::Start(row.byte_offset))?;
            let mut record_bytes = vec![
                0u8;
                usize::try_from(row.byte_length).map_err(|_| {
                    Error::session(format!("byte length too large: {}", row.byte_length))
                })?
            ];
            file.read_exact(&mut record_bytes)?;

            let checksum = crc32c_upper(&record_bytes);
            if checksum != row.crc32c {
                return Err(Error::session(format!(
                    "checksum mismatch for entry_seq={} expected={} actual={checksum}",
                    row.entry_seq, row.crc32c
                )));
            }

            if record_bytes.last() == Some(&b'\n') {
                record_bytes.pop();
            }
            let frame: SegmentFrame = serde_json::from_slice(&record_bytes)?;

            if frame.entry_seq != row.entry_seq
                || frame.entry_id != row.entry_id
                || frame.segment_seq != row.segment_seq
                || frame.frame_seq != row.frame_seq
            {
                return Err(Error::session(format!(
                    "index/frame mismatch at entry_seq={}",
                    row.entry_seq
                )));
            }

            let (payload_hash, payload_bytes) = payload_hash_and_size(&frame.payload)?;
            if frame.payload_sha256 != payload_hash || frame.payload_bytes != payload_bytes {
                return Err(Error::session(format!(
                    "payload integrity mismatch at entry_seq={}",
                    row.entry_seq
                )));
            }
        }

        Ok(())
    }

    fn bootstrap_from_disk(&mut self) -> Result<()> {
        let index_rows = self.read_index()?;
        if let Some(last) = index_rows.last() {
            self.next_entry_seq = last
                .entry_seq
                .checked_add(1)
                .ok_or_else(|| Error::session("entry sequence overflow while bootstrapping"))?;
            self.next_segment_seq = last.segment_seq;
            self.next_frame_seq = last
                .frame_seq
                .checked_add(1)
                .ok_or_else(|| Error::session("frame sequence overflow while bootstrapping"))?;
            let segment_path = self.segment_file_path(last.segment_seq);
            self.current_segment_bytes = fs::metadata(&segment_path)
                .map(|meta| meta.len())
                .map_err(|err| {
                    Error::session(format!(
                        "failed to stat active segment {} while bootstrapping: {err}",
                        segment_path.display()
                    ))
                })?;
        }
        Ok(())
    }
}

fn append_jsonl_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str::<T>(&line)?);
    }
    Ok(out)
}

fn payload_hash_and_size(payload: &Value) -> Result<(String, u64)> {
    let bytes = serde_json::to_vec(payload)?;
    let payload_bytes = u64::try_from(bytes.len())
        .map_err(|_| Error::session(format!("payload is too large: {} bytes", bytes.len())))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    Ok((hash, payload_bytes))
}

fn line_length_u64(encoded: &[u8]) -> Result<u64> {
    let line_len = encoded
        .len()
        .checked_add(1)
        .ok_or_else(|| Error::session("line length overflow"))?;
    u64::try_from(line_len).map_err(|_| Error::session("line length exceeds u64"))
}

fn crc32c_upper(data: &[u8]) -> String {
    const POLY: u32 = 0x82f6_3b78;
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let lsb_set = crc & 1;
            crc >>= 1;
            if lsb_set != 0 {
                crc ^= POLY;
            }
        }
    }
    crc = !crc;

    let mut out = String::with_capacity(8);
    let _ = write!(&mut out, "{crc:08X}");
    out
}
