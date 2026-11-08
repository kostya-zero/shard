use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Default, Debug, Deserialize, schemars::JsonSchema)]
pub enum WriteMode {
    /// Replace the whole note (frontmatter included). Creates the note and missing
    /// parent directories if needed.
    #[default]
    Overwrite,
    /// Add content to the end of the note body. The note must already exist.
    Append,
    /// Add content to the start of the note body. The note must already exist.
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
    /// Note path relative to the vault root, e.g. `projects/ideas.md`.
    /// Absolute paths and `..` are rejected.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteNoteParams {
    /// Path of the note to delete, relative to the vault root.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteNoteParams {
    /// Note path relative to the vault root, e.g. `daily/2025-01-01.md`.
    pub path: String,

    /// Markdown body of the note, without the `---` frontmatter block
    /// (use `frontmatter` for properties).
    pub content: String,

    /// Properties as a JSON object, written as YAML frontmatter.
    /// With `overwrite` they become the note's entire frontmatter.
    /// With `append`/`prepend` they are merged into the existing frontmatter:
    /// nested objects merge recursively and a null value removes the key.
    /// Omit to leave existing frontmatter untouched (`append`/`prepend`)
    /// or to write no frontmatter (`overwrite`).
    pub frontmatter: Option<Value>,

    /// How to write: `overwrite` replaces the whole note, `append` / `prepend`
    /// add `content` to the end / start of the existing body.
    pub mode: WriteMode,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDirectoryParams {
    /// Directory path relative to the vault root. Use an empty string for the root.
    pub path: String,
}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListDirectoryResult {
    /// File names directly inside the directory (names only, not full paths).
    pub files: Vec<String>,
    /// Subdirectory names directly inside the directory (names only, not full paths).
    pub dirs: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveFileParams {
    /// Current path of the file, relative to the vault root.
    pub path: String,

    /// Destination path of the file (including its file name), relative to the
    /// vault root. Missing parent directories are created. Also used to rename.
    pub new_path: String,

    /// Replace the file at `new_path` if it already exists. Default: false (error).
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PatchNoteParams {
    /// Note path relative to the vault root.
    pub path: String,

    /// Exact text to find in the note body (case-sensitive, whitespace matters).
    /// Frontmatter is not searched. Must match exactly one place unless `replace_all` is true.
    pub old_content: String,

    /// Text to put in place of `old_content`. Use an empty string to delete it.
    pub new_content: String,

    /// Set to true to replace every occurrence of `old_content`.
    /// If false and the text occurs more than once, the call fails.
    pub replace_all: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetFrontmatterParams {
    /// Note path relative to the vault root.
    pub path: String,
}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetFrontmatterResult {
    /// Note properties as a JSON object, or null if the note has no frontmatter.
    pub frontmatter: Option<Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateFrontmatterParams {
    /// Note path relative to the vault root.
    pub path: String,
    /// Properties as a JSON object, or null.
    /// With `merge`: listed keys are set, unlisted keys are kept, nested objects
    /// merge recursively, and a null value deletes that key.
    /// Without `merge`: replaces the whole frontmatter; null removes it entirely.
    pub frontmatter: Option<Map<String, Value>>,
    /// true (default): merge into the existing frontmatter.
    /// false: replace the existing frontmatter completely.
    #[serde(default = "default_merge")]
    pub merge: bool,
}

fn default_merge() -> bool {
    true
}
