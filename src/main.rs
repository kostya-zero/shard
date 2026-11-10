use std::fs;

use anyhow::{Context, ensure};
use clap::Parser;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

use crate::{cli::Cli, server::Server};

mod cli;
mod dto;
mod frontmatter;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let vault_path = fs::canonicalize(&args.path)
        .with_context(|| format!("vault path '{}' is not accessible", args.path.display()))?;
    ensure!(
        vault_path.is_dir(),
        "vault path '{}' is not a directory",
        vault_path.display()
    );
    let transport = (stdin(), stdout());
    let server = Server::new(vault_path).serve(transport).await?;
    server.waiting().await?;

    Ok(())
}
