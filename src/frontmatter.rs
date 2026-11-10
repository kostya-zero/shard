use anyhow::Result;
use serde_json::Value;

pub fn parse_matter(matter: &str) -> Result<Option<Value>> {
    if matter.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_yaml_ng::from_str(matter)?))
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
