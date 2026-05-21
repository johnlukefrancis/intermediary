// Path: crates/im_bundle/src/global_excludes_summary.rs
// Description: Manifest-facing normalized summary for bundle global excludes

use crate::global_excludes::normalize_global_excludes;
use crate::plan::GlobalExcludes;

pub fn normalized_global_excludes_summary(excludes: &GlobalExcludes) -> GlobalExcludes {
    let normalized = normalize_global_excludes(excludes);
    GlobalExcludes {
        dir_names: normalized.dir_names,
        dir_suffixes: normalized.dir_suffixes,
        file_names: normalized.file_names,
        extensions: normalized.file_suffixes,
        patterns: normalized.path_segments,
    }
}
