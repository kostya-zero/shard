use std::path::PathBuf;

use rmcp::{
    ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};

use crate::dto::EchoParams;

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

    #[tool(description = "Returns text back.")]
    async fn echo(&self, Parameters(EchoParams { text }): Parameters<EchoParams>) -> String {
        text
    }
}

#[tool_handler]
impl ServerHandler for Server {}
