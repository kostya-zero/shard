use std::fmt::Display;

use serde::Deserialize;
use serde_json::Value;

#[derive(Default, Debug, Deserialize, schemars::JsonSchema)]
pub enum WriteMode {
    #[default]
    Overwrite,
    Append,
    Prepend,
}

impl Display for WriteMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteMode::Overwrite => write!(f, "overwrite"),
            WriteMode::Append => write!(f, "append"),
            WriteMode::Prepend => write!(f, "prepend"),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadNoteParams {
    /// The path to note.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteNoteParams {
    /// The path to note.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteNoteParams {
    /// The path to note.
    pub path: String,

    /// Content to insert
    pub content: String,

    /// Optional note properties to add.
    pub frontmatter: Option<Value>,

    /// Which write mode to use.
    pub mode: WriteMode,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PatchNoteParams {
    /// The path to note.
    pub path: String,

    /// The old content to replace.
    pub old_content: String,

    /// The new content.
    pub new_content: String,

    /// If there is more than one occurrence of old content,
    /// replace_all should be set to true.
    pub replace_all: bool,
}
