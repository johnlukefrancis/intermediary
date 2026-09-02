// Path: crates/im_bundle/src/git_capture/index_tree.rs
// Description: Read-only Git tree SHA of an index listing, matching `git write-tree`

use std::collections::BTreeMap;

use sha1::{Digest, Sha1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexTreeError {
    Unmerged,
    Malformed,
}

enum Node {
    Blob { mode: Vec<u8>, sha: [u8; 20] },
    Tree(BTreeMap<Vec<u8>, Node>),
}

/// Computes the tree object id that `git write-tree` would produce for the
/// given NUL-delimited `git ls-files --stage -z` output, without writing
/// objects into the repository.
pub(crate) fn index_tree_sha(ls_files_stage: &[u8]) -> Result<String, IndexTreeError> {
    let mut root = BTreeMap::new();
    for entry in ls_files_stage.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let tab = entry
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or(IndexTreeError::Malformed)?;
        let (meta, path) = (&entry[..tab], &entry[tab + 1..]);
        let mut fields = meta.split(|byte| *byte == b' ');
        let mode = fields.next().ok_or(IndexTreeError::Malformed)?;
        let sha_hex = fields.next().ok_or(IndexTreeError::Malformed)?;
        let stage = fields.next().ok_or(IndexTreeError::Malformed)?;
        if stage != b"0" {
            return Err(IndexTreeError::Unmerged);
        }
        let sha = decode_sha(sha_hex)?;
        insert(&mut root, path, mode.to_vec(), sha)?;
    }
    Ok(hex(&hash_tree(&root)))
}

fn insert(
    tree: &mut BTreeMap<Vec<u8>, Node>,
    path: &[u8],
    mode: Vec<u8>,
    sha: [u8; 20],
) -> Result<(), IndexTreeError> {
    match path.iter().position(|byte| *byte == b'/') {
        None => {
            if path.is_empty() {
                return Err(IndexTreeError::Malformed);
            }
            tree.insert(path.to_vec(), Node::Blob { mode, sha });
            Ok(())
        }
        Some(slash) => {
            let (dir, rest) = (&path[..slash], &path[slash + 1..]);
            let child = tree
                .entry(dir.to_vec())
                .or_insert_with(|| Node::Tree(BTreeMap::new()));
            match child {
                Node::Tree(children) => insert(children, rest, mode, sha),
                Node::Blob { .. } => Err(IndexTreeError::Malformed),
            }
        }
    }
}

fn hash_tree(tree: &BTreeMap<Vec<u8>, Node>) -> [u8; 20] {
    // Git orders tree entries by name, comparing directories as `name/`.
    let mut entries: Vec<(Vec<u8>, &[u8], &Vec<u8>, [u8; 20])> = tree
        .iter()
        .map(|(name, node)| match node {
            Node::Blob { mode, sha } => (name.clone(), mode.as_slice(), name, *sha),
            Node::Tree(children) => {
                let mut key = name.clone();
                key.push(b'/');
                (key, b"40000".as_slice(), name, hash_tree(children))
            }
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut body = Vec::new();
    for (_, mode, name, sha) in entries {
        body.extend_from_slice(mode);
        body.push(b' ');
        body.extend_from_slice(name);
        body.push(0);
        body.extend_from_slice(&sha);
    }
    let mut hasher = Sha1::new();
    hasher.update(format!("tree {}\0", body.len()).as_bytes());
    hasher.update(&body);
    hasher.finalize().into()
}

fn decode_sha(hex: &[u8]) -> Result<[u8; 20], IndexTreeError> {
    if hex.len() != 40 {
        return Err(IndexTreeError::Malformed);
    }
    let mut out = [0u8; 20];
    for (index, pair) in hex.chunks(2).enumerate() {
        let text = std::str::from_utf8(pair).map_err(|_| IndexTreeError::Malformed)?;
        out[index] = u8::from_str_radix(text, 16).map_err(|_| IndexTreeError::Malformed)?;
    }
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn matches_git_write_tree_including_directory_ordering() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo = root.path();
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "t@example.test"],
            vec!["config", "user.name", "T"],
        ] {
            assert!(Command::new("git")
                .args(args)
                .current_dir(repo)
                .status()
                .unwrap()
                .success());
        }
        std::fs::create_dir_all(repo.join("a/deep")).unwrap();
        for (path, body) in [
            ("a-b", "dash sorts before slash\n"),
            ("a0", "digit\n"),
            ("a/b", "nested\n"),
            ("a/deep/c", "deeper\n"),
            ("z", "last\n"),
        ] {
            std::fs::write(repo.join(path), body).unwrap();
        }
        assert!(Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .unwrap()
            .success());
        let expected = Command::new("git")
            .arg("write-tree")
            .current_dir(repo)
            .output()
            .unwrap();
        let listing = Command::new("git")
            .args(["ls-files", "--stage", "-z", "--full-name", "--", ":/"])
            .current_dir(repo)
            .output()
            .unwrap();
        assert_eq!(
            index_tree_sha(&listing.stdout).unwrap(),
            String::from_utf8(expected.stdout).unwrap().trim()
        );
    }
}
