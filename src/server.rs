use std::{io::ErrorKind, path::PathBuf};

use gray_matter::{Matter, engine::YAML};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use serde_json::{Value, json};
use tokio::fs;

use crate::dto::{DeleteNoteParams, PatchNoteParams, ReadNoteParams, WriteMode, WriteNoteParams};

fn internal_error(e: impl ToString) -> McpError {
    McpError::internal_error("internal_error", Some(json!({"error": e.to_string()})))
}

fn render_note(frontmatter: Option<Value>, body: &str) -> Result<String, McpError> {
    match frontmatter {
        Some(f) if f != Value::Null => {
            let yaml = serde_yaml_ng::to_string(&f).map_err(internal_error)?;
            Ok(format!("---\n{yaml}---\n{body}"))
        }
        _ => Ok(body.to_string()),
    }
}

fn merge_front(old: Option<Value>, new: Option<Value>) -> Option<Value> {
    match (old, new) {
        (Some(Value::Object(mut a)), Some(Value::Object(b))) => {
            a.extend(b);
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
        let note_path = self.vault_path.join(&path);
        if !note_path.exists() {
            return Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({"path": path})),
            ));
        }

        fs::read_to_string(note_path).await.map_err(|e| {
            McpError::internal_error("internal_error", Some(json!({ "error": &e.to_string()})))
        })
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
        let note_path = self.vault_path.join(&path);
        let final_content = match mode {
            WriteMode::Overwrite => render_note(frontmatter, &content)?,
            WriteMode::Append | WriteMode::Prepend => {
                let old_content = fs::read_to_string(&note_path).await.map_err(|e| {
                    if e.kind() == ErrorKind::NotFound {
                        McpError::resource_not_found(
                            "resource_not_found",
                            Some(json!({"path": path})),
                        )
                    } else {
                        internal_error(e)
                    }
                })?;
                let old = Matter::<YAML>::new()
                    .parse::<Value>(&old_content)
                    .map_err(internal_error)?;
                let body = match mode {
                    WriteMode::Append => format!("{}\n{}", old.content.trim_end(), content),
                    _ => format!("{}\n{}", content.trim_end(), old.content),
                };
                render_note(merge_front(old.data, frontmatter), &body)?
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
        let note_path = self.vault_path.join(&path);
        let note_content = fs::read_to_string(&note_path).await.map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                McpError::resource_not_found("resource_not_found", Some(json!({"path": path})))
            } else {
                internal_error(e)
            }
        })?;
        let p = Matter::<YAML>::new()
            .parse::<Value>(&note_content)
            .map_err(internal_error)?;
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

        let final_content = render_note(p.data, &content.replace(&old_content, &new_content))?;
        fs::write(note_path, final_content)
            .await
            .map_err(internal_error)?;

        Ok(format!("Successfully replaced {count} occurrence(s)."))
    }

    #[tool(description = "Delete a note from the vault.")]
    async fn delete_note(
        &self,
        Parameters(DeleteNoteParams { path }): Parameters<DeleteNoteParams>,
    ) -> Result<String, McpError> {
        fs::remove_file(self.vault_path.join(&path))
            .await
            .map_err(|e| {
                if e.kind() == ErrorKind::NotFound {
                    McpError::resource_not_found("resource_not_found", Some(json!({"path": path})))
                } else {
                    internal_error(e)
                }
            })?;

        Ok(format!("Successfully deleted note: {path}"))
    }
}

#[tool_handler(
    name = "shard",
    instructions = "An MCP server for Obsidian",
    version = "0.1.0"
)]
impl ServerHandler for Server {}
