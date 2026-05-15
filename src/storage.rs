use crate::project::ProjectInfo;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentFile {
    #[serde(skip)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub file: String,
    pub line: u64,
    pub note: Option<String>,
    pub id: String,
    pub absolute: bool,
}

#[derive(Serialize, Deserialize)]
struct JsonFile {
    version: u64,
    bookmarks: Vec<JsonComment>,
}

#[derive(Serialize, Deserialize)]
struct JsonComment {
    file: String,
    line: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    absolute: Option<bool>,
}

pub fn storage_path(
    data_dir: &Path,
    project_id: &str,
    branch: Option<&str>,
    per_branch: bool,
) -> PathBuf {
    let mut key = project_id.to_string();
    if per_branch {
        key.push('|');
        key.push_str(branch.unwrap_or("__default__"));
    }
    let hash = sha256_hex(&key);
    data_dir.join(format!("{}.json", &hash[..12]))
}

impl Comment {
    pub fn new(file: String, line: u64, note: Option<String>, absolute: bool) -> Self {
        let nonce: u64 = rand::rng().random();
        let id_input = format!("{}{}{}", file, line, nonce);
        let id = sha256_hex(&id_input)[..16].to_string();
        Self {
            file,
            line,
            note,
            id,
            absolute,
        }
    }

    pub fn display_path(&self, project: &ProjectInfo) -> String {
        if self.absolute {
            return self.file.clone();
        }
        if let Some(root) = &project.root {
            let prefix = format!("{}/", root.display());
            if let Some(rel) = self.file.strip_prefix(&prefix) {
                return rel.to_string();
            }
        }
        self.file.clone()
    }
}

impl CommentFile {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self {
                comments: Vec::new(),
            });
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        let json: JsonFile = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
        if json.version != 2 {
            return Err(format!(
                "unsupported storage version {} in {}",
                json.version,
                path.display()
            ));
        }
        let comments = json
            .bookmarks
            .into_iter()
            .map(|jc| Comment {
                file: jc.file,
                line: jc.line,
                note: jc.note,
                id: jc.id,
                absolute: jc.absolute.unwrap_or(false),
            })
            .collect();
        Ok(Self { comments })
    }

    pub fn resolve_paths(&mut self, project: &ProjectInfo) {
        let Some(root) = &project.root else { return };
        for b in &mut self.comments {
            if !b.absolute && !Path::new(&b.file).is_absolute() {
                b.file = format!("{}/{}", root.display(), b.file);
            }
        }
    }

    pub fn save(&self, path: &Path, data_dir: &Path, project: &ProjectInfo) {
        if self.comments.is_empty() {
            let _ = fs::remove_file(path);
            return;
        }

        fs::create_dir_all(data_dir).unwrap_or_else(|e| {
            eprintln!("error: failed to create data dir: {e}");
            std::process::exit(1);
        });

        let json = JsonFile {
            version: 2,
            bookmarks: self.comments.iter().map(|c| c.to_json(project)).collect(),
        };
        let content = serde_json::to_string_pretty(&json).expect("JSON serialization failed");

        // Atomic write: temp file + rename
        let tmp_path = path.with_extension("tmp");
        let mut f = fs::File::create(&tmp_path).unwrap_or_else(|e| {
            eprintln!("error: failed to create temp file: {e}");
            std::process::exit(1);
        });
        f.write_all(content.as_bytes()).unwrap_or_else(|e| {
            eprintln!("error: failed to write temp file: {e}");
            std::process::exit(1);
        });
        fs::rename(&tmp_path, path).unwrap_or_else(|e| {
            eprintln!("error: failed to rename temp file: {e}");
            std::process::exit(1);
        });
    }
}

impl Comment {
    fn to_json(&self, project: &ProjectInfo) -> JsonComment {
        if self.absolute {
            return JsonComment {
                file: self.file.clone(),
                line: self.line,
                note: self.note.clone(),
                id: self.id.clone(),
                absolute: Some(true),
            };
        }

        let Some(root) = &project.root else {
            return JsonComment {
                file: self.file.clone(),
                line: self.line,
                note: self.note.clone(),
                id: self.id.clone(),
                absolute: Some(true),
            };
        };

        let prefix = format!("{}/", root.display());
        let Some(file) = self.file.strip_prefix(&prefix) else {
            return JsonComment {
                file: self.file.clone(),
                line: self.line,
                note: self.note.clone(),
                id: self.id.clone(),
                absolute: Some(true),
            };
        };

        JsonComment {
            file: file.to_string(),
            line: self.line,
            note: self.note.clone(),
            id: self.id.clone(),
            absolute: None,
        }
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
