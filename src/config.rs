//! Library loading policy: which directories backend libraries may be
//! loaded from. There is deliberately no default search path -- an
//! unconfigured Provider is `Unavailable` with a diagnostic, never a
//! search of `PATH`, `LD_LIBRARY_PATH` or the working directory.

use std::{
    fmt,
    path::{Path, PathBuf},
};

/// Environment variable listing trusted QNN library directories, separated
/// like `PATH` (`:` on Unix, `;` on Windows).
pub const LIBRARY_DIR_ENV: &str = "MAGNETAR_QUALCOMM_LIB_DIR";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyViolation {
    NotAbsolute,
    NotADirectory,
    /// Unix only: a group- or world-writable directory could be used to
    /// plant a library.
    WritableByOthers,
}

impl fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAbsolute => "library directory must be an absolute path",
            Self::NotADirectory => "library directory does not exist or is not a directory",
            Self::WritableByOthers => "library directory is writable by group or others",
        })
    }
}

/// Trusted-path policy for loading QNN backend libraries.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoadPolicy {
    library_dirs: Vec<PathBuf>,
}

impl LoadPolicy {
    /// A policy that trusts exactly these directories, in order.
    pub fn with_library_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            library_dirs: dirs.into_iter().collect(),
        }
    }

    /// Reads [`LIBRARY_DIR_ENV`]. Unset or empty means no trusted
    /// directory, i.e. every backend is reported unavailable.
    pub fn from_env() -> Self {
        match std::env::var_os(LIBRARY_DIR_ENV) {
            Some(value) => Self::with_library_dirs(std::env::split_paths(&value)),
            None => Self::default(),
        }
    }

    pub fn library_dirs(&self) -> &[PathBuf] {
        &self.library_dirs
    }

    /// Checks one configured directory against the policy.
    pub fn check_dir(dir: &Path) -> Result<(), PolicyViolation> {
        if !dir.is_absolute() {
            return Err(PolicyViolation::NotAbsolute);
        }
        let metadata = std::fs::metadata(dir).map_err(|_| PolicyViolation::NotADirectory)?;
        if !metadata.is_dir() {
            return Err(PolicyViolation::NotADirectory);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o022 != 0 {
                return Err(PolicyViolation::WritableByOthers);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_missing_and_non_directory_paths() {
        assert_eq!(
            LoadPolicy::check_dir(Path::new("relative/dir")),
            Err(PolicyViolation::NotAbsolute)
        );
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            LoadPolicy::check_dir(&dir.path().join("missing")),
            Err(PolicyViolation::NotADirectory)
        );
        let file = dir.path().join("file");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(
            LoadPolicy::check_dir(&file),
            Err(PolicyViolation::NotADirectory)
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_group_or_world_writable_directories() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        for mode in [0o700, 0o755] {
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(LoadPolicy::check_dir(dir.path()), Ok(()));
        }
        for mode in [0o770, 0o777, 0o702] {
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(
                LoadPolicy::check_dir(dir.path()),
                Err(PolicyViolation::WritableByOthers)
            );
        }
        // restore so tempdir cleanup works everywhere
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}
