use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use rmcp::{
    ErrorData as McpError, Json, ServerHandler, handler::server::wrapper::Parameters, tool,
    tool_handler, tool_router,
};
use serde_json::{Map, Value};
use tokio::fs;
use tracing::info;

use crate::{
    dto::{
        DeleteNoteParams, GetFrontmatterParams, GetFrontmatterResult, ListDirectoryParams,
        ListDirectoryResult, MoveFileParams, PatchNoteParams, ReadNoteParams,
        UpdateFrontmatterParams, WriteMode, WriteNoteParams,
    },
    frontmatter::{parse_matter, split_frontmatter},
};

/// Hidden entries (`.obsidian`, `.git`, `.trash`, dotfiles) and `node_modules` are off-limits.
/// Trailing dots and spaces are stripped because Windows ignores them
/// (`.git.` opens `.git`).
fn is_restricted(name: &str) -> bool {
    let name = name.trim_end_matches(['.', ' ']);
    name.is_empty() || name.starts_with('.') || name.eq_ignore_ascii_case("node_modules")
}

/// Writes via a sibling temp file + rename so readers (Obsidian, sync tools) never see a
/// truncated note. The leading dot keeps the temp file hidden from `is_restricted` listings.
async fn write_atomic(path: &Path, content: impl AsRef<[u8]>) -> std::io::Result<()> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(ErrorKind::InvalidInput.into());
    };
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    fs::write(&tmp, content).await?;
    if let Err(e) = fs::rename(&tmp, path).await {
        let _ = fs::remove_file(&tmp).await;
        return Err(e);
    }
    Ok(())
}

fn invalid_path(path: &str, reason: &str) -> McpError {
    McpError::invalid_params(format!("Invalid path '{path}': {reason}"), None)
}

async fn resolve_path(vault_path: &Path, note_path: &str) -> Result<PathBuf, McpError> {
    let rel = Path::new(note_path);
    if !rel
        .components()
        .all(|c| matches!(c, Component::Normal(n) if !n.to_string_lossy().contains(':')))
    {
        return Err(invalid_path(
            note_path,
            "it must be relative to the vault root and must not be absolute or contain '..', '.' or ':'.",
        ));
    }

    let full = vault_path.join(rel);
    let mut existing = full.as_path();
    while fs::symlink_metadata(existing).await.is_err() {
        existing = existing
            .parent()
            .ok_or_else(|| invalid_path(note_path, "no existing parent directory."))?;
    }
    let tail = full
        .strip_prefix(existing)
        .expect("existing is an ancestor of full");
    let canon = fs::canonicalize(existing)
        .await
        .map_err(|_| invalid_path(note_path, "it cannot be resolved."))?
        .join(tail);

    let inside = canon
        .strip_prefix(vault_path)
        .map_err(|_| invalid_path(note_path, "it points outside the vault."))?;
    if inside
        .components()
        .any(|c| is_restricted(&c.as_os_str().to_string_lossy()))
    {
        return Err(invalid_path(
            note_path,
            "hidden entries (starting with '.') and node_modules are not accessible.",
        ));
    }
    Ok(canon)
}

fn internal_error(e: impl ToString) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

fn not_found(path: &str) -> McpError {
    McpError::resource_not_found(format!("Path not found: {path}"), None)
}

fn yaml_error(e: impl std::fmt::Display, path: &str) -> McpError {
    McpError::invalid_params(format!("Invalid frontmatter in '{path}': {e}"), None)
}

fn io_error(e: std::io::Error, path: &str) -> McpError {
    if e.kind() == ErrorKind::NotFound {
        not_found(path)
    } else {
        internal_error(format!("I/O error on '{path}': {e}"))
    }
}

fn render_note(frontmatter: Option<Map<String, Value>>, body: &str) -> Result<String, McpError> {
    match frontmatter {
        Some(f) if !f.is_empty() => {
            let yaml = serde_yaml_ng::to_string(&f).map_err(internal_error)?;
            Ok(format!("---\n{yaml}---\n{body}"))
        }
        _ => Ok(body.to_string()),
    }
}

