use std::{fs, io::stderr};

use anyhow::{Context, ensure};
use clap::Parser;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing::info;
use tracing_subscriber::EnvFilter;

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

    // stdout carries the MCP protocol, so logs must go to stderr.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_writer(stderr)
        .with_ansi(false)
        .init();

    info!(vault = %vault_path.display(), "starting shard");

    let transport = (stdin(), stdout());
    let server = Server::new(vault_path).serve(transport).await?;
    server.waiting().await?;

    Ok(())
}
