use std::path::PathBuf;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use serde_json::json;
use tokio::fs;

use crate::dto::ReadNoteParams;

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
}

#[tool_handler(
    name = "shard",
    instructions = "An MCP server for Obsidian",
    version = "0.1.0"
)]
impl ServerHandler for Server {}