fn merge_objects(old: &mut Map<String, Value>, new: Map<String, Value>) {
    for (key, value) in new {
        match value {
            Value::Null => {
                old.remove(&key);
            }
            Value::Object(new_obj) => match old.get_mut(&key) {
                Some(Value::Object(old_obj)) => merge_objects(old_obj, new_obj),
                _ => {
                    old.insert(key, Value::Object(new_obj));
                }
            },
            value => {
                old.insert(key, value);
            }
        }
    }
}

fn merge_front(
    old: Option<Map<String, Value>>,
    new: Option<Map<String, Value>>,
) -> Option<Map<String, Value>> {
    match (old, new) {
        (Some(mut a), Some(b)) => {
            merge_objects(&mut a, b);
            Some(a)
        }
        (_, Some(n)) => Some(n),
        (o, None) => o,
    }
}

#[derive(Clone)]
pub struct Server {
    vault_path: PathBuf,
}

#[tool_router]
impl Server {
    pub fn new(vault_path: PathBuf) -> Self {
        Self { vault_path }
    }

    #[tool(
        description = "Read a note and return its raw text: the YAML frontmatter block (if any) followed by the markdown body. To get only the properties, use get_frontmatter instead."
    )]
    async fn read_note(
        &self,
        Parameters(ReadNoteParams { path }): Parameters<ReadNoteParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path).await?;
        if fs::metadata(&note_path).await.is_ok_and(|m| m.is_dir()) {
            return Err(McpError::invalid_params(
                format!("'{path}' is a directory. Use list_directory to see its contents."),
                None,
            ));
        }

        fs::read_to_string(note_path)
            .await
            .map_err(|e| io_error(e, &path))
    }

    #[tool(
        description = "Create a note, fully replace it (mode=overwrite), or add text to its start/end (mode=prepend/append). Missing parent directories are created. Prepend/append require an existing note and merge `frontmatter` into its existing properties. To change a small part of a note, prefer patch_note; to change only properties, use update_frontmatter."
    )]
    async fn write_note(
        &self,
        Parameters(WriteNoteParams {
            path,
            content,
            frontmatter,
            mode,
        }): Parameters<WriteNoteParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path).await?;
        let final_content = match mode {
            WriteMode::Overwrite => render_note(frontmatter, &content)?,
            WriteMode::Append | WriteMode::Prepend => {
                let old_content = fs::read_to_string(&note_path).await.map_err(|e| {
                    if e.kind() == ErrorKind::NotFound {
                        McpError::resource_not_found(
                            format!(
                                "Note not found: {path}. Append/prepend need an existing note; use mode=overwrite to create it."
                            ),
                            None,
                        )
                    } else {
                        io_error(e, &path)
                    }
                })?;
                let (fm, old_body) = split_frontmatter(&old_content);
                let body = match mode {
                    WriteMode::Append => format!("{}\n{}", old_body.trim_end(), content),
                    _ => format!("{}\n{}", content.trim_end(), old_body),
                };
                // Without new properties the original frontmatter is kept byte for byte.
                let head = match frontmatter {
                    None => old_content[..old_content.len() - old_body.len()].to_string(),
                    Some(new) => {
                        let matter = parse_matter(fm.unwrap_or_default())
                            .map_err(|e| yaml_error(e, &path))?;
                        render_note(merge_front(matter, Some(new)), "")?
                    }
                };
                format!("{head}{body}")
            }
        };

        if let Some(parent) = note_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| io_error(e, &path))?;
        }
        write_atomic(&note_path, final_content)
            .await
            .map_err(|e| io_error(e, &path))?;

        info!(note = %path, %mode, "note written");
        Ok(format!(
            "Successfully wrote note: {} (mode: {})",
            path, mode
        ))
    }

    #[tool(
        description = "Replace an exact string in the body of an existing note without rewriting the whole file. Frontmatter is preserved and not searched. Fails if the string is not found, or is found several times while replace_all is false."
    )]
    async fn patch_note(
        &self,
        Parameters(PatchNoteParams {
            path,
            old_content,
            new_content,
            replace_all,
        }): Parameters<PatchNoteParams>,
    ) -> Result<String, McpError> {
        if old_content.is_empty() {
            return Err(McpError::invalid_params(
                "old_content must not be empty.".to_string(),
                None,
            ));
        }
        let note_path = resolve_path(&self.vault_path, &path).await?;
        let note_content = fs::read_to_string(&note_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let (_, content) = split_frontmatter(&note_content);
        let count = content.matches(&old_content).count();

        if count == 0 {
            return Err(McpError::invalid_params(
                format!(
                    "old_content was not found in the body of '{path}'. Matching is exact (case and whitespace) and frontmatter is not searched."
                ),
                None,
            ));
        }

        if count > 1 && !replace_all {
            return Err(McpError::invalid_params(
                format!(
                    "old_content occurs {count} times in '{path}'. Set replace_all=true to replace all of them, or include more surrounding text so it matches exactly once."
                ),
                None,
            ));
        }

        let head = &note_content[..note_content.len() - content.len()];
        let final_content = format!("{head}{}", content.replace(&old_content, &new_content));
        write_atomic(&note_path, final_content)
            .await
            .map_err(|e| io_error(e, &path))?;

        info!(note = %path, count, "note patched");
        Ok(format!("Successfully replaced {count} occurrence(s)."))
    }

    #[tool(
        description = "List the files and subdirectories directly inside a vault directory (non-recursive). Use an empty path for the vault root."
    )]
    async fn list_directory(
        &self,
        Parameters(ListDirectoryParams { path }): Parameters<ListDirectoryParams>,
    ) -> Result<Json<ListDirectoryResult>, McpError> {
        let dir_path = resolve_path(&self.vault_path, &path).await?;
        if fs::metadata(&dir_path).await.is_ok_and(|m| m.is_file()) {
            return Err(McpError::invalid_params(
                format!("'{path}' is a file, not a directory."),
                None,
            ));
        }

        let mut entries = fs::read_dir(&dir_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let mut result = ListDirectoryResult::default();
        while let Some(entry) = entries.next_entry().await.map_err(|e| io_error(e, &path))? {
            let filename = entry.file_name().to_string_lossy().to_string();
            if is_restricted(&filename) {
                continue;
            }
            // Follows symlinks so links to directories land in `dirs`; a broken
            // entry (dangling symlink, race with deletion) is skipped, not fatal.
            let Ok(metadata) = fs::metadata(entry.path()).await else {
                continue;
            };
            if metadata.is_dir() {
                result.dirs.push(filename);
            } else {
                result.files.push(filename);
            }
        }
        result.dirs.sort();
        result.files.sort();

        Ok(Json(result))
    }

    #[tool(
        description = "Move or rename a file inside the vault. Missing parent directories of the destination are created. Fails if the destination exists unless overwrite=true. Does not update links in other notes."
    )]
    async fn move_file(
        &self,
        Parameters(MoveFileParams {
            path,
            new_path: new_file_path,
            overwrite,
        }): Parameters<MoveFileParams>,
    ) -> Result<String, McpError> {
        let old_path = resolve_path(&self.vault_path, &path).await?;
        let new_path = resolve_path(&self.vault_path, &new_file_path).await?;

        let old_meta = fs::metadata(&old_path)
            .await
            .map_err(|e| io_error(e, &path))?;

        if old_meta.is_dir() {
            return Err(McpError::invalid_params(
                format!("Source '{path}' is a directory."),
                None,
            ));
        }

        if old_path == new_path {
            return Err(McpError::invalid_params(
                format!("Source and destination are the same: '{path}'."),
                None,
            ));
        }

        match fs::metadata(&new_path).await {
            Ok(meta) if meta.is_dir() => {
                return Err(McpError::invalid_params(
                    format!("Destination '{new_file_path}' is a directory."),
                    None,
                ));
            }
            Ok(_) if !overwrite => {
                return Err(McpError::invalid_params(
                    format!(
                        "File '{}' exists in the vault. Use overwrite=true to overwrite it.",
                        new_file_path
                    ),
                    None,
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(e, &new_file_path)),
        }

        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| io_error(e, &new_file_path))?;
        }

        fs::rename(old_path, new_path)
            .await
            .map_err(|e| io_error(e, &path))?;

        info!(from = %path, to = %new_file_path, "file moved");
        Ok(format!(
            "Successfully moved file '{}' to '{}'",
            path, new_file_path
        ))
    }

    #[tool(description = "Permanently delete a note from the vault. This cannot be undone.")]
    async fn delete_note(
        &self,
        Parameters(DeleteNoteParams { path }): Parameters<DeleteNoteParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path).await?;
        fs::remove_file(note_path)
            .await
            .map_err(|e| io_error(e, &path))?;

        info!(note = %path, "note deleted");
        Ok(format!("Successfully deleted note: {path}"))
    }

    #[tool(
        description = "Get only the frontmatter (properties) of a note as a JSON object, or null if it has none. Cheaper than read_note when the body is not needed."
    )]
    async fn get_frontmatter(
        &self,
        Parameters(GetFrontmatterParams { path }): Parameters<GetFrontmatterParams>,
    ) -> Result<Json<GetFrontmatterResult>, McpError> {
        let note_path = resolve_path(&self.vault_path, &path).await?;
        if fs::metadata(&note_path).await.is_ok_and(|m| m.is_dir()) {
            return Err(McpError::invalid_params(
                format!("'{path}' is a directory."),
                None,
            ));
        }

        let content = fs::read_to_string(&note_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let (fm, _) = split_frontmatter(&content);
        let matter = parse_matter(fm.unwrap_or_default()).map_err(|e| yaml_error(e, &path))?;
        Ok(Json(GetFrontmatterResult {
            frontmatter: matter,
        }))
    }

    #[tool(
        description = "Change the frontmatter (properties) of a note without touching its body. By default merges into existing properties (null value deletes a key); with merge=false replaces all properties."
    )]
    async fn update_frontmatter(
        &self,
        Parameters(UpdateFrontmatterParams {
            path,
            frontmatter,
            merge,
        }): Parameters<UpdateFrontmatterParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path).await?;
        if fs::metadata(&note_path).await.is_ok_and(|m| m.is_dir()) {
            return Err(McpError::invalid_params(
                format!("'{path}' is a directory."),
                None,
            ));
        }

        let content = fs::read_to_string(&note_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let (fm, body) = split_frontmatter(&content);
        let matter = parse_matter(fm.unwrap_or_default()).map_err(|e| yaml_error(e, &path))?;
        let new_frontmatter = if merge {
            merge_front(matter, frontmatter)
        } else {
            frontmatter
        };

        let note = render_note(new_frontmatter, body)?;
        write_atomic(&note_path, note)
            .await
            .map_err(|e| io_error(e, &path))?;
        info!(note = %path, merge, "frontmatter updated");
        if merge {
            Ok(format!("Successfully merged frontmatter in '{}'.", path))
        } else {
            Ok(format!(
                "Successfully overwritten frontmatter in '{}'.",
                path
            ))
        }
    }
}

