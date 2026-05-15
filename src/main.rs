mod project;
mod storage;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process;

use serde::Deserialize;

use project::ProjectInfo;
use storage::{Comment, CommentFile};

#[derive(Deserialize)]
struct InputComment {
    file: String,
    line: u64,
    note: String,
}

#[derive(Deserialize)]
struct InputEdit {
    id: String,
    #[serde(default)]
    line: Option<u64>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Parser)]
#[command(name = "haunt", about = "CLI for haunt.nvim comments")]
struct Cli {
    /// Named session (stored in .haunt/<name>/)
    #[arg(long, global = true)]
    session: Option<String>,

    /// Disable per-branch comments (use project-wide storage)
    #[arg(long, global = true)]
    no_per_branch: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all comments for the current project/branch
    List {
        /// Filter to a specific file (relative to repo root or absolute)
        #[arg(long)]
        file: Option<String>,
    },

    /// Add a comment
    Add {
        /// File path (relative to repo root or absolute)
        #[arg(long)]
        file: String,
        /// Line number (1-based)
        #[arg(long)]
        line: u64,
        /// Annotation text
        note: String,
    },

    /// Edit a comment's note and/or line number
    Edit {
        /// Comment ID (or unique prefix)
        #[arg(long)]
        id: String,
        /// New annotation text
        #[arg(long)]
        note: Option<String>,
        /// New line number
        #[arg(long)]
        line: Option<u64>,
    },

    /// Delete a comment
    Delete {
        /// Comment ID (or unique prefix)
        #[arg(long)]
        id: String,
    },

    /// Show details of a single comment
    Show {
        /// Comment ID (or unique prefix)
        #[arg(long)]
        id: String,
    },

    /// Batch-add comments from JSON on stdin
    Apply,

    /// Batch-edit comments from JSON on stdin
    BatchEdit,

    /// Clear comments (all, or for a specific file)
    Clear {
        /// Only clear comments for this file (relative to repo root or absolute)
        #[arg(long)]
        file: Option<String>,
    },

    /// List available sessions
    Sessions,
}

fn resolve_session_name(cli_session: Option<String>) -> Option<String> {
    if let Some(name) = cli_session {
        return Some(name);
    }
    std::env::var("HAUNT_SESSION")
        .ok()
        .filter(|s| !s.is_empty())
}

fn resolve_data_dir(session: Option<String>, project: &ProjectInfo) -> PathBuf {
    let session_name = resolve_session_name(session);

    let Some(name) = session_name else {
        eprintln!("error: no session specified — set HAUNT_SESSION or pass --session <name>");
        process::exit(1);
    };

    match project.data_root() {
        Some(root) => root.join(".haunt").join(name),
        None => {
            eprintln!("error: not in a git repository");
            process::exit(1);
        }
    }
}

fn find_by_id_prefix<'a>(comments: &'a [Comment], prefix: &str) -> Result<&'a Comment, String> {
    let matches: Vec<_> = comments
        .iter()
        .filter(|c| c.id.starts_with(prefix))
        .collect();
    match matches.len() {
        0 => Err(format!("no comment matching id prefix '{prefix}'")),
        1 => Ok(matches[0]),
        n => Err(format!(
            "ambiguous id prefix '{prefix}' matches {n} comments"
        )),
    }
}

