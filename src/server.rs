use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use gray_matter::{Matter, engine::YAML};
use rmcp::{
    ErrorData as McpError, Json, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use serde_json::{Map, Value};
use tokio::fs;

use crate::{
    dto::{
        DeleteNoteParams, GetFrontmatterParams, GetFrontmatterResult, ListDirectoryParams,
        ListDirectoryResult, MoveFileParams, PatchNoteParams, ReadNoteParams,
        UpdateFrontmatterParams, WriteMode, WriteNoteParams,
    },
    frontmatter::parse_matter,
};

static EXCLUDED_DIRS: [&str; 2] = [".obsidian", ".git"];

fn resolve_path(vault_path: &Path, note_path: &str) -> Result<PathBuf, McpError> {
    let path = vault_path.join(note_path);
    let note_pathbuf = Path::new(note_path);
    if !note_pathbuf
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(McpError::invalid_params("bad path provided", None));
    }

    Ok(path)
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
        internal_error(e)
    }
}

fn render_note(frontmatter: Option<Value>, body: &str) -> Result<String, McpError> {
    match frontmatter {
        Some(f) if f != Value::Null && f != Value::Object(Map::new()) => {
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

fn merge_front(old: Option<Value>, new: Option<Value>) -> Option<Value> {
    match (old, new) {
        (Some(Value::Object(mut a)), Some(Value::Object(b))) => {
            merge_objects(&mut a, b);
            Some(Value::Object(a))
        }
        (_, Some(n)) => Some(n),
        (o, None) => o,
    }
}

#[derive(Clone)]
pub struct Server {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    vault_path: PathBuf,
}

#[tool_router]
impl Server {
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            vault_path,
            tool_router: Self::tool_router(),
        }
    }

    // I think it's a temporary solution, because I want to implement
    // separation from frontmatter and note content.
    #[tool(description = "Read note content and properties.")]
    async fn read_note(
        &self,
        Parameters(ReadNoteParams { path }): Parameters<ReadNoteParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path)?;
        fs::read_to_string(note_path)
            .await
            .map_err(|e| io_error(e, &path))
    }

    #[tool(description = "Write a note to the vault.")]
    async fn write_note(
        &self,
        Parameters(WriteNoteParams {
            path,
            content,
            frontmatter,
            mode,
        }): Parameters<WriteNoteParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path)?;
        let final_content = match mode {
            WriteMode::Overwrite => render_note(frontmatter, &content)?,
            WriteMode::Append | WriteMode::Prepend => {
                let old_content = fs::read_to_string(&note_path)
                    .await
                    .map_err(|e| io_error(e, &path))?;
                let old = Matter::<YAML>::new()
                    .parse::<Value>(&old_content)
                    .map_err(|e| yaml_error(e, &path))?;
                let body = match mode {
                    WriteMode::Append => format!("{}\n{}", old.content.trim_end(), content),
                    _ => format!("{}\n{}", content.trim_end(), old.content),
                };
                let matter = parse_matter(old.matter).map_err(|e| yaml_error(e, &path))?;
                render_note(merge_front(matter, frontmatter), &body)?
            }
        };

        if let Some(parent) = note_path.parent() {
            fs::create_dir_all(parent).await.map_err(internal_error)?;
        }
        fs::write(&note_path, final_content)
            .await
            .map_err(internal_error)?;

        Ok(format!(
            "Successfully wrote note: {} (mode: {})",
            path, mode
        ))
    }

    #[tool(
        description = "Replace the exact string inside an existing note without rewriting full file."
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
        let note_path = resolve_path(&self.vault_path, &path)?;
        let note_content = fs::read_to_string(&note_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let p = Matter::<YAML>::new()
            .parse::<Value>(&note_content)
            .map_err(|e| yaml_error(e, &path))?;
        let matter = parse_matter(p.matter).map_err(|e| yaml_error(e, &path))?;
        let content = p.content;
        let count = content.matches(&old_content).count();

        if count == 0 {
            return Err(McpError::invalid_params(
                "No occurrences of the string found.",
                None,
            ));
        }

        if count > 1 && !replace_all {
            return Err(McpError::invalid_params(
                format!(
                    "Found {} occurrences of the string. Use replace_all=true to replace all occurrences, or provide a more specific string to match exactly one occurrence.",
                    count
                ),
                None,
            ));
        }

        let final_content = render_note(matter, &content.replace(&old_content, &new_content))?;
        fs::write(note_path, final_content)
            .await
            .map_err(internal_error)?;

        Ok(format!("Successfully replaced {count} occurrence(s)."))
    }

    #[tool(description = "Lists entries in the directory.")]
    async fn list_directory(
        &self,
        Parameters(ListDirectoryParams { path }): Parameters<ListDirectoryParams>,
    ) -> Result<Json<ListDirectoryResult>, McpError> {
        let dir_path = resolve_path(&self.vault_path, &path)?;
        if dir_path.is_file() {
            return Err(McpError::invalid_params(
                "Path should point at a directory.",
                None,
            ));
        }

        let mut entries = fs::read_dir(&dir_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let mut result = ListDirectoryResult::default();
        while let Some(entry) = entries.next_entry().await.map_err(internal_error)? {
            let metadata = entry.metadata().await.map_err(internal_error)?;
            let filename = entry.file_name().to_string_lossy().to_string();
            if metadata.is_dir() && !EXCLUDED_DIRS.contains(&filename.as_str()) {
                result.dirs.push(filename);
            } else {
                result.files.push(filename);
            }
        }

        Ok(Json(result))
    }

    #[tool(description = "Move file to a new location.")]
    async fn move_file(
        &self,
        Parameters(MoveFileParams {
            path,
            new_path: new_file_path,
            overwrite,
        }): Parameters<MoveFileParams>,
    ) -> Result<String, McpError> {
        let old_path = resolve_path(&self.vault_path, &path)?;
        let new_path = resolve_path(&self.vault_path, &new_file_path)?;

        fs::metadata(&old_path)
            .await
            .map_err(|e| io_error(e, &path))?;

        if old_path == new_path {
            return Err(McpError::invalid_params(
                "Source and destination are the same.",
                None,
            ));
        }

        if new_path.is_dir() {
            return Err(McpError::invalid_params(
                format!("Destination '{new_file_path}' is a directory."),
                None,
            ));
        }

        if new_path.exists() && !overwrite {
            return Err(McpError::invalid_params(
                format!(
                    "File '{}' exists in the vault. Use overwrite=true to overwrite it.",
                    new_file_path
                ),
                None,
            ));
        }

        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).await.map_err(internal_error)?;
        }

        fs::rename(old_path, new_path)
            .await
            .map_err(internal_error)?;

        Ok(format!(
            "Successfully moved file '{}' to '{}'",
            path, new_file_path
        ))
    }

    #[tool(description = "Delete a note from the vault.")]
    async fn delete_note(
        &self,
        Parameters(DeleteNoteParams { path }): Parameters<DeleteNoteParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path)?;
        fs::remove_file(note_path)
            .await
            .map_err(|e| io_error(e, &path))?;

        Ok(format!("Successfully deleted note: {path}"))
    }

    #[tool(description = "Extracts only frontmatter from a note without reading the full content.")]
    async fn get_frontmatter(
        &self,
        Parameters(GetFrontmatterParams { path }): Parameters<GetFrontmatterParams>,
    ) -> Result<Json<GetFrontmatterResult>, McpError> {
        let note_path = resolve_path(&self.vault_path, &path)?;
        if note_path.is_dir() {
            return Err(McpError::invalid_params(
                format!("'{path}' is a directory."),
                None,
            ));
        }

        let content = fs::read_to_string(&note_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let p = Matter::<YAML>::new()
            .parse::<Value>(&content)
            .map_err(|e| {
                McpError::invalid_params(format!("Invalid frontmatter in '{path}': {e}"), None)
            })?;
        let matter = parse_matter(p.matter).map_err(|e| yaml_error(e, &path))?;
        Ok(Json(GetFrontmatterResult {
            frontmatter: matter,
        }))
    }

    #[tool(description = "Update the frontmatter of a note without touching its content.")]
    async fn update_frontmatter(
        &self,
        Parameters(UpdateFrontmatterParams {
            path,
            frontmatter,
            merge,
        }): Parameters<UpdateFrontmatterParams>,
    ) -> Result<String, McpError> {
        let note_path = resolve_path(&self.vault_path, &path)?;
        if note_path.is_dir() {
            return Err(McpError::invalid_params(
                format!("'{path}' is a directory."),
                None,
            ));
        }

        let content = fs::read_to_string(&note_path)
            .await
            .map_err(|e| io_error(e, &path))?;
        let p = Matter::<YAML>::new()
            .parse::<Value>(&content)
            .map_err(|e| {
                McpError::invalid_params(format!("Invalid frontmatter in '{path}': {e}"), None)
            })?;
        let matter = parse_matter(p.matter).map_err(|e| yaml_error(e, &path))?;

        let frontmatter = frontmatter.map(Value::Object);
        let new_frontmatter = if merge {
            merge_front(matter, frontmatter)
        } else {
            frontmatter
        };

        // gray_matter trims the body, so restore the trailing newline of the original.
        let mut body = p.content;
        if content.ends_with('\n') && !body.ends_with('\n') {
            body.push('\n');
        }
        let note = render_note(new_frontmatter, &body)?;
        fs::write(&note_path, note)
            .await
            .map_err(|e| io_error(e, &path))?;
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
    instructions = "An MCP server for Obsidian",
    version = "0.1.0"
)]
impl ServerHandler for Server {}
