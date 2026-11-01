use std::{error::Error, fmt, fs, io::Read, path::Path};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareFingerprint {
    pub architecture: String,
    pub operating_system: String,
    pub logical_cpus: usize,
}

impl HardwareFingerprint {
    pub fn current() -> Self {
        Self {
            architecture: std::env::consts::ARCH.to_string(),
            operating_system: std::env::consts::OS.to_string(),
            logical_cpus: std::thread::available_parallelism()
                .map(|value| value.get())
                .unwrap_or(1),
        }
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareProfileSource {
    FreshCalibration,
    PersistentProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareProfileStatus {
    Missing,
    Loaded,
    Incompatible,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfileMetadata {
    pub format_version: u32,
    pub backend: String,
    pub fingerprint: HardwareFingerprint,
    pub source: HardwareProfileSource,
}

impl HardwareProfileMetadata {
    pub const CURRENT_FORMAT_VERSION: u32 = 1;

    pub fn new(
        backend: impl Into<String>,
        fingerprint: HardwareFingerprint,
        source: HardwareProfileSource,
    ) -> Self {
        Self {
            format_version: Self::CURRENT_FORMAT_VERSION,
            backend: backend.into(),
            fingerprint,
            source,
        }
    }

    pub fn is_current_version(&self) -> bool {
        self.format_version == Self::CURRENT_FORMAT_VERSION
    }

    pub fn is_compatible(&self, backend: &str, fingerprint: &HardwareFingerprint) -> bool {
        self.is_current_version()
            && self.backend == backend
            && self.fingerprint.is_compatible_with(fingerprint)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfileFile<T> {
    pub metadata: HardwareProfileMetadata,
    pub payload: T,
}

impl<T> HardwareProfileFile<T> {
    pub fn new(metadata: HardwareProfileMetadata, payload: T) -> Self {
        Self { metadata, payload }
    }
}

fn write_profile_staged(path: &Path, contents: &[u8]) -> Result<(), std::io::Error> {
    use std::io::{ErrorKind, Write};
    use std::time::{SystemTime, UNIX_EPOCH};

    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "profile destination must be a regular file",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    let parent = path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "missing profile name"))?
        .to_string_lossy();

    let mut staged = None;
    for counter in 0..16 {
        let candidate = parent.join(format!(
            ".{stem}.fhec-{}-{stamp}-{counter}.tmp",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                staged = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    let (stage_path, mut file) = staged.ok_or_else(|| {
        std::io::Error::new(
            ErrorKind::AlreadyExists,
            "cannot allocate profile staging file",
        )
    })?;
    let write_result = file.write_all(contents).and_then(|_| file.sync_all());
    drop(file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&stage_path);
        return Err(error);
    }

    #[cfg(windows)]
    let replace_result = replace_profile_windows(&stage_path, path);
    #[cfg(not(windows))]
    let replace_result = fs::rename(&stage_path, path);

    if replace_result.is_err() {
        let _ = fs::remove_file(&stage_path);
    }
    replace_result
}

#[cfg(windows)]
fn replace_profile_windows(stage: &Path, destination: &Path) -> Result<(), std::io::Error> {
    use std::io::ErrorKind;

    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == ErrorKind::NotFound => return fs::rename(stage, destination),
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "profile destination changed during save",
            ));
        }
        Ok(_) => {}
        Err(error) => return Err(error),
    }

    let backup = stage.with_extension("backup");
    if backup.exists() {
        return Err(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "profile backup destination already exists",
        ));
    }
    fs::rename(destination, &backup)?;
    match fs::rename(stage, destination) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => match fs::rename(&backup, destination) {
            Ok(()) => Err(error),
            Err(restore_error) => Err(std::io::Error::new(
                error.kind(),
                format!(
                    "profile replacement failed: {error}; recovery failed: {restore_error}; backup retained at {}",
                    backup.display()
                ),
            )),
        },
    }
}

impl<T> HardwareProfileFile<T>
where
    T: Serialize,
{
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), HardwareProfileIoError> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let json = serde_json::to_string_pretty(self)?;

        write_profile_staged(path, json.as_bytes())?;

        Ok(())
    }
}

