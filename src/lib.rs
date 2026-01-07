mod author;
mod blob;
mod doc;
mod endpoint;
mod error;
mod gossip;
mod key;
mod net;
mod node;
mod tag;
mod ticket;

pub use self::author::*;
pub use self::blob::*;
pub use self::doc::*;
pub use self::endpoint::*;
pub use self::error::*;
pub use self::gossip::*;
pub use self::key::*;
pub use self::net::*;
pub use self::node::*;
pub use self::tag::*;
pub use self::ticket::*;

use tracing_subscriber::filter::LevelFilter;

// This macro includes the scaffolding for the Iroh FFI bindings.
uniffi::setup_scaffolding!();

/// The logging level. See the rust (log crate)[https://docs.rs/log] for more information.
#[derive(Debug, uniffi::Enum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> LevelFilter {
        match level {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Off => LevelFilter::OFF,
        }
    }
}

/// Set the logging level.
#[uniffi::export]
pub fn set_log_level(level: LogLevel) {
    use tracing_subscriber::{fmt, prelude::*, reload};
    let filter: LevelFilter = level.into();
    let (filter, _) = reload::Layer::new(filter);
    let mut layer = fmt::Layer::default();
    layer.set_ansi(false);
    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();
}

/// Helper function that translates a key that was derived from the [`path_to_key`] function back
/// into a path.
///
/// If `prefix` exists, it will be stripped before converting back to a path
/// If `root` exists, will add the root as a parent to the created path
/// Removes any null byte that has been appended to the key
#[uniffi::export]
pub fn key_to_path(
    key: Vec<u8>,
    prefix: Option<String>,
    root: Option<String>,
) -> Result<String, IrohError> {
    // Remove trailing null byte if present
    let key = if key.last() == Some(&0) {
        &key[..key.len() - 1]
    } else {
        &key
    };

    let key_str = std::str::from_utf8(key)
        .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in key: {}", e))?;

    // Strip prefix if present
    let path_str = if let Some(ref prefix) = prefix {
        key_str.strip_prefix(prefix.as_str()).unwrap_or(key_str)
    } else {
        key_str
    };

    // Add root as parent if present
    let path = if let Some(ref root) = root {
        std::path::PathBuf::from(root).join(path_str.trim_start_matches('/'))
    } else {
        std::path::PathBuf::from(path_str)
    };

    let path = path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Unable to convert path to string"))?
        .to_string();
    Ok(path)
}

/// Helper function that creates a document key from a canonicalized path, removing the `root` and adding the `prefix`, if they exist
///
/// Appends the null byte to the end of the key.
#[uniffi::export]
pub fn path_to_key(
    path: String,
    prefix: Option<String>,
    root: Option<String>,
) -> Result<Vec<u8>, IrohError> {
    let path = std::path::PathBuf::from(&path);

    // Strip root from path if present
    let path_str = if let Some(ref root) = root {
        let root_path = std::path::PathBuf::from(root);
        path.strip_prefix(&root_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string())
    } else {
        path.to_string_lossy().to_string()
    };

    // Add prefix if present
    let key_str = if let Some(ref prefix) = prefix {
        format!("{}{}", prefix, path_str)
    } else {
        path_str
    };

    // Append null byte
    let mut key = key_str.into_bytes();
    key.push(0);
    Ok(key)
}

#[cfg(test)]
fn setup_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_path_to_key_roundtrip() {
        let path = std::path::PathBuf::from("/").join("foo").join("bar");
        let path = path.to_str().unwrap().to_string();
        let mut key = b"/foo/bar\0".to_vec();

        let got_key = path_to_key(path.clone(), None, None).unwrap();
        assert_eq!(key, got_key);
        let got_path = key_to_path(got_key.clone(), None, None).unwrap();
        assert_eq!(path, got_path);

        // including prefix
        let prefix = String::from("prefix:");
        key = b"prefix:/foo/bar\0".to_vec();

        let got_key = path_to_key(path.clone(), Some(prefix.clone()), None).unwrap();
        assert_eq!(key, got_key);
        let got_path = key_to_path(got_key.clone(), Some(prefix.clone()), None).unwrap();
        assert_eq!(path, got_path);

        // including root
        let root = std::path::PathBuf::from("/").join("foo");
        let root = root.to_str().unwrap().to_string();
        key = b"prefix:bar\0".to_vec();

        let got_key = path_to_key(path.clone(), Some(prefix.clone()), Some(root.clone())).unwrap();
        assert_eq!(key, got_key);
        let got_path =
            key_to_path(got_key.clone(), Some(prefix.clone()), Some(root.clone())).unwrap();
        assert_eq!(path, got_path);
    }
}
