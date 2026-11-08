use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
pub struct ListDirectoryParams {
    /// The path to directory.
    pub path: String,
}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListDirectoryResult {
    pub files: Vec<String>,
    pub dirs: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveFileParams {
    /// The path to file.
    pub path: String,

    /// The new location for file.
    pub new_path: String,

    /// Overwrite if file by new path exists.
    #[serde(default)]
    pub overwrite: bool,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetFrontmatterParams {
    /// The path to note.
    pub path: String,
}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetFrontmatterResult {
    /// The frontmatter of the file.
    pub frontmatter: Option<Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateFrontmatterParams {
    /// The path to note.
    pub path: String,
    /// Properties to apply. Must be an object or null.
    /// With merge, nested objects are merged recursively and a null value removes the key.
    /// Without merge, replaces the whole frontmatter (null removes it).
    pub frontmatter: Option<Map<String, Value>>,
    /// Merge into the existing frontmatter (default) or replace it.
    #[serde(default = "default_merge")]
    pub merge: bool,
}

fn default_merge() -> bool {
    true
}
