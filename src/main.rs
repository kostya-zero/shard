use std::{fs, path::Path, process::exit};

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

    let vault_path = Path::new(&args.path).to_path_buf();
    let Ok(vault_path_canon) = fs::canonicalize(&args.path) else {
        eprintln!("failed to canonicalize the path to vault");
        exit(1);
    };
    let transport = (stdin(), stdout());
    let server = Server::new(vault_path_canon).serve(transport).await?;
    server.waiting().await?;

    Ok(())
}