impl<T> HardwareProfileFile<T>
where
    T: DeserializeOwned,
{
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, HardwareProfileIoError> {
        const MAX_PROFILE_BYTES: u64 = 1_048_576;

        let file = fs::File::open(path)?;
        let mut reader = file.take(MAX_PROFILE_BYTES + 1);
        let mut json = Vec::new();
        reader.read_to_end(&mut json)?;

        if json.len() as u64 > MAX_PROFILE_BYTES {
            return Err(HardwareProfileIoError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "hardware profile exceeds the 1 MiB size limit",
            )));
        }

        Ok(serde_json::from_slice(&json)?)
    }
}

#[derive(Debug)]
pub enum HardwareProfileIoError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for HardwareProfileIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => {
                write!(formatter, "hardware profile I/O error: {error}")
            }
            Self::Json(error) => {
                write!(formatter, "hardware profile JSON error: {error}")
            }
        }
    }
}

impl Error for HardwareProfileIoError {}

impl From<std::io::Error> for HardwareProfileIoError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for HardwareProfileIoError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde::{Deserialize, Serialize};

    use super::{
        HardwareFingerprint, HardwareProfileFile, HardwareProfileMetadata, HardwareProfileSource,
    };

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        latency_ns: f64,
        ciphertext_bytes: usize,
    }

    fn test_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!("fhec-{name}-{}-{unique}.json", std::process::id()))
    }

    #[test]
    fn current_fingerprint_has_valid_cpu_count() {
        let fingerprint = HardwareFingerprint::current();

        assert!(!fingerprint.architecture.is_empty());
        assert!(!fingerprint.operating_system.is_empty());
        assert!(fingerprint.logical_cpus >= 1);
    }

    #[test]
    fn identical_fingerprints_are_compatible() {
        let fingerprint = HardwareFingerprint {
            architecture: "x86_64".to_string(),
            operating_system: "windows".to_string(),
            logical_cpus: 16,
        };

        assert!(fingerprint.is_compatible_with(&fingerprint));
    }

    #[test]
    fn different_cpu_counts_are_not_compatible() {
        let first = HardwareFingerprint {
            architecture: "x86_64".to_string(),
            operating_system: "windows".to_string(),
            logical_cpus: 16,
        };

        let second = HardwareFingerprint {
            architecture: "x86_64".to_string(),
            operating_system: "windows".to_string(),
            logical_cpus: 8,
        };

        assert!(!first.is_compatible_with(&second));
    }

    #[test]
    fn metadata_accepts_matching_hardware() {
        let fingerprint = HardwareFingerprint {
            architecture: "x86_64".to_string(),
            operating_system: "windows".to_string(),
            logical_cpus: 16,
        };

        let metadata = HardwareProfileMetadata::new(
            "tfhe",
            fingerprint.clone(),
            HardwareProfileSource::PersistentProfile,
        );

        assert!(metadata.is_compatible("tfhe", &fingerprint));
    }

    #[test]
    fn metadata_rejects_different_backend() {
        let fingerprint = HardwareFingerprint {
            architecture: "x86_64".to_string(),
            operating_system: "windows".to_string(),
            logical_cpus: 16,
        };

        let metadata = HardwareProfileMetadata::new(
            "tfhe",
            fingerprint.clone(),
            HardwareProfileSource::PersistentProfile,
        );

        assert!(!metadata.is_compatible("openfhe", &fingerprint));
    }

    #[test]
    fn hardware_profile_round_trips_through_json() {
        let path = test_path("round-trip");

        let fingerprint = HardwareFingerprint {
            architecture: "x86_64".to_string(),
            operating_system: "windows".to_string(),
            logical_cpus: 16,
        };

        let metadata = HardwareProfileMetadata::new(
            "tfhe",
            fingerprint,
            HardwareProfileSource::FreshCalibration,
        );

        let profile = HardwareProfileFile::new(
            metadata,
            TestPayload {
                latency_ns: 123_456.0,
                ciphertext_bytes: 3260,
            },
        );

        profile.save_json(&path).unwrap();

        let loaded = HardwareProfileFile::<TestPayload>::load_json(&path).unwrap();

        assert_eq!(loaded, profile);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_json_is_rejected() {
        let path = test_path("invalid-json");

        fs::write(&path, "{ definitely not valid json").unwrap();

        let result = HardwareProfileFile::<TestPayload>::load_json(&path);

        assert!(result.is_err());

        fs::remove_file(path).unwrap();
    }
}
