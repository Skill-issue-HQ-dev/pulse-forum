//! The static site on disk: reads every file under a folder into a tree, and writes back the texts that changed.
//! Owns walking the folder and the writes; the stamping itself is core::stamp.
//! Invariant: only files whose bytes changed are written, and nothing outside the folder is touched.

use crate::core::stamp::Tree;
use std::path::Path;

/// Every file under `root`, keyed by its path relative to `root` with forward slashes.
pub fn read_tree(root: &Path) -> Result<Tree, String> {
    let mut tree = Tree::new();
    walk(root, root, &mut tree)?;
    Ok(tree)
}

fn walk(root: &Path, dir: &Path, tree: &mut Tree) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("files: read {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("files: list {}: {e}", dir.display()))?.path();
        if path.is_dir() {
            walk(root, &path, tree)?;
        } else {
            let bytes = std::fs::read(&path).map_err(|e| format!("files: read {}: {e}", path.display()))?;
            let rel = path.strip_prefix(root).map_err(|e| format!("files: {}: {e}", path.display()))?;
            tree.insert(rel.to_string_lossy().replace('\\', "/"), bytes);
        }
    }
    Ok(())
}

/// Writes the files of `after` whose bytes differ from `before`; returns how many were written.
pub fn write_changed(root: &Path, before: &Tree, after: &Tree) -> Result<usize, String> {
    let mut written = 0;
    for (rel, bytes) in after {
        if before.get(rel) != Some(bytes) {
            let path = root.join(rel);
            std::fs::write(&path, bytes).map_err(|e| format!("files: write {}: {e}", path.display()))?;
            written += 1;
        }
    }
    Ok(written)
}
