// Path: crates/im_bundle/src/git_capture/pathspec_batches.rs
// Description: Host-safe Git pathspec argument batching with atomic rename pairs

use std::ffi::OsString;

use super::path::GitPath;

const COMMAND_PATH_BYTES_LIMIT: usize = 24 * 1024;
const ARGUMENT_FRAMING_BYTES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathspecBatchError {
    UnsupportedEncoding,
    AtomicGroupTooLarge,
}

pub(crate) fn batch_single_paths(
    paths: &[GitPath],
) -> std::result::Result<Vec<Vec<OsString>>, PathspecBatchError> {
    let mut batches = PathspecBatches::default();
    for path in paths {
        batches.push_group(std::slice::from_ref(path))?;
    }
    Ok(batches.finish())
}

pub(crate) fn batch_rename_pairs(
    pairs: &[[GitPath; 2]],
) -> std::result::Result<Vec<Vec<OsString>>, PathspecBatchError> {
    let mut batches = PathspecBatches::default();
    for pair in pairs {
        batches.push_group(pair)?;
    }
    Ok(batches.finish())
}

#[derive(Default)]
struct PathspecBatches {
    completed: Vec<Vec<OsString>>,
    current: Vec<OsString>,
    current_bytes: usize,
}

impl PathspecBatches {
    fn push_group(&mut self, paths: &[GitPath]) -> std::result::Result<(), PathspecBatchError> {
        let mut encoded = Vec::with_capacity(paths.len());
        let mut encoded_bytes = 0usize;
        for path in paths {
            let argument = path
                .to_os_string()
                .ok_or(PathspecBatchError::UnsupportedEncoding)?;
            encoded_bytes = encoded_bytes
                .saturating_add(path.as_bytes().len())
                .saturating_add(ARGUMENT_FRAMING_BYTES);
            encoded.push(argument);
        }
        if encoded_bytes > COMMAND_PATH_BYTES_LIMIT {
            return Err(PathspecBatchError::AtomicGroupTooLarge);
        }
        if !self.current.is_empty()
            && self.current_bytes.saturating_add(encoded_bytes) > COMMAND_PATH_BYTES_LIMIT
        {
            self.completed.push(std::mem::take(&mut self.current));
            self.current_bytes = 0;
        }
        self.current.extend(encoded);
        self.current_bytes = self.current_bytes.saturating_add(encoded_bytes);
        Ok(())
    }

    fn finish(mut self) -> Vec<Vec<OsString>> {
        if !self.current.is_empty() {
            self.completed.push(self.current);
        }
        self.completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_large_path_sets_without_splitting_rename_pairs() {
        let path = GitPath::from_bytes(&vec![
            b'a';
            (COMMAND_PATH_BYTES_LIMIT
                - (2 * ARGUMENT_FRAMING_BYTES))
                / 2
        ]);
        let singles = batch_single_paths(&[path.clone(), path.clone(), path.clone()])
            .expect("single path batches");
        assert_eq!(singles.len(), 2);

        let pairs =
            batch_rename_pairs(&[[path.clone(), path]]).expect("rename pair at the batch boundary");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].len(), 2);
    }

    #[test]
    fn rejects_an_atomic_group_larger_than_the_host_safe_budget() {
        let oversized = GitPath::from_bytes(&vec![b'a'; COMMAND_PATH_BYTES_LIMIT + 1]);
        assert_eq!(
            batch_single_paths(&[oversized]),
            Err(PathspecBatchError::AtomicGroupTooLarge)
        );
    }
}
