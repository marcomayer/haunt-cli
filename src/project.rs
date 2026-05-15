use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub struct ProjectInfo {
    pub root: Option<PathBuf>,
    pub branch: Option<String>,
    pub project_id: String,
}

impl ProjectInfo {
    pub fn detect() -> Result<Self, String> {
        let root = git_root();
        let branch = git_branch();
        let project_id = project_id(&root);

        Ok(Self {
            root,
            branch,
            project_id,
        })
    }

    pub fn data_root(&self) -> Option<PathBuf> {
        if let Some(common_dir) = run_git(&["rev-parse", "--git-common-dir"]) {
            let path = PathBuf::from(common_dir);
            if path.is_absolute() {
                return path.parent().map(|p| p.to_path_buf());
            }
            if let Some(root) = &self.root {
                return Some(root.join(&path).parent()?.to_path_buf());
            }
        }
        self.root.clone()
    }

    pub fn resolve_file_arg(&self, file: &str) -> PathBuf {
        let path = Path::new(file);
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match &self.root {
                Some(root) => root.join(file),
                None => std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(file),
            }
        };
        normalize_lexical(&joined)
    }

    pub fn is_within_root(&self, file: &Path) -> bool {
        let Some(root) = &self.root else {
            return false;
        };
        let Ok(canonical_root) = root.canonicalize() else {
            return false;
        };
        file == canonical_root || file.starts_with(&canonical_root)
    }
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn run_git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn git_root() -> Option<PathBuf> {
    run_git(&["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

fn git_branch() -> Option<String> {
    if let Some(branch) = run_git(&["branch", "--show-current"]) {
        return Some(branch);
    }
    run_git(&["rev-parse", "--short", "HEAD"])
}

fn project_id(root: &Option<PathBuf>) -> String {
    if let Some(hash) = run_git(&["rev-list", "--max-parents=0", "HEAD"])
        && let Some(first) = hash.lines().next()
    {
        return first.to_string();
    }
    if let Some(root) = root {
        return root.to_string_lossy().to_string();
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .to_string_lossy()
        .to_string()
}
