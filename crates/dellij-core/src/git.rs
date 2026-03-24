use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8Path;
use git2::{DiffOptions, Repository};
use std::process::{Command, Stdio};

pub fn git(project_root: &Utf8Path, args: &[&str]) -> Result<()> {
    // We keep Command for complex multi-step git operations that are already working well on desktop,
    // but we can augment or replace specific ones with git2 as needed.
    // For mobile, we will eventually want to use git2 for everything.
    let status = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !status.success() {
        bail!("git {} exited with {}", args.join(" "), status);
    }
    Ok(())
}

pub fn git_output(project_root: &Utf8Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} exited with {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub fn detect_base_branch(project_root: &Utf8Path) -> Result<String> {
    let repo = Repository::open(project_root)?;
    let head = repo.find_reference("refs/remotes/origin/HEAD")?;
    let target = head.symbolic_target()
        .context("origin/HEAD is not a symbolic ref")?;
    Ok(target.trim_start_matches("refs/remotes/origin/").to_string())
}

pub fn resolve_project_root(
    project_root: Option<camino::Utf8PathBuf>,
) -> Result<camino::Utf8PathBuf> {
    if let Some(root) = project_root {
        return Ok(root);
    }
    let cwd = std::env::current_dir().context("resolving current directory")?;
    let repo = Repository::discover(&cwd)?;
    let path = repo.workdir()
        .context("repository has no workdir")?;
    camino::Utf8PathBuf::from_path_buf(path.to_path_buf())
        .map_err(|_| anyhow!("project root is not valid utf-8"))
}

/// Returns the diff between base_branch and branch as a String.
pub fn workspace_diff(
    project_root: &Utf8Path,
    base_branch: &str,
    branch: &str,
) -> Result<String> {
    let repo = Repository::open(project_root)?;
    
    let base_obj = repo.revparse_single(base_branch)?;
    let branch_obj = repo.revparse_single(branch)?;
    
    let base_tree = base_obj.peel_to_tree()?;
    let branch_tree = branch_obj.peel_to_tree()?;
    
    let mut opts = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&branch_tree), Some(&mut opts))?;
    
    let mut diff_str = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        diff_str.push(line.origin() as u8);
        diff_str.extend_from_slice(line.content());
        true
    })?;
    
    Ok(String::from_utf8_lossy(&diff_str).into_owned())
}

/// Open the worktree in an editor by name (code, cursor, zed, idea, etc.)
pub fn open_in_editor(editor: &str, path: &Utf8Path) -> Result<()> {
    Command::new(editor)
        .arg(path.as_str())
        .spawn()
        .with_context(|| format!("opening {path} in {editor}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use git2::{Repository, Signature};

    fn setup_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        
        {
            let mut index = repo.index().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = Signature::now("Test", "test@example.com").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }
        
        (dir, repo)
    }

    #[test]
    fn test_detect_base_branch() {
        let (dir, repo) = setup_repo();
        let path = Utf8Path::from_path(dir.path()).unwrap();
        
        repo.remote("origin", "https://github.com/example/repo").unwrap();
        repo.reference_symbolic("refs/remotes/origin/HEAD", "refs/remotes/origin/master", true, "mock origin/HEAD").unwrap();
        repo.reference("refs/remotes/origin/master", repo.head().unwrap().target().unwrap(), true, "mock origin/master").unwrap();
        
        let base = detect_base_branch(path).unwrap();
        assert_eq!(base, "master");
    }

    #[test]
    fn test_resolve_project_root() {
        let (dir, _repo) = setup_repo();
        let path = Utf8Path::from_path(dir.path()).unwrap();
        
        let resolved = resolve_project_root(Some(path.to_path_buf())).unwrap();
        assert_eq!(resolved, path);
        
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sub).unwrap();
        
        let resolved = resolve_project_root(None).unwrap();
        assert_eq!(resolved.canonicalize().unwrap(), path.canonicalize().unwrap());
        
        std::env::set_current_dir(old_cwd).unwrap();
    }

    #[test]
    fn test_workspace_diff() {
        let (dir, repo) = setup_repo();
        let path = Utf8Path::from_path(dir.path()).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        
        let head = repo.head().unwrap();
        let commit = repo.find_commit(head.target().unwrap()).unwrap();
        let master_name = repo.head().unwrap().shorthand().unwrap().to_string();
        
        repo.branch("feat", &commit, false).unwrap();
        repo.set_head("refs/heads/feat").unwrap();
        
        std::fs::write(dir.path().join("test.txt"), "hello\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Add test.txt", &tree, &[&commit]).unwrap();
        
        let diff = workspace_diff(path, &master_name, "feat").unwrap();
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("+hello"));
    }
}
