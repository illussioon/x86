use crate::error::{Result, X86Error, io_error};
use crate::image::{Image, ImageKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Resource {
    File(PathBuf),
    Url(String),
    Bytes { name: String, bytes: Vec<u8> },
}

impl Resource {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    pub fn url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }

    pub fn bytes(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self::Bytes {
            name: name.into(),
            bytes: bytes.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub timeout: Duration,
    pub user_agent: String,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            user_agent: format!("x86-native/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bootloader {
    pub image: Image,
    pub source: Resource,
}

impl Bootloader {
    pub fn load(source: Resource) -> Result<Self> {
        Self::load_with_options(source, &FetchOptions::default())
    }

    pub fn load_with_options(source: Resource, options: &FetchOptions) -> Result<Self> {
        let image = load_resource(&source, ImageKind::Bootloader, options)?;
        Ok(Self { image, source })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::load(Resource::file(path.as_ref().to_path_buf()))
    }

    #[cfg(feature = "remote")]
    pub fn from_url(url: impl Into<String>) -> Result<Self> {
        Self::load(Resource::url(url))
    }
}

pub fn load_resource(source: &Resource, kind: ImageKind, options: &FetchOptions) -> Result<Image> {
    match source {
        Resource::File(path) => Image::from_file(kind, path),
        Resource::Bytes { name, bytes } => Ok(Image::from_bytes(kind, name.clone(), bytes.clone())),
        Resource::Url(url) => load_url(url, kind, options),
    }
}

#[cfg(feature = "remote")]
fn load_url(url: &str, kind: ImageKind, options: &FetchOptions) -> Result<Image> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(X86Error::Remote {
            url: url.to_owned(),
            message: "only http:// and https:// URLs are supported".to_owned(),
        });
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(options.timeout)
        .user_agent(&options.user_agent)
        .build();
    let response = agent.get(url).call().map_err(|error| X86Error::Remote {
        url: url.to_owned(),
        message: error.to_string(),
    })?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes).map_err(|source| io_error(url, source))?;
    let name = url
        .rsplit('/')
        .next()
        .filter(|x| !x.is_empty())
        .unwrap_or("remote-bootloader");
    Ok(Image::from_bytes(kind, name, bytes))
}

#[cfg(not(feature = "remote"))]
fn load_url(url: &str, _kind: ImageKind, _options: &FetchOptions) -> Result<Image> {
    let _ = url;
    Err(X86Error::RemoteDisabled)
}

pub fn copy_resource_to(source: &Resource, destination: impl AsRef<Path>) -> Result<PathBuf> {
    let destination = destination.as_ref();
    let image = load_resource(source, ImageKind::Other, &FetchOptions::default())?;
    fs::write(destination, image.bytes()).map_err(|source| io_error(destination, source))?;
    Ok(destination.to_path_buf())
}
