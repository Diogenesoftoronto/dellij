use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use std::process::{Command, Stdio};

pub fn git(project_root: &Utf8Path, args: &[&str]) -> Result<()> {
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
    let output = git_output(
        project_root,
        &["symbolic-ref", "refs/remotes/origin/HEAD", "--short"],
    )?;
    Ok(output.trim().trim_start_matches("origin/").to_string())
}

pub fn resolve_project_root(
    project_root: Option<camino::Utf8PathBuf>,
) -> Result<camino::Utf8PathBuf> {
    use anyhow::anyhow;
    if let Some(root) = project_root {
        return Ok(root);
    }
    let cwd = std::env::current_dir().context("resolving current directory")?;
    let cwd = camino::Utf8PathBuf::from_path_buf(cwd)
        .map_err(|_| anyhow!("cwd is not valid utf-8"))?;
    if let Ok(root) = git_output(&cwd, &["rev-parse", "--show-toplevel"]) {
        return Ok(camino::Utf8PathBuf::from(root.trim()));
    }
    Ok(cwd)
}

/// Returns the diff between base_branch and branch as a String.
pub fn workspace_diff(
    project_root: &Utf8Path,
    base_branch: &str,
    branch: &str,
) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", &format!("{base_branch}...{branch}")])
        .current_dir(project_root)
        .stdin(Stdio::null())
        .output()
        .context("running git diff")?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Open the worktree in an editor by name (code, cursor, zed, idea, etc.)
pub fn open_in_editor(editor: &str, path: &Utf8Path) -> Result<()> {
    Command::new(editor)
        .arg(path.as_str())
        .spawn()
        .with_context(|| format!("opening {path} in {editor}"))?;
    Ok(())
}