fn main() {
    let cli = Cli::parse();
    let per_branch = !cli.no_per_branch;

    let project = match ProjectInfo::detect() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if matches!(cli.command, Command::Sessions) {
        let haunt_dir = match project.data_root() {
            Some(root) => root.join(".haunt"),
            None => {
                eprintln!("error: not in a git repository");
                process::exit(1);
            }
        };
        if let Ok(entries) = fs::read_dir(&haunt_dir) {
            let mut names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            names.sort();
            for name in names {
                println!("{name}");
            }
        }
        return;
    }

    let data_dir = resolve_data_dir(cli.session, &project);
    let storage_path = storage::storage_path(
        &data_dir,
        &project.project_id,
        project.branch.as_deref(),
        per_branch,
    );

    match cli.command {
        Command::List { file } => {
            let mut cf = match CommentFile::load(&storage_path) {
                Ok(cf) => cf,
                Err(e) => {
                    eprintln!("warning: {e}");
                    return;
                }
            };
            cf.resolve_paths(&project);
            let filter = file.map(|f| project.resolve_file_arg(&f).to_string_lossy().to_string());
            if cf.comments.is_empty() {
                return;
            }
            for c in cf
                .comments
                .iter()
                .filter(|c| filter.as_ref().is_none_or(|f| c.file == *f))
            {
                let file = c.display_path(&project);
                println!(
                    "{}:{} {} [{}]",
                    file,
                    c.line,
                    c.note.as_deref().unwrap_or(""),
                    &c.id[..8]
                );
            }
        }
        Command::Add { file, line, note } => {
            let mut cf = CommentFile::load(&storage_path).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            cf.resolve_paths(&project);
            let path = project.resolve_file_arg(&file);
            let absolute = !project.is_within_root(&path);
            let comment = Comment::new(
                path.to_string_lossy().to_string(),
                line,
                Some(note),
                absolute,
            );
            let id = comment.id.clone();
            cf.comments.push(comment);
            cf.save(&storage_path, &data_dir, &project);
            println!("{id}");
        }
        Command::Apply => {
            let input: Vec<InputComment> = serde_json::from_reader(std::io::stdin())
                .unwrap_or_else(|e| {
                    eprintln!("error: invalid JSON input: {e}");
                    process::exit(1);
                });
            let mut cf = CommentFile::load(&storage_path).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            cf.resolve_paths(&project);
            let mut ids = Vec::with_capacity(input.len());
            for ic in input {
                let path = project.resolve_file_arg(&ic.file);
                let absolute = !project.is_within_root(&path);
                let comment = Comment::new(
                    path.to_string_lossy().to_string(),
                    ic.line,
                    Some(ic.note),
                    absolute,
                );
                ids.push(comment.id.clone());
                cf.comments.push(comment);
            }
            cf.save(&storage_path, &data_dir, &project);
            for id in &ids {
                println!("{id}");
            }
        }
        Command::Edit { id, note, line } => {
            if note.is_none() && line.is_none() {
                eprintln!("error: provide at least one of --note or --line");
                process::exit(1);
            }
            let mut cf = CommentFile::load(&storage_path).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            cf.resolve_paths(&project);
            let target_id = find_by_id_prefix(&cf.comments, &id)
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                })
                .id
                .clone();
            let c = cf.comments.iter_mut().find(|c| c.id == target_id).unwrap();
            if let Some(n) = note {
                if n.is_empty() {
                    eprintln!("error: note cannot be empty");
                    process::exit(1);
                }
                c.note = Some(n);
            }
            if let Some(l) = line {
                c.line = l;
            }
            cf.save(&storage_path, &data_dir, &project);
        }
        Command::Delete { id } => {
            let mut cf = CommentFile::load(&storage_path).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            cf.resolve_paths(&project);
            let target_id = find_by_id_prefix(&cf.comments, &id)
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                })
                .id
                .clone();
            cf.comments.retain(|c| c.id != target_id);
            cf.save(&storage_path, &data_dir, &project);
        }
        Command::Show { id } => {
            let mut cf = match CommentFile::load(&storage_path) {
                Ok(cf) => cf,
                Err(e) => {
                    eprintln!("warning: {e}");
                    return;
                }
            };
            cf.resolve_paths(&project);
            let c = find_by_id_prefix(&cf.comments, &id).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            println!("id:   {}", c.id);
            println!("file: {}", c.display_path(&project));
            println!("line: {}", c.line);
            println!("note: {}", c.note.as_deref().unwrap_or(""));
        }
        Command::BatchEdit => {
            let edits: Vec<InputEdit> =
                serde_json::from_reader(std::io::stdin()).unwrap_or_else(|e| {
                    eprintln!("error: invalid JSON input: {e}");
                    process::exit(1);
                });
            let mut cf = CommentFile::load(&storage_path).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            cf.resolve_paths(&project);
            for edit in edits {
                let target_id = find_by_id_prefix(&cf.comments, &edit.id)
                    .unwrap_or_else(|e| {
                        eprintln!("error: {e}");
                        process::exit(1);
                    })
                    .id
                    .clone();
                let c = cf.comments.iter_mut().find(|c| c.id == target_id).unwrap();
                if let Some(n) = edit.note {
                    if n.is_empty() {
                        eprintln!("error: note cannot be empty for id '{}'", edit.id);
                        process::exit(1);
                    }
                    c.note = Some(n);
                }
                if let Some(l) = edit.line {
                    c.line = l;
                }
            }
            cf.save(&storage_path, &data_dir, &project);
        }
        Command::Sessions => unreachable!(),
        Command::Clear { file } => {
            let mut cf = CommentFile::load(&storage_path).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            cf.resolve_paths(&project);
            let before = cf.comments.len();
            match file {
                Some(f) => {
                    let abs = project.resolve_file_arg(&f).to_string_lossy().to_string();
                    cf.comments.retain(|c| c.file != abs);
                }
                None => cf.comments.clear(),
            }
            let removed = before - cf.comments.len();
            cf.save(&storage_path, &data_dir, &project);
            println!(
                "cleared {removed} comment{}",
                if removed == 1 { "" } else { "s" }
            );
        }
    }
}
