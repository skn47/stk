use std::fs;
use std::io;
use std::path::Path;

/// Recursively visits every regular file at or under `root`, skipping `.git`
/// directories and not following symlinks (avoiding cycles). A missing path or a
/// permission error propagates rather than being silently treated as "not a file".
/// Shared by `grep` and `find` so their traversal/skip/error behavior can't drift apart.
pub fn walk_files(root: &Path, on_file: &mut dyn FnMut(&Path)) -> io::Result<()> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.is_symlink() {
        return Ok(());
    }
    if !metadata.is_dir() {
        on_file(root);
        return Ok(());
    }
    if root.file_name().and_then(|n| n.to_str()) == Some(".git") {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(root)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        walk_files(&entry.path(), on_file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "stk-walk-files-test-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn visits_files_recursively_skipping_git() {
        let dir = temp_dir("basic");
        fs::write(dir.join("a.txt"), "").unwrap();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("b.txt"), "").unwrap();
        let git = dir.join(".git");
        fs::create_dir_all(&git).unwrap();
        fs::write(git.join("config"), "").unwrap();

        let mut visited = Vec::new();
        walk_files(&dir, &mut |p| visited.push(p.to_path_buf())).unwrap();

        assert_eq!(visited.len(), 2);
        assert!(visited.iter().any(|p| p.ends_with("a.txt")));
        assert!(visited.iter().any(|p| p.ends_with("b.txt")));
    }

    #[test]
    fn a_missing_root_path_is_a_real_error_not_silently_empty() {
        let dir = temp_dir("missing");
        let missing = dir.join("does_not_exist");

        let mut visited = Vec::new();
        let result = walk_files(&missing, &mut |p| visited.push(p.to_path_buf()));

        assert!(result.is_err());
        assert!(visited.is_empty());
    }

    #[test]
    fn a_single_file_root_visits_just_that_file() {
        let dir = temp_dir("single-file");
        let file = dir.join("only.txt");
        fs::write(&file, "").unwrap();

        let mut visited = Vec::new();
        walk_files(&file, &mut |p| visited.push(p.to_path_buf())).unwrap();

        assert_eq!(visited, vec![file]);
    }

    #[test]
    #[cfg(unix)]
    fn a_symlink_loop_does_not_cause_infinite_recursion() {
        use std::os::unix::fs::symlink;

        let dir = temp_dir("symlink-loop");
        fs::write(dir.join("real.txt"), "").unwrap();
        symlink(&dir, dir.join("loop")).unwrap();

        let mut visited = Vec::new();
        walk_files(&dir, &mut |p| visited.push(p.to_path_buf())).unwrap();

        assert_eq!(visited, vec![dir.join("real.txt")]);
    }
}
