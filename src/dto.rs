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
