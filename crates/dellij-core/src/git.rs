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
    diff.print_patch(|_delta, _hunk, line| {
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
