use crate::error::{Result, X86Error, io_error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const STATE_MAGIC: u32 = 0x8676_8676;
pub const STATE_VERSION: u32 = 6;
pub const STATE_HEADER_LEN: usize = 16;
pub const ZSTD_MAGIC: u32 = 0xFD2F_B528;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateHeader {
    pub magic: u32,
    pub version: u32,
    pub total_length: u32,
    pub metadata_length: u32,
}

#[derive(Debug, Clone)]
pub struct SavedState {
    encoded: Vec<u8>,
    decoded: Vec<u8>,
    metadata: Value,
    header: StateHeader,
    compressed: bool,
    source: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateSummary {
    pub compressed: bool,
    pub encoded_bytes: usize,
    pub decoded_bytes: usize,
    pub version: u32,
    pub buffer_count: usize,
    pub memory_bytes: Option<u64>,
    pub sha256: String,
    pub source: Option<PathBuf>,
}

impl SavedState {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let encoded = bytes.into();
        let (decoded, compressed) = decode(&encoded)?;
        let header = parse_header(&decoded)?;
        let metadata_end = STATE_HEADER_LEN + header.metadata_length as usize;
        let metadata: Value = serde_json::from_slice(&decoded[STATE_HEADER_LEN..metadata_end])?;
        if !metadata.get("state").is_some() || !metadata.get("buffer_infos").is_some() {
            return Err(X86Error::InvalidState(
                "metadata must contain state and buffer_infos".to_owned(),
            ));
        }
        Ok(Self {
            encoded,
            decoded,
            metadata,
            header,
            compressed,
            source: None,
        })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
        let mut state = Self::from_bytes(bytes)?;
        state.source = Some(path.to_path_buf());
        Ok(state)
    }

    pub fn header(&self) -> StateHeader {
        self.header
    }

    pub fn metadata(&self) -> &Value {
        &self.metadata
    }

    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded
    }

    pub fn decoded_bytes(&self) -> &[u8] {
        &self.decoded
    }

    pub fn is_compressed(&self) -> bool {
        self.compressed
    }

    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    pub fn buffer_count(&self) -> usize {
        self.metadata
            .get("buffer_infos")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    }

    pub fn memory_bytes(&self) -> Option<u64> {
        self.metadata
            .get("state")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_u64)
    }

    /// Return the raw v86 state array and typed buffers in buffer_id order.
    /// The buffers are copied out of the decoded state so callers can safely
    /// hand them to a native backend with its own lifetime.
    pub fn cpu_state_and_buffers(&self) -> Result<(Value, Vec<Vec<u8>>)> {
        let state = self
            .metadata
            .get("state")
            .cloned()
            .ok_or_else(|| X86Error::InvalidState("missing state array".to_owned()))?;
        let infos = self
            .metadata
            .get("buffer_infos")
            .and_then(Value::as_array)
            .ok_or_else(|| X86Error::InvalidState("missing buffer_infos array".to_owned()))?;
        let buffer_block_start = (STATE_HEADER_LEN + self.header.metadata_length as usize + 3) & !3;
        let mut buffers = Vec::with_capacity(infos.len());
        for (index, info) in infos.iter().enumerate() {
            let offset = info
                .get("offset")
                .and_then(Value::as_u64)
                .ok_or_else(|| X86Error::InvalidState(format!("buffer_infos[{index}] has no offset")))? as usize;
            let length = info
                .get("length")
                .and_then(Value::as_u64)
                .ok_or_else(|| X86Error::InvalidState(format!("buffer_infos[{index}] has no length")))? as usize;
            let start = buffer_block_start
                .checked_add(offset)
                .ok_or_else(|| X86Error::InvalidState("buffer offset overflow".to_owned()))?;
            let end = start
                .checked_add(length)
                .ok_or_else(|| X86Error::InvalidState("buffer length overflow".to_owned()))?;
            let bytes = self
                .decoded
                .get(start..end)
                .ok_or_else(|| X86Error::InvalidState(format!("buffer {index} exceeds decoded state")))?;
            buffers.push(bytes.to_vec());
        }
        Ok((state, buffers))
    }

    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.decoded);
        hex::encode(hasher.finalize())
    }

    pub fn summary(&self) -> StateSummary {
        StateSummary {
            compressed: self.compressed,
            encoded_bytes: self.encoded.len(),
            decoded_bytes: self.decoded.len(),
            version: self.header.version,
            buffer_count: self.buffer_count(),
            memory_bytes: self.memory_bytes(),
            sha256: self.sha256(),
            source: self.source.clone(),
        }
    }

    pub fn write_decoded(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, &self.decoded).map_err(|source| io_error(path, source))
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| X86Error::InvalidState("truncated header".to_owned()))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

pub fn parse_header(bytes: &[u8]) -> Result<StateHeader> {
    if bytes.len() < STATE_HEADER_LEN {
        return Err(X86Error::InvalidState(
            "state is shorter than header".to_owned(),
        ));
    }
    let header = StateHeader {
        magic: read_u32(bytes, 0)?,
        version: read_u32(bytes, 4)?,
        total_length: read_u32(bytes, 8)?,
        metadata_length: read_u32(bytes, 12)?,
    };
    if header.magic != STATE_MAGIC {
        return Err(X86Error::InvalidState(format!(
            "invalid magic 0x{:08x}",
            header.magic
        )));
    }
    if header.version != STATE_VERSION {
        return Err(X86Error::UnsupportedFormat(format!(
            "saved state version {}; supported version is {}",
            header.version, STATE_VERSION
        )));
    }
    let metadata_end = STATE_HEADER_LEN
        .checked_add(header.metadata_length as usize)
        .ok_or_else(|| X86Error::InvalidState("metadata length overflow".to_owned()))?;
    if metadata_end > bytes.len() {
        return Err(X86Error::InvalidState(
            "metadata exceeds state length".to_owned(),
        ));
    }
    Ok(header)
}

pub fn decode(bytes: &[u8]) -> Result<(Vec<u8>, bool)> {
    let compressed =
        bytes.len() >= 4 && u32::from_le_bytes(bytes[0..4].try_into().unwrap()) == ZSTD_MAGIC;
    if compressed {
        #[cfg(feature = "zstd")]
        {
            let decoded = zstd::stream::decode_all(bytes)
                .map_err(|error| X86Error::InvalidState(format!("zstd decode failed: {error}")))?;
            return Ok((decoded, true));
        }
        #[cfg(not(feature = "zstd"))]
        {
            return Err(X86Error::UnsupportedFormat(
                "zstd support is disabled; enable the `zstd` feature".to_owned(),
            ));
        }
    }
    Ok((bytes.to_vec(), false))
}
