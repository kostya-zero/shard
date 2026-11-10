use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// The path to your Obsidian vault.
    pub path: PathBuf,
}
