use crate::error::{Result, X86Error, io_error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageKind {
    RawDisk,
    Iso9660,
    Bios,
    VgaBios,
    Kernel,
    Initrd,
    Bootloader,
    SavedState,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    kind: ImageKind,
    name: String,
    bytes: Vec<u8>,
    source: Option<PathBuf>,
}

impl Image {
    pub fn from_bytes(kind: ImageKind, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            kind,
            name: name.into(),
            bytes: bytes.into(),
            source: None,
        }
    }

    pub fn from_file(kind: ImageKind, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
        Ok(Self {
            kind,
            name: path
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("image")
                .to_owned(),
            bytes,
            source: Some(path.to_path_buf()),
        })
    }

    pub fn kind(&self) -> ImageKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.bytes);
        hex::encode(hasher.finalize())
    }

    pub fn verify_sha256(&self, expected: &str) -> Result<()> {
        let expected = expected.trim().to_ascii_lowercase();
        let actual = self.sha256();
        if actual != expected {
            return Err(X86Error::InvalidImage(format!(
                "SHA-256 mismatch for {}: expected {}, got {}",
                self.name, expected, actual
            )));
        }
        Ok(())
    }

    pub fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, &self.bytes).map_err(|source| io_error(path, source))
    }
}
