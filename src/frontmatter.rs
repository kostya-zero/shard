use anyhow::{Result, bail};
use serde_json::{Map, Value};

pub fn parse_matter(matter: &str) -> Result<Option<Map<String, Value>>> {
    if matter.trim().is_empty() {
        return Ok(None);
    }
    match serde_yaml_ng::from_str(matter)? {
        Value::Object(map) => Ok(Some(map)),
        Value::Null => Ok(None),
        _ => bail!("frontmatter must be a mapping of properties"),
    }
}

pub fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let Some(rest) = strip_delimiter(text) else {
        return (None, text);
    };
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let yaml = &rest[..pos];
            return (Some(yaml), &rest[pos + line.len()..]);
        }
        pos += line.len();
    }
    (None, text)
}

fn strip_delimiter(text: &str) -> Option<&str> {
    let line = text.split_inclusive('\n').next()?;
    (line.trim_end_matches(['\n', '\r']) == "---").then(|| &text[line.len()..])
}
