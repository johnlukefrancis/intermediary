// Path: crates/im_bundle/src/omission.rs
// Description: Why a changed repository path fell outside the bundle selection

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OmissionReason {
    Symlink,
    UnrepresentablePath,
    RootFilesNotSelected,
    TopLevelDirNotSelected(String),
    ExcludedFile,
    ExcludedSubdir(String),
    GlobalDirName(String),
    GlobalFileName(String),
    GlobalPathPattern,
    OutsideBundleRoot,
}

impl fmt::Display for OmissionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symlink => write!(f, "symlink; bundles never follow links"),
            Self::UnrepresentablePath => {
                write!(f, "path cannot be represented as an archive entry")
            }
            Self::RootFilesNotSelected => write!(f, "root-level files are not selected"),
            Self::TopLevelDirNotSelected(dir) => {
                write!(f, "top-level directory {dir} is not selected")
            }
            Self::ExcludedFile => write!(f, "excluded file (excludedFiles)"),
            Self::ExcludedSubdir(dir) => {
                write!(f, "excluded subdirectory {dir} (excludedSubdirs)")
            }
            Self::GlobalDirName(name) => write!(f, "global directory-name exclude {name}"),
            Self::GlobalFileName(name) => write!(f, "global file-name exclude {name}"),
            Self::GlobalPathPattern => write!(f, "global path-pattern exclude"),
            Self::OutsideBundleRoot => {
                write!(f, "outside the bundle root within this repository")
            }
        }
    }
}