#[tool_handler(
    name = "shard",
    instructions = "Read and edit notes in a local Obsidian vault (a folder of Markdown files).

Paths:
- Every path is relative to the vault root, e.g. `projects/ideas.md`. No leading slash, no `.`, `..` or `:` components.
- Notes are `.md` files: include the extension, otherwise Obsidian will not show the note.
- Hidden entries (names starting with `.`, such as `.obsidian` or `.git`) and `node_modules` are not accessible.
- Use list_directory (empty path = vault root) to discover what exists. It is not recursive.

Note format:
- A note may start with a YAML frontmatter block between two `---` lines (the note's properties); the rest is the Markdown body.
- Frontmatter is a mapping of properties. Tool parameters and results show it as a JSON object.

Choosing a tool:
- read_note: the whole raw text, frontmatter included. get_frontmatter: only the properties, cheaper.
- patch_note: replace an exact string in the body. Read the note first and copy the text exactly. Prefer it over rewriting a note.
- write_note: create a note or replace it (mode=overwrite, replaces frontmatter too), or add text to its end/start (mode=append/prepend, the note must exist).
- update_frontmatter: change properties without touching the body. Merges by default; a null value removes a key.
- move_file renames or moves a note. Links in other notes are NOT updated. delete_note is permanent.

Errors explain what went wrong and how to fix it; correct the call and retry."
)]
impl ServerHandler for Server {}
